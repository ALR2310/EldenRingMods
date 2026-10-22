//! The window size math, free of Win32 so it is unit tested: the minimum
//! client size, the optional aspect lock while dragging, and the startup size
//! from `Width` / `Height`. All sizes are of the client area (the game image),
//! not the outer window; `frame` is the outer size minus the client size.

use common::config;

#[repr(C)]
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub struct Rect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

// `WM_SIZING` wParam: which edge or corner is being dragged.
pub const WMSZ_LEFT: usize = 1;
pub const WMSZ_TOP: usize = 3;
pub const WMSZ_TOPLEFT: usize = 4;
pub const WMSZ_TOPRIGHT: usize = 5;
pub const WMSZ_BOTTOM: usize = 6;
pub const WMSZ_BOTTOMLEFT: usize = 7;

/// What the client area must respect.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    /// Smallest client size.
    pub min: (i32, i32),
    /// `(width, height)` ratio to keep, `None` to resize freely.
    pub aspect: Option<(i64, i64)>,
}

impl Limits {
    pub fn from_config() -> Self {
        let aspect_w = config::get_int("AspectWidth", 16) as i64;
        let aspect_h = config::get_int("AspectHeight", 9) as i64;
        Self {
            min: (config::get_int("MinWidth", 160).max(1), config::get_int("MinHeight", 90).max(1)),
            aspect: (aspect_w > 0 && aspect_h > 0).then_some((aspect_w, aspect_h)),
        }
    }

    /// The height that keeps the aspect for `width` (unchanged without a lock).
    fn height_for(&self, width: i32, current: i32) -> i32 {
        match self.aspect {
            Some((aw, ah)) => ((width as i64 * ah + aw / 2) / aw) as i32,
            None => current,
        }
    }

    fn width_for(&self, height: i32, current: i32) -> i32 {
        match self.aspect {
            Some((aw, ah)) => ((height as i64 * aw + ah / 2) / ah) as i32,
            None => current,
        }
    }
}

/// Resizes the dragged `rect` (outer window, screen coordinates) so its client
/// area respects `limits`, moving only the edges the user is actually
/// dragging.
pub fn constrain(edge: usize, rect: &mut Rect, frame: (i32, i32), limits: &Limits) {
    let (frame_w, frame_h) = frame;
    let (min_w, min_h) = limits.min;
    let mut w = (rect.right - rect.left - frame_w).max(min_w);
    let mut h = (rect.bottom - rect.top - frame_h).max(min_h);

    if limits.aspect.is_some() {
        // Top and bottom edges drive the width; every other edge and corner
        // drives the height from the width.
        if matches!(edge, WMSZ_TOP | WMSZ_BOTTOM) {
            w = limits.width_for(h, w);
        } else {
            h = limits.height_for(w, h);
        }
        if h < min_h {
            h = min_h;
            w = limits.width_for(h, w);
        }
        if w < min_w {
            w = min_w;
            h = limits.height_for(w, h);
        }
    }

    let (outer_w, outer_h) = (w + frame_w, h + frame_h);
    if matches!(edge, WMSZ_LEFT | WMSZ_TOPLEFT | WMSZ_BOTTOMLEFT) {
        rect.left = rect.right - outer_w;
    } else {
        rect.right = rect.left + outer_w;
    }
    if matches!(edge, WMSZ_TOP | WMSZ_TOPLEFT | WMSZ_TOPRIGHT) {
        rect.top = rect.bottom - outer_h;
    } else {
        rect.bottom = rect.top + outer_h;
    }
}

/// The client size to start with from `Width` / `Height` (0 = unset), or
/// `None` if both are unset. With one set, the other follows the aspect (or
/// keeps its `current` value without a lock). Never below the minimum.
pub fn start_client_size(width: i32, height: i32, current: (i32, i32), limits: &Limits) -> Option<(i32, i32)> {
    let size = match (width > 0, height > 0) {
        (false, false) => return None,
        (true, true) => (width, height),
        (true, false) => (width, limits.height_for(width, current.1)),
        (false, true) => (limits.width_for(height, current.0), height),
    };
    Some((size.0.max(limits.min.0), size.1.max(limits.min.1)))
}

#[cfg(test)]
mod tests {
    use super::*;

