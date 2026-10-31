//! Overlay: Dear ImGui over the game's DX12 frames via hudhook. Draws the
//! target panel on the foreground draw list (no window, so it never takes
//! input or focus) from the snapshot [crate::target] keeps up to date.
//! Details: `docs/design.md`.

use std::sync::atomic::{AtomicU32, Ordering};

use hudhook::hooks::dx12::ImguiDx12Hooks;
use hudhook::imgui::{Context, DrawListMut, Ui};
use hudhook::{Hudhook, ImguiRenderLoop, RenderContext};

use common::{input, logger};

use crate::map::layer::{Icon, MapLayer};
use crate::radar::{self, Snapshot as RadarSnapshot};
use crate::radar_ui::{self, Placement, Radar};
use crate::settings;
use crate::target::{self, Buildup, Effect, Target};
use crate::text;
use crate::textures::{self, Textures};

/// The game draws its HUD in a 16:9 area fitted into the window (black bars on
/// the other axis), authored for 1080p. Everything here is laid out in that
/// area, in 1080p pixels, and scaled by its real height.
const ASPECT: f32 = 16.0 / 9.0;
const BASE_HEIGHT: f32 = 1080.0;

/// The thickness of the health bar at 1080p (the player's own is about 12); the length is a
/// setting.
const HP_HEIGHT: f32 = 13.0;
const POISE_HEIGHT: f32 = 12.0;
/// Space between the health bar and the posture bar. The frames' lines (2.4 px, centered on the edges of
/// the bars) take 1.2 px of it on each side, so a gap shows only from about 3 px.
const BAR_GAP: f32 = 5.0;
/// Font size of the numbers inside the health and the posture bar, at 1080p: small enough that
/// the digits stay inside bars 12 px thick.
const BAR_NUMBER: f32 = 10.0;
/// The buildup bars under the tan line: a row each, with the icon, the bar and the gap to the
/// bars above (and the posture numbers, which need a row of their own when shown).
const BUILDUP_ROW: f32 = 25.0;
const BUILDUP_ICON: f32 = 24.0;
const BUILDUP_ICON_GAP: f32 = 0.0;
const BUILDUP_BAR: f32 = 12.0;
const BUILDUP_GAP: f32 = 4.0;
/// The target's effects, a cell each: the icon with the time left under it, filling from the right
/// edge under its buildup bars. The player's effects have no icons of the mod: the game's own row
/// of status icons shows them, and the times go under those (`draw_icon_times`).
const EFFECT_CELL: f32 = 42.0;
const EFFECT_ICON: f32 = 38.0;
/// The target's cells are this much of the size of the game's own status icons (38 px); the time
/// text keeps a size that reads, `EFFECT_TIME_TARGET`.
const TARGET_EFFECTS_SCALE: f32 = 0.77;
const EFFECT_TIME_TARGET: f32 = 10.0;
const EFFECT_ROW: f32 = 54.0;
/// Rows of cells at most.
const EFFECT_ROWS: usize = 2;
/// Most cells in a row of the target's effects (fewer when the width of its bars is short).
const PLAYER_EFFECTS_PER_ROW: usize = 6;
/// Font size of the times under the game's own icons, at 1080p.
const GAME_ICON_TIME: f32 = 12.0;
const EFFECTS_GAP: f32 = 4.0;
/// The game's row of status icons, in 1080p pixels of its picture (measured on screenshots, the
/// slots from `state_icons`): the first icon is centered at x 155.5 and the slots are 42 apart,
/// the diamonds end at y 141, the times start a little lower.
const GAME_ICONS_CENTER_X: f32 = 155.5;
const GAME_ICONS_PITCH: f32 = 42.0;
/// The movie lays the 42 slots out 14 to a row, 41 px between the rows (`Item_0_14` is at (10, 56), the
/// first row's are at y 15); a long list (many talismans with `ExpandedTalismanSlots`, then a buff) wraps.
const GAME_ICONS_PER_ROW: usize = 14;
const GAME_ICONS_ROW_PITCH: f32 = 41.0;
const GAME_ICONS_BOTTOM: f32 = 146.0;
/// Space between the times under the game's icons and the player's buildup gauge.
const GAUGE_UNDER_ICONS_GAP: f32 = 6.0;
/// The icon fades over its last seconds.
const EFFECT_FADE_SECONDS: f32 = 5.0;
/// One color per status, in the order of `target::BUILDUP`.
const BUILDUP_COLORS: [[f32; 4]; 7] = [
    [0.39, 0.44, 0.05, 1.0],
    [0.62, 0.22, 0.10, 1.0],
    [0.60, 0.06, 0.08, 1.0],
    [0.62, 0.58, 0.46, 1.0],
    [0.42, 0.62, 0.74, 1.0],
    [0.52, 0.46, 0.72, 1.0],
    [0.78, 0.58, 0.12, 1.0],
];
/// Thickness of the tan line under the bars.
const LINE: f32 = 2.0;
/// Thickness of one texel row of the game's frame lines, per unit of size: 6 rows make the 2.4 px
/// the game's lines measure.
const WAKU_LINE_TEXEL: f32 = 0.4;
/// ImGui's default font is 13px; this makes the text about 17px at 1080p.
const FONT_SCALE: f32 = 1.3;

