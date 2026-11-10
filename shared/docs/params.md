# Param rows

Status: **Confirmed** in game on 1.16.2 with Convergence (the case below) and on vanilla 1.17. Conventions:
[research notes](../../docs/research/README.md). Code: `shared/src/params.rs`.

## Question

How does a mod read and write the rows of a param table without panicking on a regulation whose header
disagrees with the game's own tables?

## The failure

With Convergence 3.0.2.0 a mod's thread died right after logging the snapshot of `ItemLotParam_enemy`:
`param_repository.rs:357:22: called Option::unwrap() on a None value`. The mutex of the snapshot was then
poisoned, so nothing was ever applied. Vanilla 1.17 worked (5135 rows); 1.16.2 with Convergence panicked
every time.

Line 357 is in `ParamFile::rows_mut()`: `get_row_by_index_mut(lookup.index).unwrap()`. It walks the
**lookup table** the game builds after each param file, a list of `(param_id, index)` pairs sorted by id,
but sizes the walk by the `row_count` in the file header (a `u16`).

A diagnostic log showed that the header of `ItemLotParam_enemy` says **4631** rows while the runtime count
(a `u32` in the 0x10 bytes of metadata before the file, which the game really uses to build the lookup
table) is **4630**. The game builds a table one entry shorter than the header, probably because the file
has one duplicated row id, and the crate reads one entry too many into garbage memory, gets an index above
`row_count` and panics.

Rejected: a `u16` overflow of `row_count` (the header has only 4631 rows) and a different layout between
1.16.2 and 1.17 (slot 20 is `ItemLotParam_enemy` and `ITEMLOT_PARAM_ST` in both).

## The helpers

Do not use `ParamFile::rows()` or `rows_mut()` of fromsoftware-rs. Use:

- `for_each_row_mut::<P>(repo, f)`: walks by index through the file's own row descriptors
  (`get_row_by_index_mut` until it returns `None`), never touching the lookup table.
- `check::<P>(repo)`: verifies that the slot really holds `P` before writing. The crate only has a
  `debug_assert!` for it, which a release build lacks.
- `describe::<P>(repo)`: resource name, struct, paramdef version, header and runtime row counts, so a
  failure log answers its own question.
- `row_ids::<P>(repo)`: the id of every row, read from the row descriptors after the file header, with
  every data offset cross-checked against the row `get_row_by_index` returns. If the layout assumption
  breaks it returns `None` rather than wrong ids.

Snapshots are keyed by row index (stable within a session), not by id.

## Open risks

Other crates of fromsoftware-rs that walk params with `rows()` have the same weakness. A table panics only
when the regulation in use has a header that disagrees with the runtime count.
