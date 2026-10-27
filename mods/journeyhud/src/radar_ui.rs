//! The radar drawn on the overlay: a round plate with the player's arrow in
//! the middle, turning with the camera (whatever the camera faces is up),
//! with the other players, the player's spirits and the locked-on enemy as
//! marks around it. Everything is drawn from primitives, no textures.
//! Details: `docs/radar.md`.

use hudhook::imgui::{DrawListMut, Ui};

use crate::radar::{Dot, Snapshot};
use crate::settings;

/// Seconds-based easing rates (per second) of the fade in and out and of the
/// arrow following the player's facing.
const FADE_RATE: f32 = 12.0;
const TURN_RATE: f32 = 14.0;

const PLATE: [f32; 4] = [0.02, 0.03, 0.03, 0.55];
const RING: [f32; 4] = [0.59, 0.56, 0.45, 1.0];
const GUIDE: [f32; 4] = [0.59, 0.56, 0.45, 0.22];
const PLAYER: [f32; 4] = [0.93, 0.89, 0.76, 1.0];
const PARTNER: [f32; 4] = [0.51, 0.78, 1.0, 1.0];
const SPIRIT: [f32; 4] = [0.55, 0.85, 0.55, 1.0];
const ENEMY: [f32; 4] = [0.78, 0.14, 0.12, 1.0];
const NORTH: [f32; 4] = [1.0, 0.87, 0.59, 1.0];
const COMPASS: [f32; 4] = [0.84, 0.77, 0.59, 0.9];
const OUTLINE: [f32; 4] = [0.0, 0.0, 0.0, 0.85];

/// Gap between the minimap and the target bar under it, at 1080p.
const COLUMN_GAP: f32 = 14.0;

/// Height the minimap takes at the top of the right-hand column, gap included,
/// for the target bar to sit under it. Follows the setting, not the fade, so
/// the bar does not jump when the minimap hides.
pub fn column_height(res: f32) -> f32 {
    if settings::minimap_enabled() { (2.0 * settings::minimap_size() + COLUMN_GAP) * res } else { 0.0 }
}

/// What the radar remembers between frames.
pub struct Radar {
    /// 0 to 1, follows whether the radar should be on screen.
    alpha: f32,
    /// The player's facing as an angle, eased so the arrow does not jump.
    yaw: Option<f32>,
    /// The camera's directions on the ground, last good ones.
    camera: ([f32; 2], [f32; 2]),
}

impl Default for Radar {
    fn default() -> Self {
        Radar { alpha: 0.0, yaw: None, camera: ([1.0, 0.0], [0.0, 1.0]) }
    }
}

/// Where the radar sits and how big it is on this screen.
pub struct Placement {
    /// `[x, y, width, height]` of the game's 16:9 picture.
    pub area: [f32; 4],
}

impl Radar {
    /// Draws the radar for `snapshot` (`None` = nothing to show, fades out).
    pub fn draw(&mut self, ui: &Ui, snapshot: Option<&Snapshot>, placement: &Placement) {
        let dt = ui.io().delta_time;
        let in_control = snapshot.is_some_and(|s| s.hud_visible && s.fade < 0.05);
        let target = if in_control { 1.0 } else { 0.0 };
        self.alpha += (target - self.alpha) * (dt * FADE_RATE).min(1.0);
        let Some(snapshot) = snapshot else { return };
        if self.alpha < 0.01 {
            return;
        }

        if let (Some(right), Some(forward)) = (normalize(snapshot.camera_right), normalize(snapshot.camera_forward)) {
            self.camera = (right, forward);
        }
        if let Some([x, z]) = normalize(snapshot.facing) {
            let wanted = x.atan2(z);
            let yaw = self.yaw.get_or_insert(wanted);
            *yaw += wrap_angle(wanted - *yaw) * (dt * TURN_RATE).min(1.0);
        }

        let s = placement.area[3] / 1080.0;
        let radius = settings::minimap_size() * s;
        let [offset_x, offset_y] = settings::offset();
        let center = [
            placement.area[0] + placement.area[2] - offset_x * s - radius,
            placement.area[1] + offset_y * s + radius,
        ];
        let draw = ui.get_foreground_draw_list();
        let view = View {
            radar: self,
            center,
            radius,
            s,
            // The marks do not shrink below 0.8 so they stay readable in a small window.
            m: s.max(0.8),
            pixels_per_meter: radius / settings::minimap_range(),
        };
        view.draw_plate(&draw);
        view.draw_marks(&draw, snapshot);
        view.draw_compass(ui, &draw);
        view.draw_player(&draw);
    }
}