/// Seconds the lost-health block stays before it drains, and how fast it
/// drains afterwards (fraction of max health per second).
/// How fast the target panel fades in and out with the game's menus, per second (as the minimap).
const FADE_RATE: f32 = 12.0;

/// Opacity of the target panel this frame, 0 to 1, as `f32` bits (set before it is drawn).
static PANEL_ALPHA: AtomicU32 = AtomicU32::new(0x3F80_0000);

/// `color` with the panel's opacity applied.
/// `color` with its alpha times `alpha`.
fn fade_by([r, g, b, a]: [f32; 4], alpha: f32) -> [f32; 4] {
    [r, g, b, a * alpha]
}

fn fade([r, g, b, a]: [f32; 4]) -> [f32; 4] {
    [
        r,
        g,
        b,
        a * f32::from_bits(PANEL_ALPHA.load(Ordering::Relaxed)),
    ]
}

const TRAIL_HOLD: f32 = 0.9;
const TRAIL_DRAIN: f32 = 0.6;

// Sampled from the game's own bars (2560x1440 screenshots): flat dark red
// whose bottom edge fades toward the tan line, a mustard block for health
// just lost with a thin cream divider, a near-black track and one tan line
// under the bar with a small ornament at each end. No frame around the bar.
const TRACK: [f32; 4] = [0.03, 0.04, 0.04, 0.80];
const LINE_COLOR: [f32; 4] = [0.59, 0.56, 0.45, 1.0];
const HP_CORE: [f32; 4] = [0.33, 0.04, 0.03, 1.0];
const HP_EDGE: [f32; 4] = [0.44, 0.22, 0.17, 1.0];
const TRAIL_COLOR: [f32; 4] = [0.56, 0.46, 0.09, 1.0];
const DIVIDER: [f32; 4] = [0.89, 0.89, 0.75, 1.0];
// A darker gold than the block of lost value (the player's own yellow, `TRAIL_COLOR`), so that
// block shows against the bar.
const POISE_CORE: [f32; 4] = [0.38, 0.29, 0.08, 1.0];
const POISE_EDGE: [f32; 4] = [0.47, 0.37, 0.14, 1.0];
/// The bar texture is grayscale and about this bright on average; a color is
/// divided by it so the tinted result lands on the sampled color.
const TEXTURE_LEVEL: f32 = 0.86;
const TRAIL_TINT: [f32; 4] = [0.65, 0.53, 0.10, 1.0];
/// `BarBG.png` is translucent white; tinted dark it only adds the texture.
const BACKGROUND_TINT: [f32; 4] = [0.25, 0.25, 0.25, 1.0];
const TEXT: [f32; 4] = [0.90, 0.86, 0.74, 1.0];
const TEXT_SHADOW: [f32; 4] = [0.0, 0.0, 0.0, 0.85];

#[derive(Default)]
struct HudRenderLoop {
    /// Top-left and size of the game's 16:9 area, set each frame.
    area: [f32; 4],
    /// Height of the area over 1080 x the `Scale` setting, set each frame.
    s: f32,
    trail: Trail,
    /// The same for the posture bar.
    poise_trail: Trail,
    /// 0 to 1: follows whether the game is in normal play, so the panel goes with its menus.
    panel_alpha: f32,
    /// 0 to 1: follows whether the game shows its row of status icons, for the times under it.
    icons_alpha: f32,
    radar: Radar,
    /// The world map under the minimap; `None` until the player is in the world (see
    /// `before_render`) and when the game folder is unknown.
    map: Option<MapLayer>,
    /// `None` when the textures failed to load: the bars are drawn flat.
    textures: Option<Textures>,
}

/// What was just lost of a bar (health or posture), shown as a block between the current value
/// and where it was, like the game's own bars.
#[derive(Default)]
struct Trail {
    id: usize,
    value: f32,
    /// Where the block starts, `<= value` when there is none.
    from: f32,
    hold: f32,
}

impl Trail {
    /// 1 right after damage, fading to 0 as the block starts to drain.
    fn glow(&self) -> f32 {
        (self.hold / TRAIL_HOLD).clamp(0.0, 1.0)
    }

