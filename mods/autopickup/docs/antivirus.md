# Antivirus detections

Status: **Confirmed** on VirusTotal, which was re-scanned for every build below. Conventions:
[research notes](../../../docs/research/README.md).

## Question

Why did AutoPickup collect many more VirusTotal detections than mods of similar size, and which
change removes them without changing behavior?

## Results

| Build | Detections | Notes |
|---|---:|---|
| 1.1.0 as first released | 19 | engine families `Mikey` (BitDefender), `Adware` (K7), `CTX`, cloud engines, ESET |
| Rewritten, exports `G_PICKUP_RELOCATED` and `pickup_filter` | 11 | the cloud engines are gone |
| + exports reduced to `DllMain` | 11 | no change |
| + config file name string changed | 11 | no change |
| + relocated prologue in `VirtualAlloc` memory instead of `.data` | **1** | only ESET `GameHack_AGen` is left |
| + PDB path changed to `mod.pdb` in `build.rs` | **0** | the ESET label was caused by the PDB path |

The detections are fewer than they look: several vendors license one engine, so the 11 were four
signatures (`Mikey` on seven engines, `Adware` on two, `CTX`, ESET). ESET's `GameHack_AGen` label stayed until the PDB path was changed (see
[animcancel/docs/antivirus.md](../../animcancel/docs/antivirus.md), which separates the strings that
cause each detection).

## What the detected builds had

The hook copies the first 16 bytes of the game function into a `static` buffer in `.data`,
calls `VirtualProtect` to make it executable, and the trampoline in `.text` ends with
`lea r11, [rip + buffer]; jmp r11`. A DLL that jumps from its code into its own data section is
what several engines key on. The fixed build allocates the buffer with `VirtualAlloc` and the
trampoline loads its address from a static (`mov r11, [rip + RELOCATED_ADDR]`).

## Rejected

- **Exported symbols.** The detected builds exported `G_PICKUP_RELOCATED` and `pickup_filter`
  next to `DllMain`. Referencing them through `sym` in `global_asm!` removes the exports (kept,
  because a mod DLL should export `DllMain` only), but the detections did not change.
- **Config file name.** Another mod's investigation had found that renaming the file name
  string removed a detection family. Changing `AutoPickup.toml` inside the DLL did not change
  anything here.
- Imports are not the cause: the import tables of AutoPickup and of a one-detection mod are
  nearly identical.

## Rules this gives

- Keep hook stubs and relocated code in memory from `shared::codepatch::alloc_executable`,
  never in a `static` that is made executable later.
- Export `DllMain` only.
- Scan the exact file that will be released; results drift over time as vendors add signatures,
  and engines that have not finished are missing from a fresh scan.
  `python scripts/vt-scan.py <file>` uploads and prints the result, `--no-upload` only looks up
  the hash. An upload makes the file public on VirusTotal.
