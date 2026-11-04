# Contributing

[Tiếng Việt](CONTRIBUTING.vi.md)

Thanks for looking at the code. This page describes how the repository is laid out and the
conventions every mod follows, so a change in one mod reads like a change in any other.

If you use an AI assistant, point it at [AGENTS.md](AGENTS.md) and run the prompt in
[docs/review-prompt.md](docs/review-prompt.md) on its output before committing.

## Repository layout

```
shared/            code used by every mod (config, logging, memory scanning, hooks, ...)
mods/<name>/       one mod = one crate = one DLL
docs/              project-wide documentation
scripts/           build and release helpers
```

A mod looks like this:

```
mods/<name>/
├── Cargo.toml          must contain `[lints] workspace = true`
├── build.rs            embeds the Windows version resource
├── <Name>.toml         default config, embedded into the DLL at build time
├── README.md           English: what it does, how to install it, links to the docs
├── README.vi.md        Vietnamese translation
├── CHANGELOG.md        user-facing release notes (English)
├── nexus_page.bbcode   description for the mod's Nexus page (BBCode), when it has one
├── license-notice.txt  optional; any *license*.txt / *notice*.txt here or in assets/ is
│                       packed with the DLL by scripts/build-mod.ps1
└── src/
    ├── lib.rs          DllMain and wiring only
    └── ...             one module per feature
```

Anything that needs more than a README (research notes, offsets, how a game system works) goes
in `mods/<name>/docs/<topic>.md`. Write it the way [docs/research/README.md](docs/research/README.md)
describes: evidence labels, the build each finding was tested on, and rejected paths kept on the
page.

### The `shared` crate

Mods share code through `shared` instead of copying it, so a fix applies to all of them. Each mod
links its own copy, so mods share no runtime state. Look for an existing helper here before writing
one; how a module works and why is explained in `shared/docs/`.

| Group | Modules | Role |
| --- | --- | --- |
| Config and logs | `toml_config`, `logger`, `diag`, `crash`, `i18n` | TOML with migration, log file, environment dump, crash trace, menu texts |
| Memory | `memscan`, `codepatch`, `conflict` | AOB scan, code hooks, detecting another mod's hook |
| Game engine | `task`, `alloc_hook`, `player`, `params`, `anim` | Per-frame tasks and engine lookups that survive game updates, the "player is in the world" gate, safe param access, animation groups |
| Interaction | `input`, `reload`, `announce`, `menu` | Hotkeys, config reload, top-of-screen banner, in-game menu |

## Building

Requirements: Windows x64, the stable Rust toolchain with the MSVC target, and the Windows SDK
(`rc.exe` is needed for the version resource).

```powershell
cargo build --release -p <crate>
cargo test --workspace
cargo fmt --all
cargo clippy --workspace --all-targets
```

To build and package a mod: `pwsh scripts/build-mod.ps1 -Mod <Mod> [-Zip]` (output in
`builds/<crate>/`).

Scripts in `scripts/` are PowerShell for simple build and packaging steps. Use Python (3.10+,
standard library only where possible) when a script needs real logic, such as parsing JSON or
calling an API.

CI runs `fmt --check`, `clippy -D warnings` and `test`; run them locally before opening a PR.

## Code conventions

**Comments say why, not what.** If a comment restates the next line, delete it. Good comments
explain a constraint, a game quirk, or a decision that is not obvious from the code.

- No section banners, no numbered "Step 1 / Step 2" narration, no emoji.
- No dates, issue numbers, user names or "fixed in ..." history in comments. That belongs in the
  commit message or the changelog.
- Doc comments (`///`) on public items only when the name and signature do not already say it.
- Every `unsafe` block gets a `// SAFETY:` comment stating why it is sound.

**Structure.**

- `lib.rs` stays thin: load config, start logging, register features.
- One feature per module. If a file passes roughly 400 lines, look for a seam to split on.
- Prefer small functions with early returns over deep nesting.
- `shared` grows with the mods: bring a module over only when the mod being added needs it.
- Shared logic lives in `shared`. Do not copy a helper between mods; do not move code into
  `shared` until a second mod actually needs it.
