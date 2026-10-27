//! Overlay: Dear ImGui over the game's DX12 frames via hudhook. Draws the
//! target panel on the foreground draw list (no window, so it never takes
//! input or focus) from the snapshot [crate::target] keeps up to date.
//! Details: `docs/design.md`.

use hudhook::hooks::dx12::ImguiDx12Hooks;
use hudhook::imgui::{Context, DrawListMut, Ui};
use hudhook::{Hudhook, ImguiRenderLoop, RenderContext};

use common::logger;

use crate::radar::{self, Snapshot as RadarSnapshot};
use crate::radar_ui::{self, Placement, Radar};
use crate::settings;
use crate::target::{self, Target};
use crate::textures::{self, Textures};

/// The game draws its HUD in a 16:9 area fitted into the window (black bars on
/// the other axis), authored for 1080p. Everything here is laid out in that
/// area, in 1080p pixels, and scaled by its real height.
const ASPECT: f32 = 16.0 / 9.0;
const BASE_HEIGHT: f32 = 1080.0;

/// The thickness matches the player's own health bar (about 12 at 1080p); the
/// length is a setting.
const HP_HEIGHT: f32 = 12.0;
/// Flush under the HP bar, no gap.
const POISE_HEIGHT: f32 = 6.0;
/// Thickness of the tan line under the bars.
const LINE: f32 = 2.0;
/// Height of the row above the HP bar that holds the numbers, when shown.
const TEXT_ROW: f32 = 22.0;
/// ImGui's default font is 13px; this makes the text about 17px at 1080p.
const FONT_SCALE: f32 = 1.3;

/// Seconds the lost-health block stays before it drains, and how fast it
/// drains afterwards (fraction of max health per second).
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
const POISE_CORE: [f32; 4] = [0.52, 0.41, 0.11, 1.0];
const POISE_EDGE: [f32; 4] = [0.62, 0.52, 0.24, 1.0];
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
    radar: Radar,
    /// `None` when the textures failed to load: the bars are drawn flat.
    textures: Option<Textures>,
}

/// The health that was just lost, shown as a block between the current health
/// and where it was, like the game's own bars.
#[derive(Default)]
struct Trail {
    id: usize,
    hp: i32,
    /// Where the block starts (health points), `<= hp` when there is none.
    from: f32,
    hold: f32,
}

impl Trail {
    /// 1 right after damage, fading to 0 as the block starts to drain.
    fn glow(&self) -> f32 {
        (self.hold / TRAIL_HOLD).clamp(0.0, 1.0)
    }

    /// Advances to the current `target`; `dt` is the frame time in seconds.
    fn update(&mut self, target: &Target, dt: f32) {
        if self.id != target.id {
            *self = Trail { id: target.id, hp: target.hp, from: target.hp as f32, hold: 0.0 };
            return;
        }
        if target.hp < self.hp {
            self.from = self.from.max(self.hp as f32);
            self.hold = TRAIL_HOLD;
        } else if target.hp > self.hp {
            self.from = target.hp as f32;
        }
        self.hp = target.hp;
        if self.hold > 0.0 {
            self.hold -= dt;
        } else {
            self.from -= TRAIL_DRAIN * target.max_hp as f32 * dt;
        }
        self.from = self.from.clamp(target.hp as f32, target.max_hp as f32);
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

    fn before_render<'a>(&'a mut self, ctx: &mut Context, _render_context: &'a mut dyn RenderContext) {
        let [w, h] = ctx.io().display_size;
        self.area = game_area(w, h);
        self.s = self.area[3] / BASE_HEIGHT * settings::scale();
        // Not below 0.85 so the text stays readable in a small window.
        ctx.io_mut().font_global_scale = (self.s * FONT_SCALE).max(0.85);
    }

