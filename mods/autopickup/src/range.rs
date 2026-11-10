//! `Range` scales `radius`, `height` and `baseHeightOffset` of each group's `ActionButtonParam`
//! rows, always from the game's own values (kept from the first scan) so it never compounds.

use std::sync::Mutex;

use eldenring::cs::{ActionButtonParam, SoloParamRepository};

use shared::{logger, params};

use crate::groups::Group;

/// `Range` is clamped to this, so a typo of `100` cannot make the whole map one prompt.
pub const MIN_RANGE: f64 = 0.1;
pub const MAX_RANGE: f64 = 10.0;

struct Row {
    group: Group,
    index: usize,
    radius: f32,
    height: f32,
    base_height_offset: f32,
}

static ROWS: Mutex<Option<Vec<Row>>> = Mutex::new(None);

pub fn clamp_range(range: f64) -> f32 {
    if range.is_finite() { range.clamp(MIN_RANGE, MAX_RANGE) as f32 } else { 1.0 }
}

fn scan(repo: &mut SoloParamRepository) -> Result<Vec<Row>, String> {
    params::check::<ActionButtonParam>(repo)?;
    let row_ids = params::row_ids_or_reason::<ActionButtonParam>(repo)?;
    let mut rows = Vec::new();
    params::for_each_row_mut::<ActionButtonParam>(repo, |index, row| {
        let Some(&id) = row_ids.get(index) else {
            return;
        };
        if let Some(group) = crate::groups::classify(id) {
            rows.push(Row {
                group,
                index,
                radius: row.radius(),
                height: row.height(),
                base_height_offset: row.base_height_offset(),
            });
        }
    });
    Ok(rows)
}

/// Writes each group's scale onto its rows (a disabled group gets 1.0, the vanilla values).
pub fn apply(repo: &mut SoloParamRepository, ranges: &[(Group, f32)]) -> Result<usize, String> {
    let mut cache = ROWS.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    if cache.is_none() {
        // A failed scan is not cached, so the next reload tries again.
        *cache = Some(scan(repo)?);
    }
    let Some(rows) = cache.as_ref() else {
        return Ok(0);
    };
    let mut written = 0;
    for row in rows {
        let scale =
            ranges.iter().find(|(group, _)| *group == row.group).map_or(1.0, |(_, scale)| *scale);
        if let Some(target) = repo.get_row_by_index_mut::<ActionButtonParam>(row.index) {
            target.set_radius(row.radius * scale);
            target.set_height(row.height * scale);
            target.set_base_height_offset(row.base_height_offset * scale);
            written += 1;
        }
    }
    if written != rows.len() {
        logger::warn(&format!("Range: wrote {written} of {} row(s)", rows.len()));
    }
    Ok(written)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn range_is_clamped() {
        assert_eq!(clamp_range(1.0), 1.0);
        assert_eq!(clamp_range(100.0), MAX_RANGE as f32);
        assert_eq!(clamp_range(0.0), MIN_RANGE as f32);
        assert_eq!(clamp_range(f64::NAN), 1.0);
    }
}