    /// Advances to the current `value` (of at most `max`) of the enemy `id`; `dt` is the frame
    /// time in seconds.
    fn update(&mut self, id: usize, value: f32, max: f32, dt: f32) {
        if self.id != id {
            *self = Trail {
                id,
                value,
                from: value,
                hold: 0.0,
            };
            return;
        }
        if value < self.value {
            self.from = self.from.max(self.value);
            self.hold = TRAIL_HOLD;
        } else if value > self.value {
            self.from = value;
        }
        self.value = value;
        if self.hold > 0.0 {
            self.hold -= dt;
        } else {
            self.from -= TRAIL_DRAIN * max * dt;
        }
        self.from = self.from.clamp(value, max);
    }
}

impl ImguiRenderLoop for HudRenderLoop {
    fn initialize<'a>(&'a mut self, ctx: &mut Context, render_context: &'a mut dyn RenderContext) {
        // No imgui.ini next to the game exe.
        ctx.set_ini_filename(None);
        match textures::load(render_context) {
            Ok(loaded) => self.textures = Some(loaded),
            Err(e) => logger::error(&format!("Overlay: textures failed ({e}), flat bars")),
        }
        logger::log("Overlay: ImGui initialized");
    }

    fn before_render<'a>(
        &'a mut self,
        ctx: &mut Context,
        render_context: &'a mut dyn RenderContext,
    ) {
        let [w, h] = ctx.io().display_size;
        self.area = game_area(w, h);
        self.s = self.area[3] / BASE_HEIGHT * settings::scale();
        // Every text of the overlay sets its own size (`text.rs`), so the target's `Scale` does not
        // resize the minimap's letters and clock.
        ctx.io_mut().font_global_scale = (self.area[3] / BASE_HEIGHT * FONT_SCALE).max(0.85);
        // hudhook uploads on the game's queue and waits for the GPU; on loading screens that
        // queue can be waiting on the game, so only upload in normal play.
        if self.radar.in_control() {
            // Started only now, not at launch: reading the archives takes a few hundred MB and some
            // seconds of disk and CPU, which the game does not need to compete with while it starts.
            if self.map.is_none() {
                self.map = std::env::current_exe()
                    .ok()
                    .and_then(|exe| exe.parent().map(MapLayer::new));
            }
            if let Some(map) = self.map.as_mut() {
                map.upload(render_context);
            }
        }
    }

    fn render(&mut self, ui: &mut Ui) {
        if settings::minimap_enabled() {
            for (keys, zoom_in) in [
                (settings::zoom_in_keys(), true),
                (settings::zoom_out_keys(), false),
            ] {
                if keys
                    .into_iter()
                    .any(|key| key != 0 && input::is_key_pressed(key))
                {
                    settings::zoom(zoom_in);
                }
            }
        }
        let snapshot: Option<RadarSnapshot> = radar::current();
        let show_map = settings::minimap_map();
        if let (Some(map), Some(snapshot), true) = (self.map.as_mut(), snapshot.as_ref(), show_map)
        {
            if let Some(id) = snapshot.map {
                // The window must stay ahead of the widest view, within what it can hold.
                map.follow(
                    id,
                    snapshot.position,
                    settings::minimap_range() + 20.0,
                    snapshot.fragments,
                );
            }
        }
        self.radar.draw(
            ui,
            snapshot.as_ref(),
            &Placement { area: self.area },
            self.map.as_ref(),
            show_map,
        );
        let dt = ui.io().delta_time;
        let wanted = if radar::in_control() { 1.0 } else { 0.0 };
        self.panel_alpha += (wanted - self.panel_alpha) * (dt * FADE_RATE).min(1.0);
        PANEL_ALPHA.store(self.panel_alpha.to_bits(), Ordering::Relaxed);
        let icons_wanted = if crate::state_icons::shown() {
            1.0
        } else {
            0.0
        };
        self.icons_alpha += (icons_wanted - self.icons_alpha) * (dt * FADE_RATE).min(1.0);
        let layout = Layout {
            area: self.area,
            offset: settings::offset(),
            width: settings::width(),
            s: self.s,
            show_numbers: settings::show_numbers(),
            textures: self.textures.as_ref(),
            icons: self.map.as_ref(),
        };
        if self.panel_alpha > 0.01 && settings::effects_player() {
            draw_icon_times(ui, &target::own_effects_now(), self.icons_alpha, &layout);
        }
        let Some(target) = target::current() else {
            self.trail.id = 0;
            self.poise_trail.id = 0;
            anchor_gauges();
            return;
        };
        self.trail
            .update(target.id, target.hp as f32, target.max_hp as f32, dt);
        match target.poise {
            Some((poise, max)) => self.poise_trail.update(target.id, poise, max, dt),
            None => self.poise_trail.id = 0,
        }
        if self.panel_alpha > 0.01 {
            draw_target(ui, &target, [&self.trail, &self.poise_trail], &layout);
        }
    }
}

