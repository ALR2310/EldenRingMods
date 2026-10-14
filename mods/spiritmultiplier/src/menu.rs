//! Optional in-game menu via the ERNativeUI host (see `common::erui`).
//! Without `ERNativeUI.dll` the mod works exactly as before, from the ini
//! file only. Layout (System > Game Options):
//!
//! ```text
//! Spirit Multiplier           -> opens a page with:
//!   Spirit Count              -> Multiplier   number 0..1000 (0 = off, vanilla count)
//!                                Fixed Count  number 0..1000 (0 = use Multiplier)
//!                                Max Spirits  number 60..1000 (restart only)
//!   Summoning                 -> Multi Spirit, Clone Spirit, Resummon Without Rest,
//!                                Summon Anywhere (toggles)
//!   Health and Look           -> Regen Mode (choice), Regen Amount (number 0..1000,
//!                                0 = no healing), Ghostly Tint (toggle)
//!   Behavior                  -> Senses, Aggression, Follow (toggles)
//!   Teleport                  -> Warp When Far (toggle), Distance, Blocked Time,
//!                                Stuck Time, Stuck Range (numbers, 0 = game default)
//!   Log file                  on/off
//! ```
//!
//! Every setting except `MaxSpirits` is re-read by the mod once a second (or
//! every tick), so a change is live as soon as the saver has written it to
//! the config map. `MaxSpirits` sizes the summon slot array at startup, so
//! its row says it needs a restart. Numbers are typed into text rows (the ini
//! holds decimals and values wider than a byte); the regen mode stays a choice
//! row, and an ini value that is not one of its entries shows the closest one
//! and is left alone until the player changes the row. `Multiplier` and
//! `RegenValue` start at 0 because the mod ignores values <= 0 (no multiplying,
//! no healing), so a negative number would only mean the same as 0.

use std::sync::OnceLock;
use std::sync::atomic::Ordering;
use std::time::Duration;

use common::erui::{self, MenuBuilder, NumberSetting, Setting};
use common::config;

/// `(label, ini text)` of one choice; the ini text is also what `nearest` compares.
type Table = [(&'static str, &'static str)];

const REGEN_MODE_CHOICES: &Table = &[("Flat HP", "1"), ("% of max HP", "2"), ("% of missing HP", "3")];

/// Index of the table entry closest to `value`.
fn nearest(table: &Table, value: f64) -> u8 {
    let mut best = (0usize, f64::MAX);
    for (i, (_, text)) in table.iter().enumerate() {
        let distance = (text.parse::<f64>().unwrap_or(0.0) - value).abs();
        if distance < best.1 {
            best = (i, distance);
        }
    }
    best.0 as u8
}

fn regen_mode_row() -> u8 {
    nearest(REGEN_MODE_CHOICES, config::get_double("RegenMode", 2.0))
}

static MENU: OnceLock<erui::Menu> = OnceLock::new();

static MULTIPLIER: NumberSetting = NumberSetting::new("Multiplier", 0.0, 1000.0, None);
static AMOUNT: NumberSetting = NumberSetting::whole("Amount", 0.0, 1000.0, None);
static MAX_SPIRITS: NumberSetting = NumberSetting::whole("MaxSpirits", 60.0, 1000.0, None);
static MULTI_SPIRIT: Setting = Setting::flag("MultiSpirit");
static CLONE_SPIRIT: Setting = Setting::flag("CloneSpirit");
static NO_REST: Setting = Setting::flag("NoRestResummon");
static ANYWHERE: Setting = Setting::flag("SummonAnywhere");
static REGEN_MODE: Setting = Setting {
    key: "RegenMode",
    format: |index| REGEN_MODE_CHOICES[(index as usize).min(REGEN_MODE_CHOICES.len() - 1)].1.to_string(),
    apply: None,
};
static REGEN_VALUE: NumberSetting = NumberSetting::new("RegenValue", 0.0, 1000.0, None);
static GHOST_COLOR: Setting = Setting::flag("GhostColor");
static SENSES: Setting = Setting::flag("ImproveSenses");
static AGGRESSION: Setting = Setting::flag("ImproveAggression");
static FOLLOW: Setting = Setting::flag("ImproveFollow");
static WARP_WHEN_FAR: Setting = Setting::flag("WarpWhenFar");
static WARP_DISTANCE: NumberSetting = NumberSetting::new("WarpDistance", 0.0, 1000.0, None);
static WARP_BLOCKED: NumberSetting = NumberSetting::new("WarpBlockedTime", 0.0, 600.0, None);
static WARP_STUCK_TIME: NumberSetting = NumberSetting::new("WarpStuckTime", 0.0, 600.0, None);
static WARP_STUCK_RANGE: NumberSetting = NumberSetting::new("WarpStuckRange", 0.0, 100.0, None);
static LOG_FILE: Setting = Setting::flag("LogFile");

