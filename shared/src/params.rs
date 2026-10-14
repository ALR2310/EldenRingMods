//! Row access for `SoloParamRepository` params that avoids fromsoftware-rs's
//! `rows()`/`rows_mut()`: those panic on regulations whose runtime lookup
//! table is shorter than the header's row count (Convergence). Root cause:
//! `mods/dropmultiplier/docs/convergence_panic.md`; descriptor layout used by
//! [`row_ids`]: `shared/docs/params.md`.
//!
//! The trade-off: callers get a row index, not a param ID, which is fine for
//! anything that needs a stable key within one session (a hot-reload baseline
//! snapshot) and not for looking up a specific row by ID (use [`row_ids`]).

use eldenring::cs::{ParamResCap, SoloParam, SoloParamRepository};
use eldenring::fd4::ParamFile;
use eldenring::param::ParamDef;

/// The res cap in the slot fromsoftware-rs hardcodes for `P`, if present.
fn res_cap<P: SoloParam>(repo: &SoloParamRepository) -> Option<&ParamResCap> {
    repo.solo_param_holders.get(P::INDEX as usize)?.get_res_cap(0)
}

/// One-line description of `P`'s slot in `repo` for the log: resource name,
/// struct name, paramdef version and row count, so a bug-report log shows what
/// the game actually has there.
pub fn describe<P: SoloParam>(repo: &SoloParamRepository) -> String {
    let Some(res_cap) = res_cap::<P>(repo) else {
        return format!("{} (slot {}): not available", P::NAME, P::INDEX);
    };
    let file = &res_cap.param_res_cap.data;
    format!(
        "{} (slot {}): resource '{}', struct '{}' v{}, {} row(s) (runtime: {})",
        P::NAME,
        P::INDEX,
        res_cap.res_cap.name,
        file.struct_name(),
        file.paramdef_version(),
        file.row_count(),
        runtime_row_count(file)
    )
}

/// The `u32` row count the game keeps in the runtime metadata 0x10 bytes
/// before the param file (after a `u32` name offset). The file header's own
/// `row_count` is only a `u16`; the lookup table is sized by this one.
fn runtime_row_count(file: &ParamFile) -> u32 {
    unsafe {
        let metadata = (file as *const _ as *const u8).sub(0x10);
        (metadata.add(4) as *const u32).read_unaligned()
    }
}

/// Checks that the slot fromsoftware-rs hardcodes for `P` really holds `P`
/// (resource name and struct name match). fromsoftware-rs only
/// `debug_assert!`s this, so in a release build on a game version with a
/// different param order, writing through the wrong slot would silently
/// corrupt another param.
pub fn check<P: SoloParam>(repo: &SoloParamRepository) -> Result<(), String> {
    let res_cap = res_cap::<P>(repo).ok_or_else(|| describe::<P>(repo))?;
    let struct_name = res_cap.param_res_cap.data.struct_name();
    if res_cap.res_cap.name != P::NAME || struct_name != P::StructType::NAME {
        return Err(format!(
            "unexpected param at slot {}: {} (expected '{}' / '{}')",
            P::INDEX,
            describe::<P>(repo),
            P::NAME,
            P::StructType::NAME
        ));
    }
    Ok(())
}

/// Calls `f(index, row)` for every row of `P`, in file order, by row index.
/// Returns the row count.
pub fn for_each_row_mut<P: SoloParam>(
    repo: &mut SoloParamRepository,
    mut f: impl FnMut(usize, &mut P::StructType),
) -> usize {
    let mut index = 0;
    while let Some(row) = repo.get_row_by_index_mut::<P>(index) {
        f(index, row);
        index += 1;
    }
    index
}

/// Param ID of every row of `P`, in the same order as [for_each_row_mut], so
/// a caller can build its own ID -> row map without fromsoftware-rs's `get()`
/// (a binary search over the lookup table `rows_mut()` trips on).
///
/// Reads the row descriptors right after the file header, since
/// fromsoftware-rs keeps `row_data_offset` private. Every descriptor's data
/// offset is cross-checked against the row `get_row_by_index` returns; on any
/// mismatch (the layout assumption broke on some game version) this returns
/// `None` rather than wrong IDs.
pub fn row_ids<P: SoloParam>(repo: &SoloParamRepository) -> Option<Vec<u32>> {
    let file = &res_cap::<P>(repo)?.param_res_cap.data;
    let base = file as *const _ as *const u8;
    let format_2d = unsafe { base.add(0x2D).read() };
    let is_64_bit = format_2d & 0x04 != 0;
    let extended_header = is_64_bit || (format_2d & 0x01 != 0 && format_2d & 0x02 != 0);
    let start = size_of::<ParamFile>() + if extended_header { 0x10 } else { 0 };
    let (stride, data_field) = if is_64_bit { (24, 8) } else { (12, 4) };

    let mut ids = Vec::with_capacity(file.row_count());
    for index in 0..file.row_count() {
        let row = repo.get_row_by_index::<P>(index)? as *const _ as *const u8;
        unsafe {
            let descriptor = base.add(start + index * stride);
            let data_offset = if is_64_bit {
                (descriptor.add(data_field) as *const u64).read_unaligned() as usize
            } else {
                (descriptor.add(data_field) as *const u32).read_unaligned() as usize
            };
            if base.add(data_offset) != row {
                return None;
            }
            ids.push((descriptor as *const u32).read_unaligned());
        }
    }
    Some(ids)
}
