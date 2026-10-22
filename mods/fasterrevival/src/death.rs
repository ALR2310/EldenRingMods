//! Player death: calls the game's own "kill character" function as soon as the
//! death animation starts, instead of when its TAE event fires near the end,
//! so "YOU DIED" appears sooner. The function is idempotent, so the event's
//! later call is a no-op. The pause after "YOU DIED" is cut by [crate::fade].
//! Always on. How vanilla dies, the function, the animation list and the test
//! results: `docs/death_flow.md`.

use std::time::Instant;

use eldenring::cs::{CSChrTimeActModuleAnim, CSTaskGroupIndex, ChrIns, WorldChrMan};
use fromsoftware_shared::FromStatic;

use common::anim::current_anim;
use common::{config, logger, memscan};

use crate::fade::{self, FadeState};

/// `sub_1403EDA70` (2.7.1.0) / `sub_1403ED840` (2.6.2.0), unique in both. The
/// sibling "Kill Character (No Item Drop)" shares the whole prologue and
/// differs by one missing call, hence the long tail.
const KILL_CHR_PATTERN: &str = "40 53 48 83 EC 40 48 C7 44 24 20 FE FF FF FF 0F 29 74 24 30 48 8B D9 48 8B 41 58 \
     48 8B 90 C8 00 00 00 F6 42 24 01 0F 85 ?? ?? ?? ?? E8 ?? ?? ?? ?? 8B 43 68 83 F8 03 0F 84 ?? ?? ?? ?? \
     83 F8 0A 0F 84 ?? ?? ?? ?? 48 8B CB E8 ?? ?? ?? ?? 48 8B CB E8 ?? ?? ?? ?? 0F 28 F0";

type KillChrFn = unsafe extern "C" fn(*mut ChrIns);

/// Offset of the "already killed" byte in `ChrCtrlModifier`; bit 0 is the flag.
const MODIFIER_KILLED_OFFSET: usize = 0x24;

/// Whether `anim_id` (`category * 1_000_000 + id`) is a player animation whose
/// TAE carries the "Kill Character" event: regular deaths `a000_017xxx`,
/// deaths inside a grab `a000_070xxx`, `a000_117xxx`, `a000_075003`,
/// `a000_000150`. Other deaths (a fall, anim `4100`) are killed on the frame
/// HP hits 0 already. Only the animation playing now counts: the queue can
/// still show attack animations for a frame or two after HP 0.
fn has_kill_event(anim_id: i32) -> bool {
    matches!(anim_id, 150 | 17_000..=17_999 | 70_000..=70_999 | 75_003 | 117_000..=117_999)
}

/// While dead, `DeathProbe` logs at most this often (every frame buries the log).
const PROBE_INTERVAL_MS: u128 = 100;

#[derive(Default)]
struct DeathState {
    /// `Some` from the first frame HP is seen at 0 until respawn.
    since: Option<Instant>,
    kill_sent: bool,
    killed_logged: bool,
    last_log: Option<Instant>,
    last_anim_id: i32,
    last_killed: bool,
    fade: FadeState,
}

impl DeathState {
    /// Starts tracking a new death at `now`.
    fn begin(&mut self, now: Instant) {
        self.since = Some(now);
        self.kill_sent = false;
        self.killed_logged = false;
        self.last_log = None;
        self.last_anim_id = -1;
    }
}

pub fn run() {
    let kill_chr: Option<KillChrFn> = match memscan::find_pattern_in_module(KILL_CHR_PATTERN) {
        Some(addr) => {
            logger::log("KillChr: found");
            Some(unsafe { std::mem::transmute::<*mut u8, KillChrFn>(addr) })
        }
        None => {
            logger::error("KillChr: not found, death speed-up disabled");
            None
        }
    };

    let cs_task = common::task::wait_for_cs_task();
    let mut state = DeathState::default();
    common::task::run_recurring_safe(
        cs_task,
        "Death",
        CSTaskGroupIndex::FrameBegin,
        move |_data: &eldenring::fd4::FD4TaskData| tick(&mut state, kill_chr),
    );
}

fn tick(state: &mut DeathState, kill_chr: Option<KillChrFn>) {
    let Ok(world_chr_man) = (unsafe { WorldChrMan::instance() }) else { return };
    let Some(main_player) = world_chr_man.main_player.as_ref() else { return };
    let chr_ins = &main_player.chr_ins;

    fade::apply(&mut state.fade);

    // Death is `hp <= 0`, not `death_flag`: a FrameBegin task never sees the
    // flag, which the same frame's character update consumes.
    if chr_ins.modules.data.hp > 0 {
        if let Some(since) = state.since.take() {
            logger::log(&format!("Death: respawned {} ms after HP 0", since.elapsed().as_millis()));
        }
        return;
    }

    let anim = current_anim(chr_ins);
    let modifier = &*chr_ins.chr_ctrl.modifier as *const _ as *const u8;
    let killed = unsafe { *modifier.add(MODIFIER_KILLED_OFFSET) } & 1 != 0;

    let now = Instant::now();
    let since = match state.since {
        Some(since) => since,
        None => {
            state.begin(now);
            logger::log(&format!(
                "Death: died anim={} death_flag={} killed={killed}",
                anim.anim_id,
                chr_ins.chr_flags1c5.death_flag()
            ));
            now
        }
    };
    let elapsed = now.duration_since(since).as_millis();

    if !killed
        && !state.kill_sent
        && has_kill_event(anim.anim_id)
        && let Some(kill_chr) = kill_chr
    {
        state.kill_sent = true;
        unsafe { kill_chr(chr_ins as *const ChrIns as *mut ChrIns) };
        logger::log(&format!("Death: killed early +{elapsed} ms anim={} t={:.3}", anim.anim_id, anim.play_time));
    }

    if killed && !state.killed_logged {
        state.killed_logged = true;
        logger::log(&format!("Death: registered +{elapsed} ms anim={} t={:.3}", anim.anim_id, anim.play_time));
    }

    if config::get_bool("DeathProbe", false) {
        probe(state, chr_ins, anim, killed, now, elapsed);
    }
}

/// `DeathProbe`: the animation trace between HP 0 and respawn.
fn probe(state: &mut DeathState, chr_ins: &ChrIns, anim: &CSChrTimeActModuleAnim, killed: bool, now: Instant, elapsed: u128) {
    let changed = anim.anim_id != state.last_anim_id || killed != state.last_killed;
    let due = state.last_log.is_none_or(|t| now.duration_since(t).as_millis() >= PROBE_INTERVAL_MS);
    if !changed && !due {
        return;
    }
    logger::log(&format!(
        "DeathProbe: +{elapsed} ms anim={} t={:.3}/{:.3} death_flag={} killed={killed}{}",
        anim.anim_id,
        anim.play_time,
        anim.anim_length,
        chr_ins.chr_flags1c5.death_flag(),
        if changed { " *" } else { "" },
    ));
    state.last_log = Some(now);
    state.last_anim_id = anim.anim_id;
    state.last_killed = killed;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_animations_with_the_kill_event_are_sped_up() {
        for id in [150, 17_000, 17_002, 17_999, 70_000, 70_500, 75_003, 117_000, 117_500] {
            assert!(has_kill_event(id), "{id}");
        }
        // A fall (4100), idle, neighbouring ranges, other 75xxx.
        for id in [0, 4_100, 149, 151, 16_999, 18_000, 69_999, 71_000, 75_002, 75_004, 116_999, 118_000, -1] {
            assert!(!has_kill_event(id), "{id}");
        }
    }
}
