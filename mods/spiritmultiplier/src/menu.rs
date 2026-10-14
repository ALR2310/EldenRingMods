//! Optional in-game menu via the ERNativeUI host (see `common::erui`).
//! Without `ERNativeUI.dll` the mod works exactly as before, from the ini
//! file only. Layout (System > Game Options):
//!
//! ```text
//! Spirit Multiplier           -> opens a page with:
//!   Spirit Count              -> Multiplier   number -1000..1000
//!                                Fixed Count  number 0..1000 (0 = use Multiplier)
//!                                Max Spirits  number 60..1000 (restart only)
//!   Summoning                 -> Multi Spirit, Clone Spirit, Resummon Without Rest,
//!                                Summon Anywhere (toggles)
//!   Health and Look           -> Regen Mode (choice), Regen Amount (number),
//!                                Ghostly Tint (toggle)
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
//! and is left alone until the player changes the row.

use std::sync::OnceLock;
use std::sync::atomic::Ordering;
use std::time::Duration;

use common::erui::{self, NumberSetting, PageHandle, Provider, Registration, RowHandle, Setting};
use common::{config, logger};

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

fn labels(table: &Table) -> Vec<&'static str> {
    table.iter().map(|(label, _)| *label).collect()
}

fn flag(value: u8) -> String {
    (value != 0).to_string()
}

/// A choice row's ini key plus the function giving its current row value.
macro_rules! choice {
    ($setting:ident, $row:ident, $key:literal, $default:expr, $table:ident) => {
        static $setting: Setting = Setting {
            key: $key,
            format: |index| $table[(index as usize).min($table.len() - 1)].1.to_string(),
            apply: None,
        };
        fn $row() -> u8 {
            nearest($table, config::get_double($key, $default))
        }
    };
}

macro_rules! toggle {
    ($setting:ident, $row:ident, $key:literal, $default:expr) => {
        static $setting: Setting = Setting { key: $key, format: flag, apply: None };
        fn $row() -> u8 {
            config::get_bool($key, $default) as u8
        }
    };
}

static MULTIPLIER: NumberSetting = NumberSetting::new("Multiplier", -1000.0, 1000.0, None);
static AMOUNT: NumberSetting = NumberSetting::whole("Amount", 0.0, 1000.0, None);
static MAX_SPIRITS: NumberSetting = NumberSetting::whole("MaxSpirits", 60.0, 1000.0, None);
toggle!(MULTI_SPIRIT, multi_spirit_row, "MultiSpirit", true);
toggle!(CLONE_SPIRIT, clone_spirit_row, "CloneSpirit", false);
toggle!(NO_REST, no_rest_row, "NoRestResummon", true);
toggle!(ANYWHERE, anywhere_row, "SummonAnywhere", true);
choice!(REGEN_MODE, regen_mode_row, "RegenMode", 2.0, REGEN_MODE_CHOICES);
static REGEN_VALUE: NumberSetting = NumberSetting::new("RegenValue", -1000.0, 1000.0, None);
toggle!(GHOST_COLOR, ghost_color_row, "GhostColor", true);
toggle!(SENSES, senses_row, "ImproveSenses", true);
toggle!(AGGRESSION, aggression_row, "ImproveAggression", true);
toggle!(FOLLOW, follow_row, "ImproveFollow", true);
toggle!(WARP_WHEN_FAR, warp_when_far_row, "WarpWhenFar", false);
static WARP_DISTANCE: NumberSetting = NumberSetting::new("WarpDistance", 0.0, 1000.0, None);
static WARP_BLOCKED: NumberSetting = NumberSetting::new("WarpBlockedTime", 0.0, 600.0, None);
static WARP_STUCK_TIME: NumberSetting = NumberSetting::new("WarpStuckTime", 0.0, 600.0, None);
static WARP_STUCK_RANGE: NumberSetting = NumberSetting::new("WarpStuckRange", 0.0, 100.0, None);
toggle!(LOG_FILE, log_file_row, "LogFile", false);