    fn render(&mut self, ui: &mut Ui) {
        let snapshot: Option<RadarSnapshot> = radar::current();
        self.radar.draw(ui, snapshot.as_ref(), &Placement { area: self.area });
        let Some(target) = target::current() else {
            self.trail.id = 0;
            return;
        };
        self.trail.update(&target, ui.io().delta_time);
        let layout = Layout {
            area: self.area,
            offset: settings::offset(),
            width: settings::width(),
            s: self.s,
            show_numbers: settings::show_numbers(),
            textures: self.textures.as_ref(),
        };
        draw_target(ui, &target, &self.trail, &layout);
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
}

/// What a bar is drawn with: where it is, how full, and how.
struct Bar<'a> {
    from: [f32; 2],
    to: [f32; 2],
    fraction: f32,
    /// Where the block of just-lost health ends, as a fraction.
    trail: Option<f32>,
    /// 0 to 1: brightness of the glow at the end of the health.
    glow: f32,
    core: [f32; 4],
    edge: [f32; 4],
    s: f32,
    textures: Option<&'a Textures>,
}

fn draw_target(ui: &Ui, target: &Target, trail: &Trail, layout: &Layout) {
    let s = layout.s;
    // The column's offsets follow the screen only; `s` also holds the player's
    // size setting, which resizes the bar, not where it sits.
    let res = layout.area[3] / BASE_HEIGHT;
    let right = layout.area[0] + layout.area[2] - layout.offset[0] * res;
    let left = right - layout.width * s;
    let mut y = layout.area[1] + layout.offset[1] * res + radar_ui::column_height(res);

    let draw = ui.get_foreground_draw_list();
    if layout.show_numbers {
        // Right-aligned above the bar, like the number on the game's boss bar.
        let label = format!("{} / {}", target.hp, target.max_hp);
        let size = ui.calc_text_size(&label);
        let pos = [right - size[0], y + (TEXT_ROW * s - size[1]) / 2.0];
        let shadow = s.max(1.0);
        draw.add_text([pos[0] + shadow, pos[1] + shadow], TEXT_SHADOW, &label);
        draw.add_text(pos, TEXT, label);
        y += TEXT_ROW * s;
    }

    let top = y;
    let max_hp = target.max_hp as f32;
    let trail_fraction = (trail.from > target.hp as f32).then(|| trail.from / max_hp);
    let hp_bottom = y + HP_HEIGHT * s;
    let hp_bar = Bar {
        from: [left, y],
        to: [right, hp_bottom],
        fraction: target.hp as f32 / max_hp,
        trail: trail_fraction,
        glow: trail.glow(),
        core: HP_CORE,
        edge: HP_EDGE,
        s,
        textures: layout.textures,
    };
    draw_bar(&draw, &hp_bar);

    let mut bottom = hp_bottom;
    if let Some((poise, max)) = target.poise {
        bottom += POISE_HEIGHT * s;
        let poise_bar = Bar {
            from: [left, hp_bottom],
            to: [right, bottom],
            fraction: poise / max,
            trail: None,
            glow: 0.0,
            core: POISE_CORE,
            edge: POISE_EDGE,
            s,
            textures: layout.textures,
        };
        draw_bar(&draw, &poise_bar);
    }
    frame(&draw, left, right, top, bottom, s);
}