/// The largest 16:9 rectangle centered in a `w` x `h` display: `[x, y, w, h]`.
fn game_area(w: f32, h: f32) -> [f32; 4] {
    if w / h > ASPECT {
        let aw = h * ASPECT;
        [(w - aw) / 2.0, 0.0, aw, h]
    } else {
        let ah = w / ASPECT;
        [0.0, (h - ah) / 2.0, w, ah]
    }
}

struct Layout<'a> {
    area: [f32; 4],
    offset: [f32; 2],
    width: f32,
    s: f32,
    show_numbers: bool,
    textures: Option<&'a Textures>,
    /// The game's icons, for the buildup bars.
    icons: Option<&'a MapLayer>,
}

/// What a bar is drawn with: where it is, how full, and how.
struct Bar<'a> {
    from: [f32; 2],
    to: [f32; 2],
    fraction: f32,
    /// Where the block of just-lost health ends, as a fraction.
    trail: Option<f32>,
    /// The color of that block: tinted over the bar's texture, and flat when there is none.
    trail_tint: [f32; 4],
    trail_color: [f32; 4],
    /// 0 to 1: brightness of the glow at the end of the health.
    glow: f32,
    core: [f32; 4],
    edge: [f32; 4],
    s: f32,
    textures: Option<&'a Textures>,
}

fn draw_target(ui: &Ui, target: &Target, [trail, poise_trail]: [&Trail; 2], layout: &Layout) {
    let s = layout.s;
    // The column's offsets follow the screen only; `s` also holds the player's
    // size setting, which resizes the bar, not where it sits.
    let res = layout.area[3] / BASE_HEIGHT;
    let right = layout.area[0] + layout.area[2] - layout.offset[0] * res;
    let left = right - layout.width * s;
    let y = layout.area[1] + layout.offset[1] * res + radar_ui::column_height(res);

    let draw = ui.get_foreground_draw_list();
    let label = format!("{} / {}", target.hp, target.max_hp);

    let top = y;
    let max_hp = target.max_hp as f32;
    let trail_fraction = (trail.from > target.hp as f32).then(|| trail.from / max_hp);
    let hp_bottom = y + HP_HEIGHT * s;
    let hp_bar = Bar {
        from: [left, y],
        to: [right, hp_bottom],
        fraction: target.hp as f32 / max_hp,
        trail: trail_fraction,
        trail_tint: TRAIL_TINT,
        trail_color: TRAIL_COLOR,
        glow: trail.glow(),
        core: HP_CORE,
        edge: HP_EDGE,
        s,
        textures: layout.textures,
    };
    draw_bar(&draw, &hp_bar);
    if layout.show_numbers {
        draw_bar_number(ui, &label, [left, right, y, hp_bottom], BAR_NUMBER * s, s);
    }

    let mut bottom = hp_bottom;
    let mut poise_top = None;
    if let Some((poise, max)) = target.poise {
        let poise_from = hp_bottom + BAR_GAP * s;
        poise_top = Some(poise_from);
        bottom = poise_from + POISE_HEIGHT * s;
        let poise_bar = Bar {
            from: [left, poise_from],
            to: [right, bottom],
            fraction: poise / max,
            trail: (poise_trail.from > poise).then(|| poise_trail.from / max),
            trail_tint: TRAIL_TINT,
            trail_color: TRAIL_COLOR,
            glow: poise_trail.glow(),
            core: POISE_CORE,
            edge: POISE_EDGE,
            s,
            textures: layout.textures,
        };
        draw_bar(&draw, &poise_bar);
        if layout.show_numbers {
            let label = format!("{poise:.0} / {max:.0}");
            draw_bar_number(
                ui,
                &label,
                [left, right, poise_from, bottom],
                BAR_NUMBER * s,
                s,
            );
        }
    }
    // A frame for each bar, drawn over both so one bar's does not hide the other's.
    frame(&draw, left, right, top, hp_bottom, s, layout.textures);
    if let Some(poise_from) = poise_top {
        frame(&draw, left, right, poise_from, bottom, s, layout.textures);
    }

    // Under the tan line.
    let mut row_top = bottom + (LINE + BUILDUP_GAP) * s;
    for buildup in &target.buildup {
        draw_buildup(ui, &draw, buildup, [left, row_top, right], layout);
        row_top += BUILDUP_ROW * s;
    }
    if settings::effects_target() && !target.effects.is_empty() {
        let size = s * TARGET_EFFECTS_SCALE;
        let per_row = (((right - left) / (EFFECT_CELL * size)).floor() as usize)
            .clamp(1, PLAYER_EFFECTS_PER_ROW);
        let cells = Cells {
            origin: [right, row_top + EFFECTS_GAP * s],
            right_aligned: true,
            s: size,
            text: EFFECT_TIME_TARGET * s,
            per_row,
        };
        draw_effects(ui, &draw, &target.effects, cells, layout);
    }
    anchor_gauges();
}

