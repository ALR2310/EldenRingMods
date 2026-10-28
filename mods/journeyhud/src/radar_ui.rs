//! The radar drawn on the overlay: a round plate with the player's arrow in
//! the middle, turning with the camera (whatever the camera faces is up),
//! with the other players, the player's spirits and the locked-on enemy as
//! marks around it. Everything is drawn from primitives, no textures.
//! Details: `docs/radar.md`.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicU32, Ordering};

use hudhook::imgui::{DrawListMut, Ui};

use hudhook::imgui::TextureId;

use crate::map::layer::{Icon, IconTexture, MapIcon, MapLayer};
use crate::map::points;
use crate::map::tiles::{TILE, WINDOW, map_pixel};
use crate::radar::{Dot, Snapshot};
use crate::text;
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
const GRACE: [f32; 4] = [1.0, 0.84, 0.47, 1.0];
/// The ring of the game's cursor needles is a little below the middle of the image.
const NEEDLE_PIVOT: [f32; 2] = [0.5, 0.54];
/// Height of a beacon's beam at 1080p.
const BEACON_HEIGHT: f32 = 36.0;
/// Pixels drawn per pixel of the game's map icon sprites (a normal icon is about 156 px); the
/// places, the people and the graces all use it.
const ICON_SCALE: f32 = 0.20;
/// Pixels drawn per pixel of the art of a Map for Goblins marker (96 px at most): 0.3, so 29 px.
const GOBLIN_SCALE: f32 = 0.30;
/// No point icon is drawn bigger than this many pixels on a side, at 1080p.
const POINT_LIMIT: f32 = 42.0;
const BADGE: [f32; 4] = [0.09, 0.08, 0.06, 0.78];
const NORTH: [f32; 4] = [1.0, 0.87, 0.59, 1.0];
const COMPASS: [f32; 4] = [0.84, 0.77, 0.59, 0.9];
const OUTLINE: [f32; 4] = [0.0, 0.0, 0.0, 0.85];
/// Slightly warm, like the parchment of the map it is laid over.
const MAP_TINT: [f32; 4] = [0.93, 0.89, 0.82, 1.0];
/// Wedges of the map disc.
const MAP_SEGMENTS: usize = 72;

/// Font size of the compass letters and the clock at 1080p (ImGui's 13 px font, 1.3 times).
const TEXT_EM: f32 = 16.9;
/// Gap between the minimap and the target bar under it, at 1080p.
const COLUMN_GAP: f32 = 14.0;
/// The same when the time sits in between: the gap under its row, tighter.
const CLOCK_GAP: f32 = 6.0;
/// Height of the row of the in-game time under the minimap, at 1080p.
const CLOCK_ROW: f32 = 22.0;
/// Gap between the rim of the minimap and the time under it or the death count above it, at 1080p.
const TEXT_GAP: f32 = 4.0;

/// How far the column has closed up over a minimap hidden by `Hide.InCaves` or
/// `Hide.InBossFights`, 0 (open) to 1 (closed), as `f32` bits; set by [Radar::draw].
static COLLAPSED: AtomicU32 = AtomicU32::new(0);
/// How fast the column closes up and opens again, per second.
const COLLAPSE_RATE: f32 = 8.0;

/// Height the minimap takes at the top of the right-hand column, gap included,
/// for the target bar to sit under it. Follows the setting, not the fade, so the bar
/// does not jump when the minimap hides for a menu or a loading screen; a minimap hidden by
/// the cave or boss option gives its room to the bar, which slides up.
pub fn column_height(res: f32) -> f32 {
    if !settings::minimap_enabled() {
        return 0.0;
    }
    let below = if settings::minimap_clock() { CLOCK_ROW + CLOCK_GAP + TEXT_GAP } else { COLUMN_GAP };
    let open = 1.0 - f32::from_bits(COLLAPSED.load(Ordering::Relaxed));
    (2.0 * settings::minimap_size() * settings::minimap_scale() + below) * res * open
}

/// The basis that keeps north (+Z) up and east (+X) to the right.
const NORTH_UP: ([f32; 2], [f32; 2]) = ([1.0, 0.0], [0.0, 1.0]);