- Mods are linked statically against `shared` and share no runtime state with each other.

**Errors and panics.** A mod runs inside the game process, so it must not take the game down.
Use `Result` and log the failure; reserve `unwrap`/`expect` for invariants that cannot fail.
Nothing that can panic runs inside a hook.

**Game version safety.**

- Locate game functions with AOB patterns (`shared::memscan`), never hard-coded RVAs, so a game
  update does not silently break the mod. Check that a new pattern is unique on every supported
  executable version.
- Do not call `eldenring::rva::get()`; the table only covers some versions and panics on others.
- Do not set hardware breakpoints (debug registers) from a DLL; the game crashes.
- Inline hooks must not allocate or take locks.

**Antivirus detections.** Mods edit game memory, so some detections are unavoidable, but a few
habits add many more:

- Do not run code from a `static` that was made executable; use
  `shared::codepatch::alloc_executable`. A DLL that jumps into its own data section is flagged by
  several engines.
- Do not export anything except `DllMain`. Use `sym` in `global_asm!` instead of `#[no_mangle]`.
- Do not pack, obfuscate or encrypt the DLL.
- Do not spell the mod's file names (`<Mod>.toml`, `<Mod>.log`) in the code; build them from
  `shared::dll_stem`.
- Keep the `/PDBALTPATH:mod.pdb` line in `build.rs`, and keep the mod's name out of
  `FileDescription`. Engines have flagged the name in each of these places.

**Config.** TOML, one file next to the DLL, created from the embedded default on first run.
Missing keys are added from the default without touching the user's comments or values. Every
key in the default file has a one-line comment.

**Logging.** One log file per mod, written next to the DLL. Messages are short and technical:
`Component: state`, for example `Menu: registered`. Failures include the reason. Do not dump
long lists at normal verbosity. Log lines are for developers, not players; the exception is a
deliberate hint such as `game may need a mod update, check Nexus`.

**Naming.** Rust standard: `snake_case` functions and modules, `UpperCamelCase` types,
`SCREAMING_SNAKE_CASE` constants. The only exception is the crate's `[lib] name`, which is
PascalCase so the DLL gets the right file name (`#![allow(non_snake_case)]` in `lib.rs`).

## Documentation conventions

- User-facing text (README, CHANGELOG) is written for players: say what the mod does, not how
  the code is organized.
- Every mod README is short and has the same three sections: About (what it does and its known
  limits, in plain language), Installation, and Documentation (changelog, Nexus page and the
  `docs/` pages). How it works belongs in `docs/`, not in the README.
- Technical notes (offsets, how a game system works) go in `mods/<name>/docs/`.
- README and CHANGELOG are English. Add a Vietnamese README (`README.vi.md`) for each mod and
  keep it in sync.
- CHANGELOG follows [Keep a Changelog](https://keepachangelog.com): one short line per change,
  such as `Added X option.` or `Fixed crash when Y.` Only user-facing features and fixes belong
  there: leave out log, debug, build and other development changes. Versions follow
  [SemVer](https://semver.org). Bump `version` in the mod's `Cargo.toml` in the same commit
  that turns `[Unreleased]` into a release.

## Git

- `main` is always buildable. Work on a branch named `<type>/<short-topic>`, for example
  `feat/autopickup-radius`, `fix/speed-seamless`.
- Commits follow [Conventional Commits](https://www.conventionalcommits.org) with the mod name
  as scope: `fix(autoregen): stop healing while dead`. Subject in the imperative, under 72
  characters. One logical change per commit.
- Do not commit binaries, game files, `regulation.bin`, crash dumps, API keys or local paths.
- `.local/` (personal notes, a TODO list, scratch files) and `dumps/` are ignored by git. Keep
  anything that is not meant for the repository there.
- Test a change in the game before opening a PR, and say which game version and mod setup you
  tried (vanilla, Seamless Co-op, ...).