    const FRAME: (i32, i32) = (16, 39);
    const FREE: Limits = Limits { min: (160, 90), aspect: None };
    const LOCKED: Limits = Limits { min: (160, 90), aspect: Some((16, 9)) };

    /// An outer window whose client is `w` x `h`, at (100, 100).
    fn window(w: i32, h: i32) -> Rect {
        Rect { left: 100, top: 100, right: 100 + w + FRAME.0, bottom: 100 + h + FRAME.1 }
    }

    fn client(rect: &Rect) -> (i32, i32) {
        (rect.right - rect.left - FRAME.0, rect.bottom - rect.top - FRAME.1)
    }

    const WMSZ_RIGHT: usize = 2;
    const WMSZ_BOTTOMRIGHT: usize = 8;

    #[test]
    fn dragging_a_side_edge_keeps_the_aspect_by_changing_the_height() {
        let mut rect = window(1280, 500);
        constrain(WMSZ_RIGHT, &mut rect, FRAME, &LOCKED);
        assert_eq!(client(&rect), (1280, 720));
        assert_eq!((rect.left, rect.top), (100, 100)); // the left and top edges stay put
    }

    #[test]
    fn dragging_the_top_or_bottom_edge_changes_the_width() {
        let mut rect = window(500, 720);
        constrain(WMSZ_BOTTOM, &mut rect, FRAME, &LOCKED);
        assert_eq!(client(&rect), (1280, 720));
        let mut rect = window(500, 720);
        constrain(WMSZ_TOP, &mut rect, FRAME, &LOCKED);
        assert_eq!(client(&rect), (1280, 720));
        assert_eq!(rect.bottom, window(500, 720).bottom); // a top drag anchors the bottom edge
    }

    #[test]
    fn dragging_the_left_edge_anchors_the_right_edge() {
        let mut rect = window(1000, 700);
        let right = rect.right;
        constrain(WMSZ_LEFT, &mut rect, FRAME, &LOCKED);
        assert_eq!(rect.right, right);
        assert_eq!(client(&rect), (1000, 563)); // 1000 * 9 / 16 = 562.5, rounded
    }

    #[test]
    fn the_minimum_wins_even_with_the_aspect_lock() {
        let mut rect = window(20, 20);
        constrain(WMSZ_BOTTOMRIGHT, &mut rect, FRAME, &LOCKED);
        assert_eq!(client(&rect), (160, 90));
        // A tall minimum forces the width up to keep the ratio.
        let tall = Limits { min: (100, 180), aspect: Some((16, 9)) };
        let mut rect = window(50, 50);
        constrain(WMSZ_BOTTOMRIGHT, &mut rect, FRAME, &tall);
        assert_eq!(client(&rect), (320, 180));
    }

    #[test]
    fn without_a_lock_both_sides_are_free_down_to_the_minimum() {
        let mut rect = window(900, 300);
        constrain(WMSZ_BOTTOMRIGHT, &mut rect, FRAME, &FREE);
        assert_eq!(client(&rect), (900, 300));
        let mut rect = window(10, 10);
        constrain(WMSZ_BOTTOMRIGHT, &mut rect, FRAME, &FREE);
        assert_eq!(client(&rect), (160, 90));
    }

    #[test]
    fn the_start_size_follows_the_aspect_for_the_missing_side() {
        assert_eq!(start_client_size(0, 0, (800, 450), &LOCKED), None);
        assert_eq!(start_client_size(1600, 900, (800, 450), &LOCKED), Some((1600, 900)));
        assert_eq!(start_client_size(1280, 0, (800, 450), &LOCKED), Some((1280, 720)));
        assert_eq!(start_client_size(0, 720, (800, 450), &LOCKED), Some((1280, 720)));
    }

    #[test]
    fn without_a_lock_the_missing_side_keeps_its_current_value() {
        assert_eq!(start_client_size(1280, 0, (800, 450), &FREE), Some((1280, 450)));
        assert_eq!(start_client_size(0, 720, (800, 450), &FREE), Some((800, 720)));
    }

    #[test]
    fn the_start_size_is_never_below_the_minimum() {
        assert_eq!(start_client_size(50, 50, (800, 450), &LOCKED), Some((160, 90)));
    }
}