struct Rows {
    registration: Registration,
    /// Each row with the function that reads its current value from the config.
    rows: Vec<(RowHandle, fn() -> u8)>,
    /// The typed-number rows, with the function that reads each from the config.
    numbers: Vec<(&'static NumberSetting, fn() -> f64)>,
}

static ROWS: OnceLock<Rows> = OnceLock::new();

enum Kind {
    Toggle,
    Choice(&'static Table),
}

fn number_row(
    p: &Provider,
    numbers: &mut Vec<(&'static NumberSetting, fn() -> f64)>,
    page: PageHandle,
    label: &str,
    help: &str,
    setting: &'static NumberSetting,
    read: fn() -> f64,
) -> Result<(), String> {
    p.add_number(page, label, help, setting, read())?;
    numbers.push((setting, read));
    Ok(())
}

fn build(hmodule: u64, connection: erui::Connection) -> Result<Rows, String> {
    let provider = connection.register_provider("spirit-multiplier", "Spirit Multiplier", hmodule)?;
    let mut rows: Vec<(RowHandle, fn() -> u8)> = Vec::new();
    let mut numbers: Vec<(&'static NumberSetting, fn() -> f64)> = Vec::new();
    {
        let mut add =
            |page: PageHandle, label: &str, help: &str, kind: Kind, setting: &'static Setting, read: fn() -> u8| -> Result<(), String> {
                let row = match kind {
                    Kind::Toggle => {
                        provider.add_toggle(page, label, help, read() != 0, erui::setting_changed, setting.user_data())?
                    }
                    Kind::Choice(table) => provider.add_inline_choice(
                        page,
                        label,
                        help,
                        &labels(table),
                        read(),
                        erui::setting_changed,
                        setting.user_data(),
                    )?,
                };
                rows.push((row, read));
                Ok(())
            };

        let root = provider.root_page();
        let (page, _entry) = provider.add_submenu(
            root,
            "Spirit Multiplier",
            "Settings for the Spirit Multiplier mod.",
            "Spirit Multiplier",
            "Settings for the Spirit Multiplier mod.",
        )?;
        let sub = |label: &str, help: &str| provider.add_submenu(page, label, help, label, help).map(|(p, _)| p);

        let count = sub("Spirit Count", "How many spirits each Spirit Ash summons.")?;
        number_row(
            &provider,
            &mut numbers,
            count,
            "Multiplier",
            "Times an Ash's usual spirit count (decimals allowed, e.g. 1.5). Used when Fixed Count is 0. 1 = vanilla.",
            &MULTIPLIER,
            || config::get_double("Multiplier", 2.0),
        )?;
        number_row(
            &provider,
            &mut numbers,
            count,
            "Fixed Count",
            "Spirits every Ash summons, whatever its usual count. 0 = use the Multiplier instead.",
            &AMOUNT,
            || config::get_double("Amount", 0.0),
        )?;
        number_row(
            &provider,
            &mut numbers,
            count,
            "Max Spirits",
            "Most spirits one Ash can summon. Above 60 may lower FPS or crash. Applies after restarting the game.",
            &MAX_SPIRITS,
            || config::get_double("MaxSpirits", 60.0),
        )?;

        let summoning = sub("Summoning", "How and where spirits can be summoned.")?;
        add(
            summoning,
            "Multi Spirit",
            "Summon several different Spirit Ashes at the same time.",
            Kind::Toggle,
            &MULTI_SPIRIT,
            multi_spirit_row,
        )?;
        add(
            summoning,
            "Clone Spirit",
            "Needs Multi Spirit. Using an Ash again summons another copy instead of sending it back.",
            Kind::Toggle,
            &CLONE_SPIRIT,
            clone_spirit_row,
        )?;
        add(
            summoning,
            "Resummon Without Rest",
            "Summon your spirits again without resting at a Site of Grace.",
            Kind::Toggle,
            &NO_REST,
            no_rest_row,
        )?;
        add(
            summoning,
            "Summon Anywhere",
            "Summon spirits outside the usual summoning pool areas.",
            Kind::Toggle,
            &ANYWHERE,
            anywhere_row,
        )?;

        let health = sub("Health and Look", "How spirits heal and how they look.")?;
        add(
            health,
            "Regen Mode",
            "What the healing amount means. Also heals Torrent.",
            Kind::Choice(REGEN_MODE_CHOICES),
            &REGEN_MODE,
            regen_mode_row,
        )?;
        number_row(
            &provider,
            &mut numbers,
            health,
            "Regen Amount",
            "Healing per second: HP in Flat HP mode, a percent in the other modes (decimals allowed). 0 = no healing.",
            &REGEN_VALUE,
            || config::get_double("RegenValue", 0.5),
        )?;
        add(
            health,
            "Ghostly Tint",
            "On = spirits keep their ghostly blue look. Off = they look like normal characters.",
            Kind::Toggle,
            &GHOST_COLOR,
            ghost_color_row,
        )?;

        let behavior = sub("Behavior", "Make spirits smarter in a fight.")?;
        add(
            behavior,
            "Senses",
            "Spirits notice enemies from farther away and remember them longer.",
            Kind::Toggle,
            &SENSES,
            senses_row,
        )?;
        add(
            behavior,
            "Aggression",
            "Spirits start fighting sooner, attack together and call for help.",
            Kind::Toggle,
            &AGGRESSION,
            aggression_row,
        )?;
        add(
            behavior,
            "Follow",
            "Spirits stay closer to you and cross ledges, ladders and holes to keep up.",
            Kind::Toggle,
            &FOLLOW,
            follow_row,
        )?;

        let teleport = sub("Teleport", "Experimental: spirits teleport to you sooner. 0 = game default.")?;
        add(
            teleport,
            "Warp When Far",
            "Also teleport spirits that are far away even if they can see you.",
            Kind::Toggle,
            &WARP_WHEN_FAR,
            warp_when_far_row,
        )?;
        number_row(
            &provider,
            &mut numbers,
            teleport,
            "Distance (m)",
            "Teleport when a spirit is farther than this many metres. 0 = game default.",
            &WARP_DISTANCE,
            || config::get_double("WarpDistance", 0.0),
        )?;
        number_row(
            &provider,
            &mut numbers,
            teleport,
            "Blocked Time (s)",
            "Teleport after this many seconds without line of sight. 0 = game default.",
            &WARP_BLOCKED,
            || config::get_double("WarpBlockedTime", 0.0),
        )?;
        number_row(
            &provider,
            &mut numbers,
            teleport,
            "Stuck Time (s)",
            "Teleport after being stuck this many seconds. 0 = game default.",
            &WARP_STUCK_TIME,
            || config::get_double("WarpStuckTime", 0.0),
        )?;
        number_row(
            &provider,
            &mut numbers,
            teleport,
            "Stuck Range (m)",
            "Moving less than this many metres counts as stuck. 0 = game default.",
            &WARP_STUCK_RANGE,
            || config::get_double("WarpStuckRange", 0.0),
        )?;

        add(
            page,
            "Log file",
            "Write a log file next to the DLL, for troubleshooting.",
            Kind::Toggle,
            &LOG_FILE,
            log_file_row,
        )?;
    }
    let registration = provider.commit()?;
    Ok(Rows { registration, rows, numbers })
}

/// Registers the menu on a worker thread. Must start early: the host closes
/// registration shortly after startup.
pub fn start(hmodule: u64, ini_path: String) {
    // Every setting is re-read by the mod on its own; no reload request needed.
    erui::start_saver(ini_path, None);
    std::thread::spawn(move || {
        let connection = match erui::connect(Duration::from_secs(10)) {
            Ok(c) => c,
            Err(reason) => {
                logger::log(&format!("In-game menu unavailable ({reason}); using the ini only."));
                return;
            }
        };
        match build(hmodule, connection) {
            Ok(rows) => {
                let _ = ROWS.set(rows);
                logger::log("In-game menu: registered.");
            }
            Err(reason) => {
                logger::log(&format!("In-game menu registration failed: {reason}"));
                return;
            }
        }
        // Follow the reload key: show the reloaded values in the menu.
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
    if let Some(rows) = ROWS.get() {
        for (row, read) in &rows.rows {
            if let Err(reason) = rows.registration.set_row_value(*row, read()) {
                logger::log(&format!("In-game menu: couldn't update a row ({reason})."));
            }
        }
        for (setting, read) in &rows.numbers {
            if let Err(reason) = setting.show(&rows.registration, read()) {
                logger::log(&format!("In-game menu: couldn't update a row ({reason})."));
            }
        }
    }
}
