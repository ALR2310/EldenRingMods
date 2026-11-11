//! Hides the health bars and the damage numbers the game floats above ordinary enemies. They are
//! the `HP` and `Damage` clips and the dark shape behind the bar (depth 1, without a name) of the
//! `EnemyTag0` to `EnemyTag7` clips of the HUD movie (one sprite shared by all eight): the movie is
//! edited as the game opens it, and they go far below the picture. The name of the tag stays.

use shared::gfx;
use shared::logger;

use crate::settings;

const PATH: [&str; 1] = ["EnemyTag0"];
const BACKGROUND_DEPTH: u16 = 1;
const TWIPS_PER_PX: i32 = 20;
/// The picture is 1080 px high; this is well past its bottom edge.
const OFF_PICTURE_PX: i32 = 5000;

/// `movie` with the bar moved away, `None` to leave it as it is.
pub fn edit(movie: &[u8]) -> Option<Vec<u8>> {
    if settings::enemy_hp_bar() {
        return None;
    }
    let dy = OFF_PICTURE_PX * TWIPS_PER_PX;
    let edited = gfx::shift_placements(movie, &PATH, &[("HP", 0, dy), ("Damage", 0, dy)])
        .and_then(|hp_moved| gfx::shift_depths(&hp_moved, &PATH, &[(BACKGROUND_DEPTH, 0, dy)]));
    match edited {
        Ok(edited) => Some(edited),
        Err(err) => {
            logger::warn(&format!("EnemyTag: movie edit failed ({err:?}), the bars stay"));
            None
        }
    }
}
