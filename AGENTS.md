# Project guidelines

Rust workspace of Elden Ring mods (one crate per DLL, shared code in `common`). Read
[CONTRIBUTING.md](CONTRIBUTING.md) for layout, build commands and the full conventions; this
file covers how to approach a change.

## Before writing code

- Read 3 to 5 existing files that already do something similar and match their structure,
  naming and error handling. Copying the project's habits matters more than any written rule.
  Good references: see the list at the bottom.
- Look for an existing helper in `common` before writing a new one.

## Coding philosophy

- Prefer simple, readable solutions. Straightforward beats clever.
- Follow existing project conventions over personal preference.
- Keep changes minimal and focused. Do not refactor unrelated code.
- No abstractions, traits or wrapper types without a concrete need today.
- No helper functions for trivial one-liners.
- Do not comment obvious code. Comment the reason when it is not apparent. No doc comments on
  items whose name and signature already say it.
- Reuse existing utilities instead of reimplementing them.

### Where defensive code is required

Mods run inside the game process, so "avoid excessive checks" does not apply to the game boundary:

- Game pointers and singletons can be null or not yet initialized; check them.
- Nothing that can panic, allocate or lock runs inside a hook.
- Find game functions by AOB pattern, never a hard-coded RVA.

Everywhere else (internal calls between our own functions, values we just validated) do not
add redundant checks.

## Before finishing a task

- Review every changed file as a reviewer would.
- Remove unused code, imports and leftover debugging output.
- Check for duplicated logic and inconsistent naming.
- Ask whether the solution is over-engineered; if so, simplify.
- Run `cargo fmt --all`, `cargo clippy --workspace --all-targets` and `cargo test --workspace`.
- Do not claim something works in game unless it was actually tested there.

## Reference files

Filled in once the first mods are migrated (pick the cleanest, one per kind):

- Mod entry point (`lib.rs`):
- Feature module with a game hook:
- Feature module without a hook:
- Config handling:
- Mod README:
