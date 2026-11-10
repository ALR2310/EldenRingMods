# TOML config

Status: **Confirmed**; covered by unit tests for migration, template merging and saving values.
Code: `shared/src/toml_config.rs`.

## Declaring a config

A mod declares a serde struct `T` with `#[derive(Default)]` and
`#[serde(default, deny_unknown_fields)]` on **every** struct. **Defaults live only in the
mod's embedded template.** Writing each value twice, in the template and in a hand-written
`Default`, drifted apart.

The file is read in two passes (`parse`):

1. The user's text alone, to report errors with line numbers. Missing keys take the derived
   zero value in this pass.
2. The user's values laid over the template, which gives the real config: a missing key takes
   the template's value.

`T::default()` is the derived zero value and is never a config in use; `defaults` returns the
template's values. A mod test should assert that `template_missing_keys` is empty, otherwise a
field the template lacks silently becomes zero.

- Missing keys fall back to the template, so an old file still loads.
- Unknown or mistyped keys and wrong types are **errors with a line** (the message comes from
  `toml`), not ignored. At start-up they are moved aside first (below).
- A file that fails to parse never replaces the config in use: at start-up the mod runs on the
  template, on reload the last good config stays.

## Versioned migration

The file carries a format version at `Migration::version_key` (for example
`General.ConfigVersion`); the template holds the current version there. A file without it is
version 1. `Migration::steps[i]` takes a file from version `i + 1` to `i + 2`, so each step runs
exactly once per file, in order.

**A released step never changes.** Files that already went through it will not run it again;
fix a mistake with a new step. A mod that has not been released yet keeps `ConfigVersion = 1`
and `STEPS = &[]`: renaming or moving keys during development edits the template directly.

## Bringing the file in line with the template

At start-up, through `toml_edit`, so the user's comments, values and layout survive:

1. Run the migration steps the file has not seen, then set the version to the current one.
2. Add the keys and tables the template has and the file lacks, with the template's comments.
3. Move keys the template does not have into a comment block at the end of the file, so
   nothing a user set is lost and the file still parses. Top-level names in `Migration::keep`
   (lists such as `[[Override]]`) are left alone.

A file from a **newer** format version (the mod was downgraded) is not touched; otherwise step 3
would delete the new keys. A file that is not valid TOML is also left alone (the parse step
reports it). With no file, the template is written. Every write goes through a temporary file
and a rename.

## Use

The parsed config is an `Arc<T>` snapshot: call `TomlConfig::get` once per tick and read plain
fields, no string lookups. `save_values` writes `("Section.Key", "text")` pairs into the file
keeping comments and layout (each text becomes a bool, an integer, a float or a string) and
re-reads it.

`reload` logs every key whose value changed, one line each, for example
`Config: Materials.Range: 2.0 -> 3.0`, so the log shows what a reload picked up. A config type
must implement `Serialize` for this.

`key_name` is a `deserialize_with` for a hotkey field read by `input::parse_virtual_key`. It
accepts a string (`"F5"`, `"0x74"`) or a bare number (`116`), which TOML would otherwise reject
where a string is expected.

Logging and the reload key are driven by the mod: `logger::set_enabled` and `reload::run`.