/// One frame of the radar: the fixed geometry plus the camera basis.
struct View<'a> {
    radar: &'a Radar,
    center: [f32; 2],
    radius: f32,
    s: f32,
    /// Scale of the marks and arrows.
    m: f32,
    pixels_per_meter: f32,
}

impl View<'_> {
    fn color(&self, [r, g, b, a]: [f32; 4]) -> [f32; 4] {
        [r, g, b, a * self.radar.alpha]
    }

    /// Turns a horizontal world direction into a screen direction.
    fn to_screen(&self, [dx, dz]: [f32; 2]) -> [f32; 2] {
        let ([rx, rz], [fx, fz]) = self.radar.camera;
        [dx * rx + dz * rz, -(dx * fx + dz * fz)]
    }

    fn at(&self, direction: [f32; 2], distance: f32) -> [f32; 2] {
        [self.center[0] + direction[0] * distance, self.center[1] + direction[1] * distance]
    }

    fn draw_plate(&self, draw: &DrawListMut<'_>) {
        draw.add_circle(self.center, self.radius, self.color(PLATE)).filled(true).num_segments(96).build();
        for fraction in [1.0 / 3.0, 2.0 / 3.0] {
            draw.add_circle(self.center, self.radius * fraction, self.color(GUIDE)).num_segments(96).thickness(self.s.max(1.0)).build();
        }
        draw.add_circle(self.center, self.radius, self.color(RING)).num_segments(96).thickness(2.0 * self.s.max(0.8)).build();
    }

    fn draw_marks(&self, draw: &DrawListMut<'_>, snapshot: &Snapshot) {
        // Spirits first so a partner or the enemy sits on top of them.
        self.draw_dots(draw, &snapshot.spirits, SPIRIT, 4.5, false);
        self.draw_dots(draw, &snapshot.partners, PARTNER, 6.5, true);
        if let Some((at, inside)) = snapshot.target.and_then(|offset| self.place(offset, true)) {
            let alpha = if inside { 1.0 } else { 0.8 };
            let [r, g, b, _] = ENEMY;
            let radius = 5.0 * self.m;
            draw.add_circle(at, radius, self.color([r, g, b, alpha])).filled(true).build();
            draw.add_circle(at, radius, self.color([OUTLINE[0], OUTLINE[1], OUTLINE[2], OUTLINE[3] * alpha]))
                .thickness(self.m)
                .build();
        }
    }

    fn draw_dots(&self, draw: &DrawListMut<'_>, dots: &[Dot], color: [f32; 4], size: f32, pin: bool) {
        for dot in dots {
            let Some((at, inside)) = self.place(dot.offset, pin) else { continue };
            let Some(forward) = normalize(self.to_screen(dot.facing)) else { continue };
            let alpha = if inside { 1.0 } else { 0.8 };
            let size = if inside { size } else { size * 0.85 };
            self.arrow(draw, at, forward, size * self.m, [color[0], color[1], color[2], alpha]);
        }
    }

    /// Screen position of a world `offset`, and whether it is inside the
    /// radar. Outside, it is pinned to the rim when `pin`, else dropped.
    fn place(&self, offset: [f32; 2], pin: bool) -> Option<([f32; 2], bool)> {
        let screen = self.to_screen(offset);
        let meters = screen[0].hypot(screen[1]);
        let rim = self.radius - 10.0 * self.s;
        if meters * self.pixels_per_meter <= rim {
            return Some((self.at(screen, self.pixels_per_meter), true));
        }
        pin.then(|| (self.at([screen[0] / meters, screen[1] / meters], rim), false))
    }

    /// A dart pointing along `forward` (a unit screen direction).
    fn arrow(&self, draw: &DrawListMut<'_>, at: [f32; 2], forward: [f32; 2], size: f32, color: [f32; 4]) {
        let right = [-forward[1], forward[0]];
        let point = |f: f32, r: f32| {
            [at[0] + (forward[0] * f + right[0] * r) * size, at[1] + (forward[1] * f + right[1] * r) * size]
        };
        let (tip, left, back, side) = (point(1.0, 0.0), point(-0.7, -0.75), point(-0.35, 0.0), point(-0.7, 0.75));
        let fill = self.color(color);
        draw.add_triangle(tip, left, back, fill).filled(true).build();
        draw.add_triangle(tip, back, side, fill).filled(true).build();
        let outline = self.color([OUTLINE[0], OUTLINE[1], OUTLINE[2], OUTLINE[3] * color[3]]);
        draw.add_polyline(vec![tip, left, back, side, tip], outline).thickness(1.2 * self.s.max(0.8)).build();
    }

    /// N, E, S and W on the rim, turning with the camera.
    fn draw_compass(&self, ui: &Ui, draw: &DrawListMut<'_>) {
        for (i, letter) in ["N", "E", "S", "W"].into_iter().enumerate() {
            let (sin, cos) = (i as f32 * std::f32::consts::FRAC_PI_2).sin_cos();
            let at = self.at(self.to_screen([sin, cos]), self.radius - 11.0 * self.s);
            let size = ui.calc_text_size(letter);
            let pos = [(at[0] - size[0] / 2.0).round(), (at[1] - size[1] / 2.0).round()];
            let shadow = self.s.max(1.0);
            draw.add_text([pos[0] + shadow, pos[1] + shadow], self.color(OUTLINE), letter);
            draw.add_text(pos, self.color(if i == 0 { NORTH } else { COMPASS }), letter);
        }
    }

    /// The player's own arrow in the middle.
    fn draw_player(&self, draw: &DrawListMut<'_>) {
        let (sin, cos) = self.radar.yaw.unwrap_or(0.0).sin_cos();
        if let Some(forward) = normalize(self.to_screen([sin, cos])) {
            self.arrow(draw, self.center, forward, 8.0 * self.m, PLAYER);
        }
    }
}

