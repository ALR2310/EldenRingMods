//! Row access for `SoloParamRepository` params without fromsoftware-rs's `rows()`/`rows_mut()`,
//! which read the game's runtime lookup table; some regulations build it one entry shorter than the
//! header's row count and the extra read panics. Rows are walked by index through the param file's
//! own descriptors instead. Callers get a row index, not a param ID; [`row_ids`] maps one to the
//! other.

use eldenring::cs::{ParamResCap, SoloParam, SoloParamRepository};
use eldenring::fd4::ParamFile;
use eldenring::param::ParamDef;

fn res_cap<P: SoloParam>(repo: &SoloParamRepository) -> Option<&ParamResCap> {
    repo.solo_param_holders.get(P::INDEX as usize)?.get_res_cap(0)
}

/// One-line description of `P`'s slot for the log: resource and struct name, paramdef version, rows.
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

/// The `u32` row count the game keeps 0x10 bytes before the param file; the header's own
/// `row_count` is only a `u16`.
fn runtime_row_count(file: &ParamFile) -> u32 {
    // SAFETY: the game keeps this metadata 0x10 bytes before every param file.
    unsafe {
        let metadata = (file as *const _ as *const u8).sub(0x10);
        (metadata.add(4) as *const u32).read_unaligned()
    }
}

/// Checks that the slot fromsoftware-rs hardcodes for `P` holds `P`. It only `debug_assert!`s
/// this, so in a release build on another game version a write could silently corrupt another
/// param.
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

/// Calls `f(index, row)` for every row of `P` in file order; returns the row count.
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

/// Param ID of every row of `P`, in the order of [for_each_row_mut]. Reads the row descriptors
/// after the file header (fromsoftware-rs keeps `row_data_offset` private) and cross-checks every
/// data offset against the row `get_row_by_index` returns; `None` if the layout assumption breaks.
pub fn row_ids<P: SoloParam>(repo: &SoloParamRepository) -> Option<Vec<u32>> {
    row_ids_or_reason::<P>(repo).ok()
}

/// [row_ids], with the reason it gave up, for a log line.
pub fn row_ids_or_reason<P: SoloParam>(repo: &SoloParamRepository) -> Result<Vec<u32>, String> {
    // Typed on purpose: without the annotation `file as *const _` would be the address of the
    // pointer field, not of the param file.
    let file: &ParamFile =
        &res_cap::<P>(repo).ok_or("param slot not available")?.param_res_cap.data;
    let base = file as *const _ as *const u8;
    // SAFETY: byte 0x2D lies inside the param file header.
    let format_2d = unsafe { base.add(0x2D).read() };
    let is_64_bit = format_2d & 0x04 != 0;
    let extended_header = is_64_bit || (format_2d & 0x01 != 0 && format_2d & 0x02 != 0);
    let start = size_of::<ParamFile>() + if extended_header { 0x10 } else { 0 };
    let (stride, data_field) = if is_64_bit { (24, 8) } else { (12, 4) };

    let mut ids = Vec::with_capacity(file.row_count());
    for index in 0..file.row_count() {
        let row = repo.get_row_by_index::<P>(index).ok_or_else(|| {
            format!("row {index} of {} not readable (format {format_2d:#x})", file.row_count())
        })? as *const _ as *const u8;
        // SAFETY: `index < row_count`, so the row descriptor lies inside the param file's descriptor table.
        unsafe {
            let descriptor = base.add(start + index * stride);
            let data_offset = if is_64_bit {
                (descriptor.add(data_field) as *const u64).read_unaligned() as usize
            } else {
                (descriptor.add(data_field) as *const u32).read_unaligned() as usize
            };
            if base.add(data_offset) != row {
                return Err(format!(
                    "row {index} of {}: descriptor data offset {data_offset:#x} != row at {:#x} (format {format_2d:#x}, start {start:#x}, stride {stride})",
                    file.row_count(),
                    (row as usize).wrapping_sub(base as usize)
                ));
            }
            ids.push((descriptor as *const u32).read_unaligned());
        }
    }
    Ok(ids)
}
