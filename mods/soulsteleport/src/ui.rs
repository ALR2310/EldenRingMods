//! In-game menu listing co-op partners; clicking one teleports there. Built on
//! `common::overlay` (`shared/docs/overlay.md`). The menu runs on the render
//! thread and only shares [SHARED] (a snapshot plus status text) and
//! [REQUEST] (the partner clicked) with the game task in `warp.rs`. Details:
//! `docs/menu_ui.md`.

use std::sync::Mutex;

use common::overlay::{
    self, ERROR_TEXT, GOLD, GOLD_DIM, TEXT_MUTED, Ui, WindowConfig, centered_text,
};

pub use common::overlay::{MENU_OPEN, set_ini_path, toggle};

#[derive(Clone)]
pub struct PartnerRow {
    pub steam_id: u64,
    pub character_name: String,
    pub steam_name: String,
    pub role: &'static str,
}

#[derive(Default)]
pub struct Shared {
    pub in_session: bool,
    /// Our own character is hostile (invading) - teleport is disabled.
    pub hostile: bool,
    pub partners: Vec<PartnerRow>,
    /// Last outcome/progress line ("Locating X...", "X did not respond").
    /// Private: set through [set_status] / [Shared::set_status] so
    /// `status_set` stays in sync.
    status: String,
    /// When `status` was last set - it's cleared [STATUS_TTL] later (unless a
    /// request is still in flight), so no line can hang around forever.
    status_set: Option<std::time::Instant>,
    /// A WHERE is in flight - buttons disabled until it resolves.
    pub busy: bool,
}

pub static SHARED: Mutex<Option<Shared>> = Mutex::new(None);

/// SteamID of the partner the player clicked, for the game task to take.
pub static REQUEST: Mutex<Option<u64>> = Mutex::new(None);

pub fn with_shared<R>(f: impl FnOnce(&mut Shared) -> R) -> R {
    let mut guard = SHARED.lock().unwrap();
    f(guard.get_or_insert_with(Shared::default))
}

const STATUS_TTL: std::time::Duration = std::time::Duration::from_secs(15);

impl Shared {
    pub fn set_status(&mut self, text: impl Into<String>) {
        self.status = text.into();
        self.status_set = Some(std::time::Instant::now());
    }

    fn current_status(&mut self) -> String {
        if !self.busy && self.status_set.is_some_and(|t| t.elapsed() >= STATUS_TTL) {
            self.status.clear();
            self.status_set = None;
        }
        self.status.clone()
    }
}

pub fn set_status(text: impl Into<String>) {
    with_shared(|s| s.set_status(text));
}

const WINDOW_SIZE_AT_1080P: [f32; 2] = [420.0, 360.0];
const WINDOW_POS_AT_1080P: [f32; 2] = [60.0, 60.0];

fn role_color(role: &str) -> [f32; 4] {
    match role {
        "Host" => GOLD,
        "Invader" | "Bloody Finger" | "Recusant" => ERROR_TEXT,
        "Unknown" | "Other" => TEXT_MUTED,
        _ => GOLD_DIM,
    }
}

fn draw_body(ui: &Ui) {
    let (in_session, hostile, partners, status, busy) =
        with_shared(|s| (s.in_session, s.hostile, s.partners.clone(), s.current_status(), s.busy));
    let pad = ui.clone_style().frame_padding;

    if !in_session {
        centered_text(ui, TEXT_MUTED, "Not in a co-op session.");
    } else if hostile {
        centered_text(ui, TEXT_MUTED, "Teleport is disabled while invading.");
    } else if partners.is_empty() {
        centered_text(ui, TEXT_MUTED, "No other player in this session.");
    } else {
        ui.text_colored(TEXT_MUTED, format!("Players ({})", partners.len()));
        let _disabled = ui.begin_disabled(busy);
        for p in &partners {
            let label = format!("{}##{}", p.character_name, p.steam_id);
            let role_w = ui.calc_text_size(p.role)[0];
            let row_start = ui.cursor_pos();
            if ui.button_with_size(&label, [-1.0, 0.0]) {
                *REQUEST.lock().unwrap() = Some(p.steam_id);
            }
            let hovered = ui.is_item_hovered();
            let after = ui.cursor_pos();
            let x = ui.window_content_region_max()[0] - role_w - pad[0];
            ui.set_cursor_pos([x, row_start[1] + pad[1]]);
            ui.text_colored(role_color(p.role), p.role);
            ui.set_cursor_pos(after);
            if hovered {
                ui.tooltip_text(format!("Steam: {}", p.steam_name));
            }
        }
    }

    if !status.is_empty() {
        ui.spacing();
        ui.separator();
        let color = if status.contains("did not respond") || status.contains("Can't") || status.contains("no longer") {
            ERROR_TEXT
        } else {
            GOLD
        };
        let _wrap = ui.push_text_wrap_pos();
        ui.text_colored(color, &status);
    }
}

/// Installs the DX12 hooks. Call once from `DllMain`'s worker thread.
pub fn install() {
    overlay::install(
        WindowConfig {
            title: "Souls Teleport",
            id: "##souls_teleport",
            default_size: WINDOW_SIZE_AT_1080P,
            default_pos: WINDOW_POS_AT_1080P,
        },
        draw_body,
    );
}