/// Tells the game's buildup gauge of the player where it goes: at the left, under the game's row of
/// status icons and their times, at the game's own size. Far from the target's panel, so
/// the player's buildup is not taken for the enemy's.
fn anchor_gauges() {
    // The gauge has its own size setting, not the target panel's, as a fraction of the game's own
    // (the game scales it with the resolution itself).
    let scale = crate::settings::player_scale();
    let left = GAME_ICONS_CENTER_X - GAME_ICONS_PITCH / 2.0;
    // Under the last row the game's icons fill.
    let used = (0..crate::state_icons::SLOTS).take_while(|&slot| crate::state_icons::icon(slot).is_some()).count();
    let last_row = used.saturating_sub(1) / GAME_ICONS_PER_ROW;
    let top = GAME_ICONS_BOTTOM + last_row as f32 * GAME_ICONS_ROW_PITCH + GAME_ICON_TIME * 1.5 + GAUGE_UNDER_ICONS_GAP;
    crate::state_icons::place_gauges(left + crate::state_icons::GAUGE_WIDTH * scale, top, scale);
}

/// The time left as `24s`, or `1:30` from a minute on.
fn effect_time(left: f32) -> String {
    let seconds = left.max(0.0).ceil() as u32;
    if seconds >= 60 {
        format!("{}:{:02}", seconds / 60, seconds % 60)
    } else {
        format!("{seconds}s")
    }
}

/// The time left under each icon of the game's own row (the effects of the player, one icon per
/// slot): for a slot's icon, the effect with that icon that lasts longest (the "Cycled" helper rows
/// of a buff have a tenth of a second and are left out of the list); an effect without an end
/// shows nothing.
fn draw_icon_times(ui: &Ui, effects: &[Effect], alpha: f32, layout: &Layout) {
    let res = layout.area[3] / BASE_HEIGHT;
    let em = GAME_ICON_TIME * res;
    let centered = |slot: usize| {
        layout.area[0] + (GAME_ICONS_CENTER_X + (slot % GAME_ICONS_PER_ROW) as f32 * GAME_ICONS_PITCH) * res
    };
    let time_below = |label: &str, center: f32, row: usize, alpha: f32| {
        let size = text::measure(ui, label, em);
        let pos = [
            (center - size[0] / 2.0).round(),
            (layout.area[1] + (GAME_ICONS_BOTTOM + row as f32 * GAME_ICONS_ROW_PITCH) * res).round(),
        ];
        text::add_shadowed(
            label,
            pos,
            em,
            fade(fade_by(TEXT, alpha)),
            fade(fade_by(TEXT_SHADOW, alpha)),
            res.max(1.0),
        );
    };
    for slot in 0..crate::state_icons::SLOTS {
        let Some(icon) = crate::state_icons::icon(slot) else {
            break;
        };
        let left = effects
            .iter()
            .filter(|e| e.kind.is_none() && e.icon == icon && !e.permanent)
            .map(|e| e.left)
            .fold(f32::NEG_INFINITY, f32::max);
        if left > 0.0 && alpha > 0.01 {
            time_below(&effect_time(left), centered(slot), slot / GAME_ICONS_PER_ROW, alpha);
        }
    }
}

/// Where the cells of a row of effects start and how big they are.
struct Cells {
    /// The right edge of the first cell when `right_aligned`, the left edge otherwise; and the top.
    origin: [f32; 2],
    right_aligned: bool,
    /// The size the cells are drawn at.
    s: f32,
    /// The font size of the time, in pixels.
    text: f32,
    /// Cells in a row; at most [EFFECT_ROWS] rows are drawn.
    per_row: usize,
}

