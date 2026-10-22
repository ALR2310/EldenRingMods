//! Player-state triggers for `Regen.PerTick.Trigger` (out of / in combat,
//! idle, during a gesture) and the L2 "last attack was a Skill" latch behind
//! `Regen.PerHit.ExcludeAow`. The gesture/idle logic is a state machine driven
//! by an injected clock, so it is unit tested. Why gestures need a delayed
//! confirmation, and the attempts that failed: `docs/gesture_trigger.md`.

use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Instant;

use common::anim::{current_anim_id, is_skill_tae};
use common::config;
use common::logger;
use common::player::ActionSnapshot;
use eldenring::cs::WorldChrMan;
use fromsoftware_shared::FromStatic;

/// How long after the last hit landed or taken the player counts as "in
/// combat". The game exposes no such flag, so this is a grace period.
const COMBAT_TIMEOUT_MS: u64 = 15_000;
/// How long the player must do nothing before `Trigger=3` (idle) applies.
const IDLE_GRACE_MS: u64 = 5_000;
/// Floor before looking at a pending gesture, to skip a same-frame read.
const GESTURE_CONFIRM_MIN_WAIT_MS: u64 = 200;
/// How long the animation id must hold the same value before it is trusted:
/// transitional animations change every ~0.5-1s, a real sit holds for seconds.
const ANIM_STABLE_MS: u64 = 1_000;
/// Give up on a pending gesture that never settles.
const GESTURE_CONFIRM_TIMEOUT_MS: u64 = 6_000;

/// `Regen.PerTick.GestureId` default: the sitting-family gestures. These are
/// half the ids of The Grand Archives' CE table (`requested_gesture` reads
/// back `id / 2`), and "Fetal Position" is left out on purpose. Editable in
/// the ini so a new gesture needs no new DLL.
const DEFAULT_GESTURE_IDS: &str = "80,90,91,92,93,94,95,97,100,101";

/// Milliseconds since the first call: the clock every function here uses.
pub fn now_ms() -> u64 {
    static START: OnceLock<Instant> = OnceLock::new();
    START.get_or_init(Instant::now).elapsed().as_millis() as u64
}

// Written by the hit hook and the tick on different threads, hence atomics.
static LAST_COMBAT_ACTIVITY_MS: AtomicU64 = AtomicU64::new(0);

/// Called whenever the player lands or takes a confirmed hit.
pub fn mark_combat_activity() {
    LAST_COMBAT_ACTIVITY_MS.store(now_ms().max(1), Ordering::Relaxed);
}

/// Whether a hit was landed or taken within `COMBAT_TIMEOUT_MS`.
pub fn is_in_combat() -> bool {
    let last = LAST_COMBAT_ACTIVITY_MS.load(Ordering::Relaxed);
    last != 0 && now_ms().saturating_sub(last) < COMBAT_TIMEOUT_MS
}

/// Whether the last attack-starting button was L2 (Skill), as opposed to
/// R1/R2/L1. Latched on the press, not the held state: a multi-hit Ash of War
/// lands hits long after L2 is released.
static LAST_ATTACK_WAS_SKILL: AtomicBool = AtomicBool::new(false);

pub fn is_last_attack_skill() -> bool {
    LAST_ATTACK_WAS_SKILL.load(Ordering::Relaxed)
}

/// The main player's playing animation id, `None` if not resolved yet.
pub fn main_player_anim_id() -> Option<i32> {
    let world_chr_man = unsafe { WorldChrMan::instance() }.ok()?;
    Some(current_anim_id(&world_chr_man.main_player.as_ref()?.chr_ins))
}

/// Whether the player's current animation is an Ash of War one (TAE prefix
/// 600..1000). Catches stance Ashes whose follow-up R1/R2 fools the latch.
pub fn is_skill_anim() -> bool {
    main_player_anim_id().is_some_and(|id| is_skill_tae(id / 1_000_000))
}

/// `Regen.PerTick.GestureId` as a list of ids (bad entries are skipped).
pub fn gesture_trigger_ids() -> Vec<i32> {
    parse_gesture_ids(&config::get_string("Regen.PerTick.GestureId", DEFAULT_GESTURE_IDS))
}

fn parse_gesture_ids(text: &str) -> Vec<i32> {
    text.split(',').filter_map(|id| id.trim().parse().ok()).collect()
}

/// A gesture press waiting to be confirmed by the animation settling.
#[derive(Clone, Copy)]
struct PendingGesture {
    since: u64,
    /// The animation id playing at the press.
    press_anim: i32,
}

