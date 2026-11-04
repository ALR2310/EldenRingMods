//! The drag mode (`General.DragKey`): the mouse is freed from the game so the minimap and the
//! target bar can be dragged to where the player wants them. `docs/layout_drag.md`.
//!
//! The cursor and mouse-block hooks are the ones of `shared::overlay`: while `overlay::MENU_OPEN`
//! is set the game gets no mouse and its cursor is no longer pinned. They are installed the first time the mode starts, not at launch,
//! so a player who never uses it has no extra hook in the game. Dragging changes the offsets at
//! once and the file is written when the button is released.

use std::sync::atomic::{AtomicU8, Ordering};

use hudhook::imgui::{Context, MouseButton, Ui};
use shared::{logger, overlay};

use crate::radar::ui as radar_ui;
use crate::{config, settings};

const HOOKS_NONE: u8 = 0;
const HOOKS_INSTALLING: u8 = 1;
const HOOKS_READY: u8 = 2;
static HOOKS: AtomicU8 = AtomicU8::new(HOOKS_NONE);

/// Starts installing the overlay's cursor and mouse-block hooks, once.
fn install_hooks() {
    if HOOKS
        .compare_exchange(HOOKS_NONE, HOOKS_INSTALLING, Ordering::AcqRel, Ordering::Acquire)
        .is_ok()
    {
        std::thread::spawn(|| {
            overlay::install_input_hooks();
            HOOKS.store(HOOKS_READY, Ordering::Release);
        });
    }
}