/// The effects as cells, `per_row` to a row, from the right edge leftwards or from the left
/// edge rightwards.
fn draw_effects(
    ui: &Ui,
    draw: &DrawListMut<'_>,
    effects: &[Effect],
    cells: Cells,
    layout: &Layout,
) {
    let (s, [edge, top]) = (cells.s, cells.origin);
    let shown = target::shown_effects(effects.to_vec(), cells.per_row * EFFECT_ROWS);
    for (i, effect) in shown.iter().enumerate() {
        let (row, column) = (i / cells.per_row, i % cells.per_row);
        let (x0, x1) = if cells.right_aligned {
            let x1 = edge - column as f32 * EFFECT_CELL * s;
            (x1 - EFFECT_CELL * s, x1)
        } else {
            let x0 = edge + column as f32 * EFFECT_CELL * s;
            (x0, x0 + EFFECT_CELL * s)
        };
        let y = top + row as f32 * EFFECT_ROW * s;
        let alpha = if effect.permanent {
            1.0
        } else {
            (0.4 + 0.6 * effect.left / EFFECT_FADE_SECONDS).clamp(0.4, 1.0)
        };
        let height = EFFECT_ICON * s;
        let center = (x0 + x1) / 2.0;
        let icon_top = y + (2.0 * s).round();
        if let Some(icon) = effect
            .kind
            .and_then(Icon::buildup)
            .and_then(|icon| layout.icons?.icon(icon))
        {
            let width = (height * icon.size[0] / icon.size[1]).min(EFFECT_CELL * s);
            draw.add_image(
                icon.texture,
                [center - width / 2.0, icon_top],
                [center + width / 2.0, icon_top + height],
            )
            .col(fade([1.0, 1.0, 1.0, alpha]))
            .build();
        } else if let Some((texture, cell)) =
            layout.icons.and_then(|map| map.status_icon(effect.icon))
        {
            draw.add_image(
                texture,
                [center - height / 2.0, icon_top],
                [center + height / 2.0, icon_top + height],
            )
            .uv_min(cell.uv_min)
            .uv_max(cell.uv_max)
            .col(fade([1.0, 1.0, 1.0, alpha]))
            .build();
        }
        if !effect.permanent {
            let label = effect_time(effect.left);
            let em = cells.text;
            let size = text::measure(ui, &label, em);
            let pos = [
                (center - size[0] / 2.0).round(),
                (icon_top + height + 1.0 * s).round(),
            ];
            text::add_shadowed(&label, pos, em, fade(TEXT), fade(TEXT_SHADOW), s.max(1.0));
        }
    }
}

/// One buildup bar: the game's icon of the status, then a bar filled with what has built up, with
/// `built / max` in it when the numbers are shown.
fn draw_buildup(
    ui: &Ui,
    draw: &DrawListMut<'_>,
    buildup: &Buildup,
    [left, top, right]: [f32; 3],
    layout: &Layout,
) {
    let s = layout.s;
    let middle = top + BUILDUP_ROW * s / 2.0;
    let mut bar_left = left;
    if let Some(icon) = layout
        .icons
        .and_then(|map| Icon::buildup(buildup.kind).and_then(|icon| map.icon(icon)))
    {
        let height = BUILDUP_ICON * s;
        let width = height * icon.size[0] / icon.size[1];
        draw.add_image(
            icon.texture,
            [left, middle - height / 2.0],
            [left + width, middle + height / 2.0],
        )
        .col(fade([1.0, 1.0, 1.0, 1.0]))
        .build();
        bar_left = left + width + BUILDUP_ICON_GAP * s;
    }
    let color = BUILDUP_COLORS[buildup.kind];
    let half = BUILDUP_BAR * s / 2.0;
    let bar = Bar {
        from: [bar_left, middle - half],
        to: [right, middle + half],
        fraction: buildup.built as f32 / buildup.max as f32,
        trail: None,
        trail_tint: TRAIL_TINT,
        trail_color: TRAIL_COLOR,
        glow: 0.0,
        core: color,
        edge: color,
        s,
        textures: layout.textures,
    };
    draw_bar(draw, &bar);
    if layout.show_numbers {
        let label = format!("{} / {}", buildup.built, buildup.max);
        draw_bar_number(
            ui,
            &label,
            [bar_left, right, middle - half, middle + half],
            BAR_NUMBER * s,
            s,
        );
    }
}

/// `text` centered in the bar `[left, right, top, bottom]`, set at a font size of `em` pixels so it
/// stays inside the bar, with a shadow so it reads over both the bar and the track.
fn draw_bar_number(ui: &Ui, text: &str, [left, right, top, bottom]: [f32; 4], em: f32, s: f32) {
    let size = text::measure(ui, text, em);
    let pos = [
        ((left + right - size[0]) / 2.0).round(),
        ((top + bottom - size[1]) / 2.0).round(),
    ];
    text::add_shadowed(text, pos, em, fade(TEXT), fade(TEXT_SHADOW), s.max(1.0));
}

fn draw_bar(draw: &DrawListMut<'_>, bar: &Bar<'_>) {
    draw.add_rect(bar.from, bar.to, fade(TRACK))
        .filled(true)
        .build();
    match bar.textures {
        Some(textures) => textured_bar(draw, bar, textures),
        None => flat_bar(draw, bar),
    }
}