/// Idle and gesture state, updated once per frame by the tick.
#[derive(Default)]
pub struct Triggers {
    /// When the player stopped doing anything; `None` while busy.
    idle_since: Option<u64>,
    gesture_active: bool,
    pending: Option<PendingGesture>,
    /// The playing animation id and when it last changed.
    seen_anim: Option<(i32, u64)>,
}

impl Triggers {
    /// Feeds one frame: `now` in ms, this frame's input, the playing
    /// animation id, and whether a gesture id is one `GestureId` lists.
    pub fn update(&mut self, now: u64, input: &ActionSnapshot, anim: Option<i32>, is_trigger_gesture: impl Fn(i32) -> bool) {
        if let Some(id) = anim
            && self.seen_anim.map(|(seen, _)| seen) != Some(id)
        {
            self.seen_anim = Some((id, now));
        }

        if input.r1 || input.r2 || input.l1 {
            LAST_ATTACK_WAS_SKILL.store(false, Ordering::Relaxed);
        } else if input.l2 {
            LAST_ATTACK_WAS_SKILL.store(true, Ordering::Relaxed);
        }

        if input.new_gesture {
            let gesture = input.requested_gesture;
            if self.gesture_active {
                // A second request while sitting always stands up first, and
                // fires the same signal as starting one: context tells them
                // apart. Standing up needs no confirmation.
                self.gesture_active = false;
                self.pending = None;
                logger::log(&format!("Gesture: {gesture} -> cancel"));
            } else if self.pending.is_some() {
                // Not restarted: by now the animation may already be partway
                // into the sit, which would make the baseline look "unchanged".
                logger::log(&format!("Gesture: {gesture} -> ignored (pending)"));
            } else if is_trigger_gesture(gesture) {
                let press_anim = anim.unwrap_or(0);
                self.pending = Some(PendingGesture { since: now, press_anim });
                logger::log(&format!("Gesture: {gesture} anim={press_anim} -> pending"));
            }
        } else if input.busy {
            self.gesture_active = false;
        }
        self.confirm_pending(now, input.busy);

        if input.busy {
            self.idle_since = None;
        } else if self.idle_since.is_none() {
            self.idle_since = Some(now);
        }
    }

    /// Resolves a pending gesture on a later frame: confirmed once the
    /// animation id is non-zero, differs from the press-time one and held
    /// steady for `ANIM_STABLE_MS`; dropped on any busy input or on timeout.
    fn confirm_pending(&mut self, now: u64, busy: bool) {
        let Some(pending) = self.pending else {
            return;
        };
        if busy {
            self.pending = None;
            return;
        }
        let elapsed = now.saturating_sub(pending.since);
        if elapsed < GESTURE_CONFIRM_MIN_WAIT_MS {
            return;
        }
        let (anim, anim_since) = self.seen_anim.unwrap_or((0, 0));
        let stable_for = now.saturating_sub(anim_since);
        if anim != 0 && anim != pending.press_anim && stable_for >= ANIM_STABLE_MS {
            self.pending = None;
            self.gesture_active = true;
            logger::log(&format!("Gesture: confirmed anim={anim} stable={stable_for}ms"));
        } else if elapsed >= GESTURE_CONFIRM_TIMEOUT_MS {
            self.pending = None;
            self.gesture_active = false;
            logger::log(&format!("Gesture: timed out anim={anim} stable={stable_for}ms"));
        }
    }

    /// No movement and no held action for `IDLE_GRACE_MS`.
    pub fn is_idle(&self, now: u64) -> bool {
        self.idle_since.is_some_and(|since| now.saturating_sub(since) >= IDLE_GRACE_MS)
    }

    pub fn is_gesture_active(&self) -> bool {
        self.gesture_active
    }

