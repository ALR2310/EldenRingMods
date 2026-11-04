//! In-game Toolbox GUI (Dear ImGui over DX12 via `shared::overlay`).
//! Opened/closed with `MenuKey` (default `F8`). Lets you:
//! - Click a button for any of the 7 status buildup bars (Poison, Scarlet Rot,
//!   Bleed, Death Blight, Frostbite, Sleep, Madness) to trigger/add buildup
//!   immediately, or clear individual / all bars, and adjust `BuildupAmount`
//!   live (saved to `Toolbox.toml`).
//! - Toggle `MapAnywhere` and `TorrentAnywhere` in `Toolbox.toml` (startup code
//!   patches, so a note reminds you to restart when changed).

use std::sync::atomic::{AtomicBool, Ordering};

use shared::logger;
use shared::overlay::{self, GOLD, GOLD_DIM, TEXT_MUTED, Ui, WindowConfig};

use crate::buildup::{self, BARS, GuiAction};
use crate::config;

const WINDOW_SIZE_AT_1080P: [f32; 2] = [460.0, 560.0];
const WINDOW_POS_AT_1080P: [f32; 2] = [70.0, 70.0];

static MAP_PATCHED_AT_START: AtomicBool = AtomicBool::new(false);
static TORRENT_PATCHED_AT_START: AtomicBool = AtomicBool::new(false);

pub fn record_startup_state(map_on: bool, torrent_on: bool) {
    MAP_PATCHED_AT_START.store(map_on, Ordering::Relaxed);
    TORRENT_PATCHED_AT_START.store(torrent_on, Ordering::Relaxed);
}

fn draw_body(ui: &Ui) {
    let snap = buildup::snapshot();
    let pad = ui.clone_style().frame_padding;

    // --- Section 1: Status Buildup -----------------------------------------
    ui.text_colored(GOLD, "Status Buildup (Self)");
    if !snap.in_game {
        ui.text_colored(TEXT_MUTED, "Load into a character to apply buildup.");
    }

    // BuildupAmount slider (0 = instant full bar, or step points).
    let mut amount = config::get().buildup.amount.clamp(0, 1000);
    ui.set_next_item_width(-140.0);
    if ui
        .slider_config("Amount (0 = Full)", 0, 500)
        .display_format(if amount <= 0 { "Full bar (instant)" } else { "%d pts" })
        .build(&mut amount)
    {
        config::save(&[("Buildup.Amount", amount.to_string())]);
    }

    let _disabled = ui.begin_disabled(!snap.in_game);
    for (i, bar) in BARS.iter().enumerate() {
        let (cur, max) = snap.bars[i];
        let filled = (max - cur).max(0);
        let clear_label = format!("0##clear_{}", bar.name);
        let clear_w = ui.calc_text_size("0")[0] + pad[0] * 2.0;
        let avail = ui.content_region_avail()[0];
        let main_w = (avail - clear_w - 8.0).max(120.0);

        let btn_text = if snap.in_game && max > 0 {
            format!("+ {}   [{filled} / {max}]##apply_{}", bar.label, bar.name)
        } else {
            format!("+ {}##apply_{}", bar.label, bar.name)
        };
        if ui.button_with_size(&btn_text, [main_w, 0.0]) {
            buildup::queue(GuiAction::Apply(i));
        }
        ui.same_line();
        if ui.button_with_size(&clear_label, [clear_w, 0.0]) {
            buildup::queue(GuiAction::ClearOne(i));
        }
        if ui.is_item_hovered() {
            ui.tooltip_text(format!("Clear {} buildup", bar.label));
        }
    }

    if ui.button_with_size("Clear All Buildup", [-1.0, 0.0]) {
        buildup::queue(GuiAction::ClearAll);
    }
    drop(_disabled);

    // --- Section 2: World Patches ------------------------------------------
    ui.spacing();
    ui.separator();
    ui.spacing();
    ui.text_colored(GOLD, "Patches (saved to Toolbox.toml)");

    let mut map_on = config::get().map_anywhere.enabled;
    if ui.checkbox("Map & Fast Travel Anywhere (boss / caves)", &mut map_on) {
        config::save(&[("MapAnywhere.Enabled", map_on.to_string())]);
    }
    let mut torrent_on = config::get().torrent_anywhere.enabled;
    if ui.checkbox("Summon Torrent Anywhere", &mut torrent_on) {
        config::save(&[("TorrentAnywhere.Enabled", torrent_on.to_string())]);
    }

    let need_restart = map_on != MAP_PATCHED_AT_START.load(Ordering::Relaxed)
        || torrent_on != TORRENT_PATCHED_AT_START.load(Ordering::Relaxed);
    if need_restart {
        ui.text_colored(GOLD_DIM, "* Restart the game to apply patch toggle changes.");
    }

    if !snap.status.is_empty() {
        ui.spacing();
        ui.separator();
        let _wrap = ui.push_text_wrap_pos();
        ui.text_colored(GOLD_DIM, &snap.status);
    }
}

/// Hooks the overlay after `General.LoadDelay` ms. Other overlay mods hook the game's DX12 swap
/// chain too; holding this back keeps the hooks from overlapping (`overlay::install` has its
/// own wait and lock on top). Blocks this thread, so call it on its own.
pub fn install() {
    overlay::set_store(config::overlay_store());
    let delay = config::get().general.load_delay;
    if delay > 0 {
        logger::log(&format!("Menu: waiting {delay} ms before hooking"));
        std::thread::sleep(std::time::Duration::from_millis(u64::from(delay)));
    }
    overlay::install(
        WindowConfig {
            title: "Toolbox",
            id: "##toolbox",
            default_size: WINDOW_SIZE_AT_1080P,
            default_pos: WINDOW_POS_AT_1080P,
        },
        draw_body,
    );
}