/// The bar from the game's own textures: grain background, the bar texture
/// tinted and cropped to the health, a tinted block for the health just lost,
/// and the game's divider and glow at the end of the health.
fn textured_bar(draw: &DrawListMut<'_>, bar: &Bar<'_>, textures: &Textures) {
    let (from, to) = (bar.from, bar.to);
    let width = to[0] - from[0];
    // The textures have a transparent row above and below the visible bar.
    let unit = (to[1] - from[1]) / textures::BAR_ROWS;
    let (top, bottom) = (from[1] - unit, to[1] + unit);
    let tint = |c: [f32; 4]| {
        fade([
            (c[0] / TEXTURE_LEVEL).min(1.0),
            (c[1] / TEXTURE_LEVEL).min(1.0),
            (c[2] / TEXTURE_LEVEL).min(1.0),
            1.0,
        ])
    };

    draw.add_image(textures.background, [from[0], top], [to[0], bottom])
        .col(fade(BACKGROUND_TINT))
        .build();

    let fraction = bar.fraction.clamp(0.0, 1.0);
    let end = from[0] + width * fraction;
    if let Some(trail) = bar.trail {
        let trail = trail.clamp(fraction, 1.0);
        draw.add_image(textures.bar, [end, top], [from[0] + width * trail, bottom])
            .uv_min([fraction, 0.0])
            .uv_max([trail, 1.0])
            .col(fade(bar.trail_tint))
            .build();
    }
    if fraction > 0.0 {
        draw.add_image(textures.bar, [from[0], top], [end, bottom])
            .uv_max([fraction, 1.0])
            .col(tint(bar.core))
            .build();
    }
    if bar.trail.is_some() && fraction > 0.0 {
        let middle = (from[1] + to[1]) / 2.0;
        let [edge_w, edge_h] = textures::EDGE_SIZE.map(|v| v * unit);
        draw.add_image(
            textures.edge,
            [end - edge_w / 2.0, middle - edge_h / 2.0],
            [end + edge_w / 2.0, middle + edge_h / 2.0],
        )
        .col(fade([1.0, 1.0, 1.0, 1.0]))
        .build();
        if bar.glow > 0.0 {
            let [glow_w, glow_h] = textures::GLOW_SIZE.map(|v| v * unit);
            let right = end + textures::GLOW_TAIL * unit;
            draw.add_image(
                textures.glow,
                [right - glow_w, middle - glow_h / 2.0],
                [right, middle + glow_h / 2.0],
            )
            .col(fade([1.0, 1.0, 1.0, bar.glow]))
            .build();
        }
    }
}

/// Fallback when the textures could not be loaded: flat colors.
fn flat_bar(draw: &DrawListMut<'_>, bar: &Bar<'_>) {
    let (from, to) = (bar.from, bar.to);
    let width = to[0] - from[0];
    let fraction = bar.fraction.clamp(0.0, 1.0);
    if let Some(trail) = bar.trail {
        let trail = trail.clamp(fraction, 1.0);
        draw.add_rect(
            [from[0] + width * fraction, from[1]],
            [from[0] + width * trail, to[1]],
            fade(bar.trail_color),
        )
        .filled(true)
        .build();
    }
    if fraction > 0.0 {
        let end = from[0] + width * fraction;
        let fade = from[1] + (to[1] - from[1]) * 0.8;
        let (core, edge) = (self::fade(bar.core), self::fade(bar.edge));
        draw.add_rect(from, [end, fade], core).filled(true).build();
        draw.add_rect_filled_multicolor([from[0], fade], [end, to[1]], core, core, edge, edge);
        if bar.trail.is_some() {
            let line = bar.s.max(1.0);
            draw.add_rect([end - line, from[1]], [end, to[1]], self::fade(DIVIDER))
                .filled(true)
                .build();
        }
    }
}

/// The frame around the bars: the game's box for its buildup gauge (a thin line above and below,
/// a cap at each end) from its own textures, or drawn shapes (the tan line under the bars and a
/// small ornament at each end) when they could not be loaded.
fn frame(
    draw: &DrawListMut<'_>,
    left: f32,
    right: f32,
    top: f32,
    bottom: f32,
    s: f32,
    textures: Option<&Textures>,
) {
    if let Some(textures) = textures {
        textured_frame(draw, [left, right, top, bottom], s, textures);
        return;
    }
    let line = LINE * s.max(0.85);
    let under = bottom + line;
    let color = fade(LINE_COLOR);
    draw.add_rect([left, bottom], [right, under], color)
        .filled(true)
        .build();
    draw.add_triangle(
        [left - 3.0 * s, under],
        [left + 2.0 * s, under],
        [left + 1.5 * s, top - 1.5 * s],
        color,
    )
    .filled(true)
    .build();
    draw.add_triangle(
        [right - 2.0 * s, under],
        [right + 3.0 * s, under],
        [right + 0.5 * s, bottom - 3.0 * s],
        color,
    )
    .filled(true)
    .build();
}