/// A spot of the trail is kept when the player is this many meters from the last one, up to
/// `TRAIL_LENGTH` of them; a jump of `TELEPORT_DISTANCE` (a warp, a fast travel) starts it over.
const TRAIL_SPACING: f32 = 1.5;
const TRAIL_LENGTH: usize = 800;
const TELEPORT_DISTANCE: f32 = 150.0;

/// What the radar remembers between frames.
pub struct Radar {
    /// 0 to 1, follows whether the radar should be on screen.
    alpha: f32,
    /// The player's facing as an angle, eased so the arrow does not jump.
    yaw: Option<f32>,
    /// The camera's directions on the ground, last good ones.
    camera: ([f32; 2], [f32; 2]),
    /// Whether the last frame was normal play (no menu changing the HUD, no fade).
    in_control: bool,
    /// 0 to 1: how far the column has closed up over a minimap hidden by an option.
    collapsed: f32,
    /// Where the player has been, oldest first, in `trail_space`.
    trail: VecDeque<[f32; 2]>,
    trail_space: i32,
}

impl Default for Radar {
    fn default() -> Self {
        Radar { alpha: 0.0, yaw: None, camera: ([1.0, 0.0], [0.0, 1.0]), in_control: false, collapsed: 0.0, trail: VecDeque::new(), trail_space: 0 }
    }
}

/// Where the radar sits and how big it is on this screen.
pub struct Placement {
    /// `[x, y, width, height]` of the game's 16:9 picture.
    pub area: [f32; 4],
}

impl Radar {
    /// Adds the player's position to the trail when they moved far enough, and starts it over
    /// in another space or after a jump.
    fn follow_trail(&mut self, snapshot: &Snapshot) {
        if !settings::minimap_trail() {
            self.trail.clear();
            return;
        }
        if snapshot.space != self.trail_space {
            self.trail.clear();
            self.trail_space = snapshot.space;
        }
        if let Some(&last) = self.trail.back() {
            let distance = (snapshot.position[0] - last[0]).hypot(snapshot.position[1] - last[1]);
            if distance > TELEPORT_DISTANCE {
                self.trail.clear();
            } else if distance < TRAIL_SPACING {
                return;
            }
        }
        self.trail.push_back(snapshot.position);
        if self.trail.len() > TRAIL_LENGTH {
            self.trail.pop_front();
        }
    }

    /// Whether the last frame was normal play: the time to upload textures.
    pub fn in_control(&self) -> bool {
        self.in_control
    }