fn normalize([x, y]: [f32; 2]) -> Option<[f32; 2]> {
    let length = x.hypot(y);
    (length > 1e-4).then(|| [x / length, y / length])
}

/// An angle difference wrapped into [-PI, PI) so easing takes the short way.
fn wrap_angle(angle: f32) -> f32 {
    use std::f32::consts::{PI, TAU};
    (angle + PI).rem_euclid(TAU) - PI
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrap_angle_takes_the_short_way_around() {
        let almost = |a: f32, b: f32| (a - b).abs() < 1e-5;
        assert!(almost(wrap_angle(0.1 - 6.0), 0.1 - 6.0 + std::f32::consts::TAU));
        assert!(almost(wrap_angle(3.5), 3.5 - std::f32::consts::TAU));
        assert!(almost(wrap_angle(0.4), 0.4));
    }

    #[test]
    fn normalize_skips_a_zero_vector() {
        assert_eq!(normalize([0.0, 0.0]), None);
        assert_eq!(normalize([0.0, 2.0]), Some([0.0, 1.0]));
    }

    #[test]
    fn what_the_camera_faces_is_up_and_its_right_is_right() {
        let mut radar = Radar::default();
        // Camera looking along +X: its right is +Z... world directions map as the camera sees them.
        radar.camera = ([0.0, 1.0], [1.0, 0.0]);
        let view = View { radar: &radar, center: [0.0, 0.0], radius: 96.0, s: 1.0, m: 1.0, pixels_per_meter: 1.0 };
        assert_eq!(view.to_screen([1.0, 0.0]), [0.0, -1.0]);
        assert_eq!(view.to_screen([0.0, 1.0]), [1.0, 0.0]);
    }
}