/// `frame` from the game's box. Its lines are `LINE_TEXEL` thick per texel of the texture and sit
/// on the top and the bottom of the bars; the caps are stretched so they span from one line to the
/// other. The texture's texels are `s / 2` wide, as the bars' are.
fn textured_frame(
    draw: &DrawListMut<'_>,
    [left, right, top, bottom]: [f32; 4],
    s: f32,
    textures: &Textures,
) {
    let white = fade([1.0, 1.0, 1.0, 1.0]);
    let [width, height] = textures::WAKU_SIZE;
    let kx = 0.5 * s;
    let thick = WAKU_LINE_TEXEL * s;
    for ([from, to, center], y) in [
        (textures::WAKU_TOP_LINE, top),
        (textures::WAKU_BOTTOM_LINE, bottom),
    ] {
        draw.add_image(
            textures.waku,
            [left, y - (center - from) * thick],
            [right, y + (to - center) * thick],
        )
        .uv_min([0.1, from / height])
        .uv_max([0.1 + (right - left) / kx / width, to / height])
        .col(white)
        .build();
    }
    // Caps: the art spans the lines, 28.5 texels of the texture apart.
    let between = textures::WAKU_BOTTOM_LINE[2] - textures::WAKU_TOP_LINE[2];
    let ky = (bottom - top) / between;
    let y0 = top - textures::WAKU_TOP_LINE[2] * ky;
    let y1 = y0 + height * ky;
    let [left_w, _] = textures::WAKU_LEFT_SIZE;
    let x1 = left + 1.0 * s;
    draw.add_image(textures.waku_left, [x1 - left_w * kx, y0], [x1, y1])
        .col(white)
        .build();
    let from = textures::WAKU_CAP_FROM;
    let x0 = right + 1.0 * s - (textures::WAKU_CAP_ART_END - from) * kx;
    draw.add_image(textures.waku, [x0, y0], [x0 + (width - from) * kx, y1])
        .uv_min([from / width, 0.0])
        .uv_max([1.0, 1.0])
        .col(white)
        .build();
}

pub fn install() {
    match Hudhook::builder()
        .with::<ImguiDx12Hooks>(HudRenderLoop::default())
        .build()
        .apply()
    {
        Ok(()) => logger::log("Overlay: DX12 hooks applied"),
        Err(e) => logger::error(&format!(
            "Overlay: DX12 hooks failed ({e:?}), overlay unavailable"
        )),
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn effect_time_counts_whole_seconds_and_switches_to_minutes() {
        assert_eq!(effect_time(24.3), "25s");
        assert_eq!(effect_time(9.0), "9s");
        assert_eq!(effect_time(59.2), "1:00");
        assert_eq!(effect_time(90.0), "1:30");
        assert_eq!(effect_time(-1.0), "0s");
    }

    use super::{Trail, effect_time, game_area};

    #[test]
    fn area_is_the_16_9_rect_centered_in_the_display() {
        // 16:10 (2560x1600): bars above and below, as seen in game.
        assert_eq!(game_area(2560.0, 1600.0), [0.0, 80.0, 2560.0, 1440.0]);
        // 16:9: the whole display.
        assert_eq!(game_area(1920.0, 1080.0), [0.0, 0.0, 1920.0, 1080.0]);
        // 21:9 (2560x1080): bars left and right.
        let a = game_area(2560.0, 1080.0);
        assert_eq!((a[1], a[3]), (0.0, 1080.0));
        assert!((a[2] - 1920.0).abs() < 0.01 && (a[0] - 320.0).abs() < 0.01);
    }

    /// One step of a trail over a bar of 100 with `value` in it.
    fn step(trail: &mut Trail, id: usize, value: i32, dt: f32) {
        trail.update(id, value as f32, 100.0, dt);
    }

    #[test]
    fn trail_holds_after_damage_then_drains_to_the_current_health() {
        let mut trail = Trail::default();
        step(&mut trail, 1, 100, 0.016);
        assert_eq!(trail.from, 100.0);

        // Lose 30: the block starts where health was and stays for the hold time.
        step(&mut trail, 1, 70, 0.016);
        assert_eq!(trail.from, 100.0);
        step(&mut trail, 1, 70, 0.5);
        assert_eq!(trail.from, 100.0);

        // Past the hold it drains, never below the current health.
        step(&mut trail, 1, 70, 0.5);
        step(&mut trail, 1, 70, 0.2);
        assert!(trail.from < 100.0 && trail.from >= 70.0);
        step(&mut trail, 1, 70, 2.0);
        assert_eq!(trail.from, 70.0);
    }

    #[test]
    fn trail_resets_on_another_enemy_and_follows_healing() {
        let mut trail = Trail::default();
        step(&mut trail, 1, 100, 0.016);
        step(&mut trail, 1, 50, 0.016);
        // A different enemy starts with no block.
        step(&mut trail, 2, 80, 0.016);
        assert_eq!(trail.from, 80.0);
        // Healing moves the start up with the health.
        step(&mut trail, 2, 90, 0.016);
        assert_eq!(trail.from, 90.0);
    }
}