fn hooks_ready() -> bool {
    HOOKS.load(Ordering::Acquire) == HOOKS_READY
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Part {
    Minimap,
    Target,
}

/// A part being dragged: where the mouse and the part's offsets were when the button went down.
struct Grab {
    part: Part,
    mouse: [f32; 2],
    start: [f32; 2],
}

#[derive(Default)]
pub struct Drag {
    on: bool,
    buttons_down: [bool; 2],
    grab: Option<Grab>,
    /// An offset moved since the file was written.
    dirty: bool,
}

const FRAME: [f32; 4] = [0.86, 0.70, 0.31, 1.0];
const FRAME_FILL: [f32; 4] = [0.86, 0.70, 0.31, 0.16];
const HINT: [f32; 4] = [0.97, 0.87, 0.57, 1.0];

impl Drag {
    pub fn on(&self) -> bool {
        self.on
    }

    /// Starts or ends the mode (the drag key).
    pub fn toggle(&mut self) {
        self.on = !self.on;
        settings::set_dragging(self.on);
        if self.on {
            install_hooks();
            logger::log("Drag: mode on");
        } else {
            self.save();
            self.grab = None;
            overlay::MENU_OPEN.store(false, Ordering::Relaxed);
            logger::log("Drag: mode off");
        }
    }

    /// Frees and reads the mouse while the mode is on and its hooks are in; gives it back otherwise.
    pub fn before_render(&mut self, ctx: &mut Context) {
        let capture = self.on && hooks_ready();
        if overlay::is_open() != capture {
            overlay::MENU_OPEN.store(capture, Ordering::Relaxed);
        }
        ctx.io_mut().mouse_draw_cursor = capture;
        if capture {
            overlay::feed_mouse(ctx, &mut self.buttons_down);
        } else if self.buttons_down != [false; 2] {
            self.buttons_down = [false; 2];
            ctx.io_mut().add_mouse_button_event(MouseButton::Left, false);
            ctx.io_mut().add_mouse_button_event(MouseButton::Right, false);
        }
    }

    /// Draws a frame around the minimap and the target bar and moves whichever is dragged.
    /// `area` is the game's 16:9 picture, `target` the bar's rectangle (`[x, y, w, h]`).
    pub fn render(&mut self, ui: &Ui, area: [f32; 4], target: [f32; 4]) {
        let capture = hooks_ready();
        let mouse = ui.io().mouse_pos;
        let res = area[3] / 1080.0;
        let (center, radius) = radar_ui::circle(&area);
        let over_minimap = |p: [f32; 2]| (p[0] - center[0]).hypot(p[1] - center[1]) <= radius;
        let over_target = |p: [f32; 2]| {
            p[0] >= target[0]
                && p[0] <= target[0] + target[2]
                && p[1] >= target[1]
                && p[1] <= target[1] + target[3]
        };

        if capture && self.grab.is_none() && ui.is_mouse_clicked(MouseButton::Left) {
            self.grab = if over_target(mouse) {
                Some(Grab { part: Part::Target, mouse, start: settings::target_offset() })
            } else if over_minimap(mouse) {
                Some(Grab { part: Part::Minimap, mouse, start: settings::offset() })
            } else {
                None
            };
        }
        if let Some(grab) = &self.grab {
            if ui.is_mouse_down(MouseButton::Left) {
                // The offsets count from the right and the top edge.
                let x = f64::from(grab.start[0] - (mouse[0] - grab.mouse[0]) / res);
                let y = f64::from(grab.start[1] + (mouse[1] - grab.mouse[1]) / res);
                self.dirty = true;
                match grab.part {
                    Part::Minimap => {
                        settings::set_offset_x(x);
                        settings::set_offset_y(y);
                    }
                    Part::Target => {
                        settings::set_target_offset_x(x);
                        settings::set_target_offset_y(y);
                    }
                }
            } else {
                self.grab = None;
                self.save();
            }
        }

        let draw = ui.get_foreground_draw_list();
        let held = self.grab.as_ref().map(|grab| grab.part);
        let lit = |part: Part, hovered: bool| {
            capture && (held == Some(part) || (held.is_none() && hovered))
        };
        let minimap_lit = lit(Part::Minimap, over_minimap(mouse));
        let target_lit = lit(Part::Target, over_target(mouse));
        let line = 2.0 * res.max(0.75);
        draw.add_circle(center, radius, FRAME_FILL).filled(true).num_segments(64).build();
        draw.add_circle(center, radius, FRAME)
            .thickness(if minimap_lit { line * 2.0 } else { line })
            .num_segments(64)
            .build();
        let corners = [target[0], target[1]];
        let far = [target[0] + target[2], target[1] + target[3]];
        draw.add_rect(corners, far, FRAME_FILL).filled(true).build();
        draw.add_rect(corners, far, FRAME)
            .thickness(if target_lit { line * 2.0 } else { line })
            .build();
        draw.add_text([center[0] - radius * 0.5, center[1] - 8.0 * res], HINT, "Minimap");
        draw.add_text(
            [target[0] + 6.0 * res, target[1] + target[3] * 0.5 - 8.0 * res],
            HINT,
            "Target",
        );
        let hint = if capture {
            "Drag the minimap or the target bar. Press the drag key again to finish."
        } else {
            "Drag mode: getting ready..."
        };
        let width = ui.calc_text_size(hint)[0];
        draw.add_text([area[0] + (area[2] - width) * 0.5, area[1] + 12.0 * res], HINT, hint);
    }

    /// Writes the four offsets into the config file (comments kept).
    fn save(&mut self) {
        if !std::mem::take(&mut self.dirty) {
            return;
        }
        let [x, y] = settings::offset();
        let [tx, ty] = settings::target_offset();
        let saved = config::save(&[
            ("Layout.OffsetX", format!("{}", x as i32)),
            ("Layout.OffsetY", format!("{}", y as i32)),
            ("Layout.TargetOffsetX", format!("{}", tx as i32)),
            ("Layout.TargetOffsetY", format!("{}", ty as i32)),
        ]);
        if saved {
            logger::log(&format!("Drag: saved offsets ({x}, {y}) and target ({tx}, {ty})"));
        } else {
            logger::error("Drag: the offsets could not be saved");
        }
    }
}