fn build(b: &mut MenuBuilder) -> Result<(), String> {
    let page = b.submenu(b.root(), "Spirit Multiplier", "Settings for the Spirit Multiplier mod.")?;

    let count = b.submenu(page, "Spirit Count", "How many spirits each Spirit Ash summons.")?;
    b.number(
        count,
        "Multiplier",
        "Times an Ash's usual spirit count (decimals allowed, e.g. 1.5). Used when Fixed Count is 0. 1 = vanilla, 0 = off.",
        &MULTIPLIER,
        || config::get_double("Multiplier", 2.0),
    )?;
    b.number(
        count,
        "Fixed Count",
        "Spirits every Ash summons, whatever its usual count. 0 = use the Multiplier instead.",
        &AMOUNT,
        || config::get_double("Amount", 0.0),
    )?;
    b.number(
        count,
        "Max Spirits",
        "Most spirits one Ash can summon. Above 60 may lower FPS or crash. Applies after restarting the game.",
        &MAX_SPIRITS,
        || config::get_double("MaxSpirits", 60.0),
    )?;

    let summoning = b.submenu(page, "Summoning", "How and where spirits can be summoned.")?;
    b.toggle(
        summoning,
        "Multi Spirit",
        "Summon several different Spirit Ashes at the same time.",
        &MULTI_SPIRIT,
        || config::get_bool("MultiSpirit", true) as u8,
    )?;
    b.toggle(
        summoning,
        "Clone Spirit",
        "Needs Multi Spirit. Using an Ash again summons another copy instead of sending it back.",
        &CLONE_SPIRIT,
        || config::get_bool("CloneSpirit", false) as u8,
    )?;
    b.toggle(
        summoning,
        "Resummon Without Rest",
        "Summon your spirits again without resting at a Site of Grace.",
        &NO_REST,
        || config::get_bool("NoRestResummon", true) as u8,
    )?;
    b.toggle(
        summoning,
        "Summon Anywhere",
        "Summon spirits outside the usual summoning pool areas.",
        &ANYWHERE,
        || config::get_bool("SummonAnywhere", true) as u8,
    )?;

    let health = b.submenu(page, "Health and Look", "How spirits heal and how they look.")?;
    let labels: Vec<&str> = REGEN_MODE_CHOICES.iter().map(|(label, _)| *label).collect();
    b.choice(health, "Regen Mode", "What the healing amount means. Also heals Torrent.", &labels, &REGEN_MODE, regen_mode_row)?;
    b.number(
        health,
        "Regen Amount",
        "Healing per second: HP in Flat HP mode, a percent in the other modes (decimals allowed). 0 = no healing.",
        &REGEN_VALUE,
        || config::get_double("RegenValue", 0.5),
    )?;
    b.toggle(
        health,
        "Ghostly Tint",
        "On = spirits keep their ghostly blue look. Off = they look like normal characters.",
        &GHOST_COLOR,
        || config::get_bool("GhostColor", true) as u8,
    )?;

    let behavior = b.submenu(page, "Behavior", "Make spirits smarter in a fight.")?;
    b.toggle(
        behavior,
        "Senses",
        "Spirits notice enemies from farther away and remember them longer.",
        &SENSES,
        || config::get_bool("ImproveSenses", true) as u8,
    )?;
    b.toggle(
        behavior,
        "Aggression",
        "Spirits start fighting sooner, attack together and call for help.",
        &AGGRESSION,
        || config::get_bool("ImproveAggression", true) as u8,
    )?;
    b.toggle(
        behavior,
        "Follow",
        "Spirits stay closer to you and cross ledges, ladders and holes to keep up.",
        &FOLLOW,
        || config::get_bool("ImproveFollow", true) as u8,
    )?;

    let teleport = b.submenu(page, "Teleport", "Experimental: spirits teleport to you sooner. 0 = game default.")?;
    b.toggle(
        teleport,
        "Warp When Far",
        "Also teleport spirits that are far away even if they can see you.",
        &WARP_WHEN_FAR,
        || config::get_bool("WarpWhenFar", false) as u8,
    )?;
    b.number(
        teleport,
        "Distance (m)",
        "Teleport when a spirit is farther than this many metres. 0 = game default.",
        &WARP_DISTANCE,
        || config::get_double("WarpDistance", 0.0),
    )?;
    b.number(
        teleport,
        "Blocked Time (s)",
        "Teleport after this many seconds without line of sight. 0 = game default.",
        &WARP_BLOCKED,
        || config::get_double("WarpBlockedTime", 0.0),
    )?;
    b.number(
        teleport,
        "Stuck Time (s)",
        "Teleport after being stuck this many seconds. 0 = game default.",
        &WARP_STUCK_TIME,
        || config::get_double("WarpStuckTime", 0.0),
    )?;
    b.number(
        teleport,
        "Stuck Range (m)",
        "Moving less than this many metres counts as stuck. 0 = game default.",
        &WARP_STUCK_RANGE,
        || config::get_double("WarpStuckRange", 0.0),
    )?;

    b.toggle(
        page,
        "Log file",
        "Write a log file next to the DLL, for troubleshooting.",
        &LOG_FILE,
        || config::get_bool("LogFile", false) as u8,
    )?;
    Ok(())
}

/// Registers the menu on a worker thread. Must start early: the host closes
/// registration shortly after startup.
pub fn start(hmodule: u64, ini_path: String) {
    // Every setting is re-read by the mod on its own; no reload request needed.
    erui::start_saver(ini_path, None);
    erui::spawn_menu(&MENU, hmodule, "spirit-multiplier", "Spirit Multiplier", build);
    // Follow the reload key: show the reloaded values in the menu.
    std::thread::spawn(|| {
        let mut seen = common::reload::RELOAD_GENERATION.load(Ordering::Relaxed);
        loop {
            std::thread::sleep(Duration::from_millis(500));
            let now = common::reload::RELOAD_GENERATION.load(Ordering::Relaxed);
            if now != seen {
                seen = now;
                sync_from_config();
            }
        }
    });
}

/// After an ini reload: show the reloaded values in the menu.
pub fn sync_from_config() {
    if let Some(menu) = MENU.get() {
        menu.sync();
    }
}