    /// Whether `Regen.PerTick.Trigger` lets this tick heal: 0 always, 1 out of
    /// combat, 2 in combat, 3 idle, 4 during a configured gesture.
    pub fn condition_met(&self, trigger: i32, now: u64) -> bool {
        match trigger {
            1 => !is_in_combat(),
            2 => is_in_combat(),
            3 => self.is_idle(now),
            4 => self.is_gesture_active(),
            _ => true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(new_gesture: bool, busy: bool) -> ActionSnapshot {
        ActionSnapshot { r1: false, r2: false, l1: false, l2: false, new_gesture, requested_gesture: 80, busy }
    }

    fn sitting_gestures(id: i32) -> bool {
        id == 80
    }

    /// Runs frames every 100ms from `from` to `to` (exclusive) with the given
    /// animation id, no input.
    fn run(t: &mut Triggers, from: u64, to: u64, anim: i32) {
        for now in (from..to).step_by(100) {
            t.update(now, &input(false, false), Some(anim), sitting_gestures);
        }
    }

    fn press(t: &mut Triggers, now: u64, anim: i32) {
        t.update(now, &input(true, false), Some(anim), sitting_gestures);
    }

    #[test]
    fn a_real_sit_is_confirmed_once_the_animation_holds() {
        let mut t = Triggers::default();
        run(&mut t, 0, 1_000, 0); // standing idle
        press(&mut t, 1_000, 0);
        run(&mut t, 1_100, 1_600, 200); // sitting down: id changes
        assert!(!t.is_gesture_active(), "not yet stable");
        run(&mut t, 1_600, 2_800, 200);
        assert!(t.is_gesture_active());
    }

    #[test]
    fn a_gesture_blocked_mid_cast_is_never_confirmed() {
        let mut t = Triggers::default();
        run(&mut t, 0, 1_000, 50); // casting
        press(&mut t, 1_000, 50); // press is blocked: the cast keeps playing
        run(&mut t, 1_100, 3_000, 50);
        run(&mut t, 3_000, 8_000, 0); // cast ends, falls back to idle (id 0)
        assert!(!t.is_gesture_active());
    }

    #[test]
    fn a_pending_gesture_times_out() {
        let mut t = Triggers::default();
        press(&mut t, 0, 10);
        // The animation keeps changing, never holding for a second.
        for (i, now) in (100..7_000).step_by(100).enumerate() {
            t.update(now, &input(false, false), Some(300 + (i % 2) as i32), sitting_gestures);
        }
        assert!(!t.is_gesture_active());
        assert!(t.pending.is_none(), "the pending confirmation was dropped");
    }

    #[test]
    fn a_repeat_press_while_pending_keeps_the_original_baseline() {
        let mut t = Triggers::default();
        press(&mut t, 1_000, 0);
        run(&mut t, 1_100, 1_300, 150); // already partway into the sit
        press(&mut t, 1_300, 150); // impatient second press
        run(&mut t, 1_400, 3_000, 200);
        assert!(t.is_gesture_active(), "confirmed against the baseline of the first press");
    }

    #[test]
    fn pressing_again_while_sitting_stands_up_at_once() {
        let mut t = Triggers::default();
        press(&mut t, 0, 0);
        run(&mut t, 100, 2_500, 200);
        assert!(t.is_gesture_active());
        press(&mut t, 2_500, 200);
        assert!(!t.is_gesture_active());
        assert!(t.pending.is_none());
    }

    #[test]
    fn any_busy_input_cancels_the_gesture_and_a_pending_one() {
        let mut t = Triggers::default();
        press(&mut t, 0, 0);
        t.update(100, &input(false, true), Some(0), sitting_gestures);
        assert!(t.pending.is_none());

        let mut t = Triggers::default();
        press(&mut t, 0, 0);
        run(&mut t, 100, 2_500, 200);
        assert!(t.is_gesture_active());
        t.update(2_500, &input(false, true), Some(200), sitting_gestures);
        assert!(!t.is_gesture_active());
    }

    #[test]
    fn a_gesture_not_in_the_list_is_ignored() {
        let mut t = Triggers::default();
        let mut other = input(true, false);
        other.requested_gesture = 5;
        t.update(0, &other, Some(0), sitting_gestures);
        assert!(t.pending.is_none());
    }

    #[test]
    fn idle_needs_the_grace_period_and_resets_on_input() {
        let mut t = Triggers::default();
        assert!(!t.is_idle(0), "unknown before the first frame");
        run(&mut t, 0, 4_900, 0);
        assert!(!t.is_idle(4_900));
        run(&mut t, 4_900, 5_100, 0);
        assert!(t.is_idle(5_100));
        t.update(5_200, &input(false, true), Some(0), sitting_gestures);
        assert!(!t.is_idle(5_200));
    }

    #[test]
    fn the_skill_latch_follows_the_last_attack_button() {
        let mut t = Triggers::default();
        let mut l2 = input(false, false);
        l2.l2 = true;
        t.update(0, &l2, None, sitting_gestures);
        assert!(is_last_attack_skill());
        t.update(100, &input(false, false), None, sitting_gestures); // released: stays latched
        assert!(is_last_attack_skill());
        let mut r1 = input(false, false);
        r1.r1 = true;
        t.update(200, &r1, None, sitting_gestures);
        assert!(!is_last_attack_skill());
    }

    #[test]
    fn gesture_ids_skip_bad_entries() {
        assert_eq!(parse_gesture_ids("80, 90,x,,101"), vec![80, 90, 101]);
        assert_eq!(parse_gesture_ids(DEFAULT_GESTURE_IDS).len(), 10);
    }
}