    /// Draws the minimap for `snapshot` (`None` = nothing to show, fades out). `layer` holds
    /// the world map and the game's icons; the map is drawn under the marks when `show_map`
    /// is set and the layer has a window for where the player is.
    pub fn draw(&mut self, ui: &Ui, snapshot: Option<&Snapshot>, placement: &Placement, layer: Option<&MapLayer>, show_map: bool) {
        let dt = ui.io().delta_time;
        let in_control = snapshot.is_some_and(|s| s.hud_visible && s.fade < 0.05);
        self.in_control = in_control;
        let hidden = snapshot.is_some_and(|s| {
            (settings::minimap_hide_in_caves() && s.in_cave) || (settings::minimap_hide_in_boss_fights() && s.boss_fight)
        });
        self.collapsed += (if hidden { 1.0 } else { 0.0 } - self.collapsed) * (dt * COLLAPSE_RATE).min(1.0);
        COLLAPSED.store(self.collapsed.to_bits(), Ordering::Relaxed);
        let target = if in_control && !hidden { 1.0 } else { 0.0 };
        self.alpha += (target - self.alpha) * (dt * FADE_RATE).min(1.0);
        let Some(snapshot) = snapshot else { return };
        if self.alpha < 0.01 {
            return;
        }

        self.follow_trail(snapshot);
        if !settings::minimap_rotate() {
            self.camera = NORTH_UP;
        } else if let (Some(right), Some(forward)) = (normalize(snapshot.camera_right), normalize(snapshot.camera_forward)) {
            self.camera = (right, forward);
        }
        if let Some([x, z]) = normalize(snapshot.facing) {
            let wanted = x.atan2(z);
            let yaw = self.yaw.get_or_insert(wanted);
            *yaw += wrap_angle(wanted - *yaw) * (dt * TURN_RATE).min(1.0);
        }

        // `res` follows the screen only (where the column sits); `s` also holds the scale setting.
        let res = placement.area[3] / 1080.0;
        let s = res * settings::minimap_scale();
        let radius = settings::minimap_size() * s;
        let [offset_x, offset_y] = settings::offset();
        let center = [
            placement.area[0] + placement.area[2] - offset_x * res - radius,
            placement.area[1] + offset_y * res + radius,
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
            layer,
        };
        view.draw_plate(&draw);
        if let Some(layer) = layer.filter(|_| show_map) {
            view.draw_map(&draw, snapshot, layer);
        }
        view.draw_vignette(&draw);
        view.draw_frame(&draw);
        view.draw_trail(&draw, snapshot);
        let marks = view.point_marks(snapshot);
        view.draw_points(&draw, &marks);
        view.draw_goblins(&draw, snapshot);
        view.draw_graces(&draw, snapshot);
        view.draw_beacons(&draw, snapshot);
        view.draw_marks(&draw, snapshot);
        if settings::minimap_compass() {
            view.draw_compass(ui, &draw);
        }
        view.draw_player(&draw);
        if settings::minimap_clock() {
            view.draw_clock(ui, snapshot, res);
        }
        if settings::minimap_deaths() {
            view.draw_deaths(ui, snapshot, res);
        }
    }
}

/// A point of interest of the world map placed on the minimap.
struct PointMark {
    at: [f32; 2],
    /// Half the drawn size.
    half: [f32; 2],
    texture: TextureId,
    icon: MapIcon,
    cleared: bool,
    /// A person (the layer above the places).
    npc: bool,
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
    /// The world map and the game's icons, when the map files are available.
    layer: Option<&'a MapLayer>,
}