fn draw_bar(draw: &DrawListMut<'_>, bar: &Bar<'_>) {
    draw.add_rect(bar.from, bar.to, TRACK).filled(true).build();
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
    let tint = |c: [f32; 4]| [(c[0] / TEXTURE_LEVEL).min(1.0), (c[1] / TEXTURE_LEVEL).min(1.0), (c[2] / TEXTURE_LEVEL).min(1.0), 1.0];

    draw.add_image(textures.background, [from[0], top], [to[0], bottom]).col(BACKGROUND_TINT).build();

    let fraction = bar.fraction.clamp(0.0, 1.0);
    let end = from[0] + width * fraction;
    if let Some(trail) = bar.trail {
        let trail = trail.clamp(fraction, 1.0);
        draw.add_image(textures.bar, [end, top], [from[0] + width * trail, bottom])
            .uv_min([fraction, 0.0])
            .uv_max([trail, 1.0])
            .col(TRAIL_TINT)
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
        draw.add_image(textures.edge, [end - edge_w / 2.0, middle - edge_h / 2.0], [end + edge_w / 2.0, middle + edge_h / 2.0])
            .build();
        if bar.glow > 0.0 {
            let [glow_w, glow_h] = textures::GLOW_SIZE.map(|v| v * unit);
            let right = end + textures::GLOW_TAIL * unit;
            draw.add_image(textures.glow, [right - glow_w, middle - glow_h / 2.0], [right, middle + glow_h / 2.0])
                .col([1.0, 1.0, 1.0, bar.glow])
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
        draw.add_rect([from[0] + width * fraction, from[1]], [from[0] + width * trail, to[1]], TRAIL_COLOR)
            .filled(true)
            .build();
    }
    if fraction > 0.0 {
        let end = from[0] + width * fraction;
        let fade = from[1] + (to[1] - from[1]) * 0.8;
        draw.add_rect(from, [end, fade], bar.core).filled(true).build();
        draw.add_rect_filled_multicolor([from[0], fade], [end, to[1]], bar.core, bar.core, bar.edge, bar.edge);
        if bar.trail.is_some() {
            let line = bar.s.max(1.0);
            draw.add_rect([end - line, from[1]], [end, to[1]], DIVIDER).filled(true).build();
        }
    }
}

/// The tan line under the bars and a small ornament at each end.
fn frame(draw: &DrawListMut<'_>, left: f32, right: f32, top: f32, bottom: f32, s: f32) {
    let line = LINE * s.max(0.85);
    let under = bottom + line;
    draw.add_rect([left, bottom], [right, under], LINE_COLOR).filled(true).build();
    draw.add_triangle([left - 3.0 * s, under], [left + 2.0 * s, under], [left + 1.5 * s, top - 1.5 * s], LINE_COLOR)
        .filled(true)
        .build();
    draw.add_triangle([right - 2.0 * s, under], [right + 3.0 * s, under], [right + 0.5 * s, bottom - 3.0 * s], LINE_COLOR)
        .filled(true)
        .build();
}

pub fn install() {
    match Hudhook::builder().with::<ImguiDx12Hooks>(HudRenderLoop::default()).build().apply() {
        Ok(()) => logger::log("Overlay: DX12 hooks applied"),
        Err(e) => logger::error(&format!("Overlay: DX12 hooks failed ({e:?}), overlay unavailable")),
    }
}

#[cfg(test)]
mod tests {
    use super::{Target, Trail, game_area};

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

    fn target(id: usize, hp: i32) -> Target {
        Target { id, npc_param_id: 1, offset: [0.0, 0.0], hp, max_hp: 100, poise: None }
    }

    #[test]
    fn trail_holds_after_damage_then_drains_to_the_current_health() {
        let mut trail = Trail::default();
        trail.update(&target(1, 100), 0.016);
        assert_eq!(trail.from, 100.0);

        // Lose 30: the block starts where health was and stays for the hold time.
        trail.update(&target(1, 70), 0.016);
        assert_eq!(trail.from, 100.0);
        trail.update(&target(1, 70), 0.5);
        assert_eq!(trail.from, 100.0);

        // Past the hold it drains, never below the current health.
        trail.update(&target(1, 70), 0.5);
        trail.update(&target(1, 70), 0.2);
        assert!(trail.from < 100.0 && trail.from >= 70.0);
        trail.update(&target(1, 70), 2.0);
        assert_eq!(trail.from, 70.0);
    }

    #[test]
    fn trail_resets_on_another_enemy_and_follows_healing() {
        let mut trail = Trail::default();
        trail.update(&target(1, 100), 0.016);
        trail.update(&target(1, 50), 0.016);
        // A different enemy starts with no block.
        trail.update(&target(2, 80), 0.016);
        assert_eq!(trail.from, 80.0);
        // Healing moves the start up with the health.
        trail.update(&target(2, 90), 0.016);
        assert_eq!(trail.from, 90.0);
    }
}