impl View<'_> {
    /// A game icon, once it is read and uploaded.
    fn icon(&self, icon: Icon) -> Option<IconTexture> {
        self.layer.and_then(|layer| layer.icon(icon))
    }

    /// `icon` centered on `at`, `height` pixels tall (the width keeps its proportions).
    fn draw_icon(&self, draw: &DrawListMut<'_>, icon: IconTexture, at: [f32; 2], height: f32, alpha: f32) {
        let half = [height * icon.size[0] / icon.size[1] / 2.0, height / 2.0];
        draw.add_image(icon.texture, [at[0] - half[0], at[1] - half[1]], [at[0] + half[0], at[1] + half[1]])
            .col(self.color([1.0, 1.0, 1.0, alpha]))
            .build();
    }

    /// A needle-shaped cursor icon turned to point along `forward` (a unit screen direction),
    /// turning on its ring rather than on the middle of the image, with the `emblem` the game
    /// puts in the middle of the ring.
    fn draw_cursor(&self, draw: &DrawListMut<'_>, (needle, emblem): (IconTexture, Option<IconTexture>), at: [f32; 2], forward: [f32; 2], height: f32, alpha: f32) {
        self.draw_needle(draw, needle, at, forward, height, alpha);
        if let Some(emblem) = emblem {
            // Same scale as the needle, centered on the ring.
            let scale = height / needle.size[1];
            self.draw_turned(draw, emblem, at, forward, [emblem.size[0] * scale, emblem.size[1] * scale], [0.5, 0.5], alpha);
        }
    }

    fn draw_needle(&self, draw: &DrawListMut<'_>, icon: IconTexture, at: [f32; 2], forward: [f32; 2], height: f32, alpha: f32) {
        let size = [height * icon.size[0] / icon.size[1], height];
        self.draw_turned(draw, icon, at, forward, size, NEEDLE_PIVOT, alpha);
    }

    /// `icon` of `size` pixels turned to `forward`, on the point of the image at `pivot` (0 to 1).
    #[allow(clippy::too_many_arguments)]
    fn draw_turned(&self, draw: &DrawListMut<'_>, icon: IconTexture, at: [f32; 2], forward: [f32; 2], size: [f32; 2], pivot: [f32; 2], alpha: f32) {
        let right = [-forward[1], forward[0]];
        let corner = |u: f32, v: f32| {
            let (x, y) = ((u - pivot[0]) * size[0], (v - pivot[1]) * size[1]);
            [at[0] + right[0] * x - forward[0] * y, at[1] + right[1] * x - forward[1] * y]
        };
        draw.add_image_quad(icon.texture, corner(0.0, 0.0), corner(1.0, 0.0), corner(1.0, 1.0), corner(0.0, 1.0))
            .col(self.color([1.0, 1.0, 1.0, alpha]))
            .build();
    }

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

    /// The inverse of `to_screen`: the camera's right and forward are orthonormal on the ground.
    fn to_world(&self, [sx, sy]: [f32; 2]) -> [f32; 2] {
        let ([rx, rz], [fx, fz]) = self.radar.camera;
        [sx * rx - sy * fx, sx * rz - sy * fz]
    }

    fn draw_plate(&self, draw: &DrawListMut<'_>) {
        draw.add_circle(self.center, self.radius, self.color(PLATE)).filled(true).num_segments(96).build();
    }

    /// The world map: a fan of wedges clips the texture to the disc and gives each
    /// rim vertex its own rotated UV. Does nothing while there is no window for this spot.
    fn draw_map(&self, draw: &DrawListMut<'_>, snapshot: &Snapshot, map: &MapLayer) {
        let Some(id) = snapshot.map else { return };
        let Some((texture, origin)) = map.window(id) else { return };
        let player = map_pixel(id, snapshot.position);
        let meters_per_pixel = 1.0 / self.pixels_per_meter;
        let uv = |offset: [f32; 2]| {
            let world = self.to_world(offset.map(|d| d * meters_per_pixel));
            let x = player[0] + world[0] - (origin[0] * TILE as i32) as f32;
            let y = player[1] + world[1] - (origin[1] * TILE as i32) as f32;
            [x / WINDOW as f32, 1.0 - y / WINDOW as f32]
        };
        let center_uv = uv([0.0, 0.0]);
        // A window from somewhere else would draw the wrong place until the new one arrives.
        if !center_uv.iter().all(|c| (0.0..=1.0).contains(c)) {
            return;
        }
        let tint = self.color(MAP_TINT);
        let rim = |k: usize| {
            let (sin, cos) = (k as f32 / MAP_SEGMENTS as f32 * std::f32::consts::TAU).sin_cos();
            [cos * self.radius, sin * self.radius]
        };
        for i in 0..MAP_SEGMENTS {
            let (a, b) = (rim(i), rim(i + 1));
            let (pa, pb) = ([self.center[0] + a[0], self.center[1] + a[1]], [self.center[0] + b[0], self.center[1] + b[1]]);
            draw.add_image_quad(texture, self.center, pa, pb, pb).uv(center_uv, uv(a), uv(b), uv(b)).col(tint).build();
        }
    }

    /// Darkens toward the rim so the rim sits on the map instead of cutting it.
    fn draw_vignette(&self, draw: &DrawListMut<'_>) {
        for i in 0..4 {
            let radius = self.radius - (6.0 + i as f32 * 3.0) * self.s;
            let alpha = 0.22 - i as f32 * 0.05;
            draw.add_circle(self.center, radius, self.color([0.0, 0.0, 0.0, alpha]))
                .num_segments(96)
                .thickness(4.0 * self.s)
                .build();
        }
    }

    /// The points of interest of the world map as they are drawn, each with the game's own icon (a
    /// point whose icon is not among the sprites is skipped).
    fn point_marks(&self, snapshot: &Snapshot) -> Vec<PointMark> {
        let Some(layer) = self.layer else { return Vec::new() };
        let rim = self.radius - 10.0 * self.s;
        snapshot
            .points
            .iter()
            .filter_map(|point| {
                let screen = self.to_screen(point.offset);
                if screen[0].hypot(screen[1]) * self.pixels_per_meter > rim {
                    return None;
                }
                let (texture, icon) = layer.map_icon(point.icon)?;
                // The game's sprites are about 156 px for a normal icon; draw them at a fixed fraction
                // of that, but never bigger than `POINT_LIMIT` pixels.
                let npc = points::is_npc(point.icon);
                let scale = (ICON_SCALE * self.m).min(POINT_LIMIT * self.m / icon.size[1].max(icon.size[0]));
                Some(PointMark {
                    at: self.at(screen, self.pixels_per_meter),
                    half: [icon.size[0] * scale / 2.0, icon.size[1] * scale / 2.0],
                    texture,
                    icon,
                    cleared: point.cleared,
                    npc,
                })
            })
            .collect()
    }

    /// The places first, then the people over them, as in the game.
    fn draw_points(&self, draw: &DrawListMut<'_>, marks: &[PointMark]) {
        for mark in marks.iter().filter(|mark| !mark.npc).chain(marks.iter().filter(|mark| mark.npc)) {
            let alpha = if mark.cleared { 0.45 } else { 1.0 };
            draw.add_image(mark.texture, [mark.at[0] - mark.half[0], mark.at[1] - mark.half[1]], [mark.at[0] + mark.half[0], mark.at[1] + mark.half[1]])
                .uv_min(mark.icon.uv_min)
                .uv_max(mark.icon.uv_max)
                .col(self.color([1.0, 1.0, 1.0, alpha]))
                .build();
        }
    }

    /// The path the player has walked, fading toward the oldest end, up to where they stand.
    /// Segments with an end outside the minimap are left out.
    fn draw_trail(&self, draw: &DrawListMut<'_>, snapshot: &Snapshot) {
        let limit = self.radius - 6.0 * self.s;
        let project = |spot: [f32; 2]| {
            let screen = self.to_screen([spot[0] - snapshot.position[0], spot[1] - snapshot.position[1]]);
            self.at(screen, self.pixels_per_meter)
        };
        let inside = |p: [f32; 2]| (p[0] - self.center[0]).hypot(p[1] - self.center[1]) < limit;
        let spots: Vec<[f32; 2]> = self.radar.trail.iter().map(|&spot| project(spot)).chain([self.center]).collect();
        for (i, segment) in spots.windows(2).enumerate() {
            if !inside(segment[0]) || !inside(segment[1]) {
                continue;
            }
            let age = (i + 1) as f32 / (spots.len() - 1) as f32;
            draw.add_line(segment[0], segment[1], self.color([1.0, 0.93, 0.75, 0.10 + 0.55 * age]))
                .thickness(2.0 * self.s.max(0.8))
                .build();
        }
    }

    /// The markers of Map for Goblins, with its icons (a 96 px art is drawn about 16 px at 1080p).
    fn draw_goblins(&self, draw: &DrawListMut<'_>, snapshot: &Snapshot) {
        let Some(layer) = self.layer else { return };
        let rim = self.radius - 10.0 * self.s;
        for marker in &snapshot.goblins {
            let screen = self.to_screen(marker.offset);
            if screen[0].hypot(screen[1]) * self.pixels_per_meter > rim {
                continue;
            }
            let Some((texture, icon)) = layer.goblin_icon(marker.icon) else { continue };
            let at = self.at(screen, self.pixels_per_meter);
            let half = [icon.size[0] * GOBLIN_SCALE / 2.0 * self.m, icon.size[1] * GOBLIN_SCALE / 2.0 * self.m];
            draw.add_image(texture, [at[0] - half[0], at[1] - half[1]], [at[0] + half[0], at[1] + half[1]])
                .uv_min(icon.uv_min)
                .uv_max(icon.uv_max)
                .col(self.color([1.0, 1.0, 1.0, 1.0]))
                .build();
        }
    }

    /// The found sites of grace, each with the game's icon (a plain gold point without it), over the
    /// points of interest as in the game.
    fn draw_graces(&self, draw: &DrawListMut<'_>, snapshot: &Snapshot) {
        let rim = self.radius - 10.0 * self.s;
        let icon = self.icon(Icon::Grace);
        for offset in &snapshot.graces {
            let screen = self.to_screen(*offset);
            if screen[0].hypot(screen[1]) * self.pixels_per_meter > rim {
                continue;
            }
            let at = self.at(screen, self.pixels_per_meter);
            if let Some(icon) = icon {
                self.draw_icon(draw, icon, at, icon.size[1] * ICON_SCALE * self.m, 1.0);
                continue;
            }
            let [r, g, b, _] = GRACE;
            draw.add_circle(at, 7.0 * self.m, self.color([r, g, b, 0.16])).filled(true).build();
            draw.add_circle(at, 4.0 * self.m, self.color([r, g, b, 0.95])).thickness(1.4 * self.m).build();
            draw.add_circle(at, 1.8 * self.m, self.color([1.0, 0.97, 0.87, 1.0])).filled(true).build();
        }
    }

    /// The beacons the player has placed, with the game's beam icon: its lower tip is the spot.
    /// One too far for the minimap is pinned to the rim, like the other players.
    fn draw_beacons(&self, draw: &DrawListMut<'_>, snapshot: &Snapshot) {
        let Some(icon) = self.icon(Icon::Beacon) else { return };
        for offset in &snapshot.beacons {
            let Some((at, inside)) = self.place(*offset, true) else { continue };
            let height = BEACON_HEIGHT * self.m;
            let half = height * icon.size[0] / icon.size[1] / 2.0;
            let alpha = if inside { 1.0 } else { 0.8 };
            draw.add_image(icon.texture, [at[0] - half, at[1] - height], [at[0] + half, at[1]])
                .col(self.color([1.0, 1.0, 1.0, alpha]))
                .build();
        }
    }

    /// The guide rings and the rim.
    fn draw_frame(&self, draw: &DrawListMut<'_>) {
        for fraction in [1.0 / 3.0, 2.0 / 3.0] {
            draw.add_circle(self.center, self.radius * fraction, self.color(GUIDE)).num_segments(96).thickness(self.s.max(1.0)).build();
        }
        draw.add_circle(self.center, self.radius, self.color(RING)).num_segments(96).thickness(2.0 * self.s.max(0.8)).build();
    }

    fn draw_marks(&self, draw: &DrawListMut<'_>, snapshot: &Snapshot) {
        // Spirits first so a partner or the enemy sits on top of them.
        self.draw_dots(draw, &snapshot.spirits, Icon::Spirit, SPIRIT, 4.5, false);
        self.draw_dots(draw, &snapshot.partners, Icon::Partner, PARTNER, 6.5, true);
        if let Some((at, inside)) = snapshot.target.and_then(|offset| self.place(offset, true)) {
            let alpha = if inside { 1.0 } else { 0.8 };
            if let Some(icon) = self.icon(Icon::Enemy) {
                self.draw_icon(draw, icon, at, 18.0 * self.m, alpha);
                return;
            }
            let [r, g, b, _] = ENEMY;
            let radius = 5.0 * self.m;
            draw.add_circle(at, radius, self.color([r, g, b, alpha])).filled(true).build();
            draw.add_circle(at, radius, self.color([OUTLINE[0], OUTLINE[1], OUTLINE[2], OUTLINE[3] * alpha]))
                .thickness(self.m)
                .build();
        }
    }

    fn draw_dots(&self, draw: &DrawListMut<'_>, dots: &[Dot], icon: Icon, color: [f32; 4], size: f32, pin: bool) {
        for dot in dots {
            let Some((at, inside)) = self.place(dot.offset, pin) else { continue };
            let alpha = if inside { 1.0 } else { 0.8 };
            if let Some(texture) = self.icon(icon) {
                match icon {
                    // The cursors of players point the way they face.
                    Icon::Partner => {
                        if let Some(forward) = normalize(self.to_screen(dot.facing)) {
                            let cursor = (texture, self.icon(Icon::PartnerEmblem));
                            self.draw_cursor(draw, cursor, at, forward, 30.0 * self.m, alpha);
                        }
                    }
                    _ => self.draw_icon(draw, texture, at, 15.0 * self.m, alpha),
                }
                continue;
            }
            let Some(forward) = normalize(self.to_screen(dot.facing)) else { continue };
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
        let em = TEXT_EM * self.m;
        for (i, letter) in ["N", "E", "S", "W"].into_iter().enumerate() {
            let (sin, cos) = (i as f32 * std::f32::consts::FRAC_PI_2).sin_cos();
            let at = self.at(self.to_screen([sin, cos]), self.radius - 12.0 * self.s);
            draw.add_circle(at, 8.0 * self.m, self.color(BADGE)).filled(true).build();
            let size = text::measure(ui, letter, em);
            let pos = [(at[0] - size[0] / 2.0).round(), (at[1] - size[1] / 2.0).round()];
            let color = self.color(if i == 0 { NORTH } else { COMPASS });
            text::add_shadowed(letter, pos, em, color, self.color(OUTLINE), self.s.max(1.0));
        }
    }

    /// The in-game time, centered in its row right under the minimap.
    fn draw_clock(&self, ui: &Ui, snapshot: &Snapshot, res: f32) {
        let Some([hours, minutes]) = snapshot.clock else { return };
        let row_top = self.center[1] + self.radius + TEXT_GAP * res;
        self.draw_centered(ui, &format!("{hours:02}:{minutes:02}"), row_top, res);
    }

    /// The death count, centered in a row right above the minimap. It takes no room from the
    /// column (the minimap does not move down); at the very top of the screen it is clamped on.
    fn draw_deaths(&self, ui: &Ui, snapshot: &Snapshot, res: f32) {
        let Some(deaths) = snapshot.deaths else { return };
        let row_top = (self.center[1] - self.radius - (CLOCK_ROW + TEXT_GAP) * res).max(0.0);
        self.draw_centered(ui, &format!("Deaths {deaths}"), row_top, res);
    }

    /// `text` centered on the minimap horizontally and in the middle of a clock-sized row.
    fn draw_centered(&self, ui: &Ui, text: &str, row_top: f32, res: f32) {
        let em = TEXT_EM * res;
        let size = text::measure(ui, text, em);
        let pos = [(self.center[0] - size[0] / 2.0).round(), (row_top + (CLOCK_ROW * res - size[1]) / 2.0).round()];
        text::add_shadowed(text, pos, em, self.color(COMPASS), self.color(OUTLINE), self.s.max(1.0));
    }

    /// The player's own arrow in the middle.
    fn draw_player(&self, draw: &DrawListMut<'_>) {
        let (sin, cos) = self.radar.yaw.unwrap_or(0.0).sin_cos();
        if let Some(forward) = normalize(self.to_screen([sin, cos])) {
            match self.icon(Icon::Player) {
                Some(icon) => {
                    let cursor = (icon, self.icon(Icon::PlayerEmblem));
                    self.draw_cursor(draw, cursor, self.center, forward, 34.0 * self.m, 1.0);
                }
                None => self.arrow(draw, self.center, forward, 8.0 * self.m, PLAYER),
            }
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
    fn north_up_puts_north_at_the_top_and_east_on_the_right() {
        let mut radar = Radar::default();
        radar.camera = NORTH_UP;
        let view = View { radar: &radar, center: [0.0, 0.0], radius: 96.0, s: 1.0, m: 1.0, pixels_per_meter: 1.0, layer: None };
        assert_eq!(view.to_screen([0.0, 1.0]), [0.0, -1.0]);
        assert_eq!(view.to_screen([1.0, 0.0]), [1.0, 0.0]);
    }

    #[test]
    fn what_the_camera_faces_is_up_and_its_right_is_right() {
        let mut radar = Radar::default();
        // Camera looking along +X: its right is +Z... world directions map as the camera sees them.
        radar.camera = ([0.0, 1.0], [1.0, 0.0]);
        let view = View { radar: &radar, center: [0.0, 0.0], radius: 96.0, s: 1.0, m: 1.0, pixels_per_meter: 1.0, layer: None };
        assert_eq!(view.to_screen([1.0, 0.0]), [0.0, -1.0]);
        assert_eq!(view.to_screen([0.0, 1.0]), [1.0, 0.0]);
    }
}
