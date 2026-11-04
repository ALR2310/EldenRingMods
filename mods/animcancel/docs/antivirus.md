# Antivirus detections

Status: **Confirmed** on VirusTotal, one scan per variant below. Conventions:
[research notes](../../../docs/research/README.md).

## Question

A straight port of AnimCancel was detected by 13 engines, while mods of similar size had one. Which
strings in the DLL cause it, and can they be removed without renaming the mod, its DLL or its config
file?

## Results

Rows build on the one above unless they say otherwise. The DLL is always named `AnimCancel.dll`.

| Variant | Detections | Engines |
|---|---:|---|
| Straight port | 13 | `Doina` family on 7 engines, CTX, Cynet, ESET, K7 (2), Malwarebytes |
| Config and log file names built at run time from the DLL's own name, so `AnimCancel.toml` is not a string in the DLL | 5 | `Doina` and CTX gone |
| Same, with the menu, probes, weapon rule and cancel code switched off | 5 | no change, so it is not the logic |
| That minimal build with every identity string changed to `Zeta` | 1 | Cynet |
| Run-time file names, PDB path changed to `mod.pdb` | 1 | Malwarebytes `RiskWare.GameMod` only |
| + `InternalName` and `OriginalFilename` changed | 1 | Malwarebytes only |
| + `ProductName` changed, `FileDescription` kept as `Anim Cancel - Elden Ring mod` | 1 | Malwarebytes only |
| Run-time file names, PDB path changed, `FileDescription` changed to `Elden Ring mod` | **0** | none |

The last row is what the repository builds: `ProductName`, `InternalName` and `OriginalFilename` keep the
mod's name.

## What each string does

| String in the DLL | Detections it caused |
|---|---|
| the literal config file name `AnimCancel.toml` | `Doina` family (7 engines) and CTX |
| the PDB path `AnimCancel.pdb` in the debug directory | K7 (2), Cynet, ESET |
| the mod's name in `FileDescription` | Malwarebytes `RiskWare.GameMod` |

The mechanism is not known; these are the engines' reactions to the strings, measured one at a time.
An earlier investigation had found only the file name; the other two appear as engines add signatures.

## Rules this gives

- Build a mod's file names from `shared::dll_stem` at run time, never spell `<Mod>.toml` or
  `<Mod>.log` out.
- Give the linker a neutral PDB path (`/PDBALTPATH:mod.pdb` in `build.rs`).
- Keep the mod's name out of `FileDescription`; `ProductName` can carry it.

## Rejected

- Renaming the config file to `AnimCancelV1.toml` (the earlier workaround) removes the `Doina` family
  only. It costs users a file name that no longer matches the DLL.
- Imports are not the cause: the import table of AnimCancel is a subset of the one of a mod that has
  one detection.
- Removing code (menu, probes, weapon rule, the cancel logic itself) changed nothing.

## Open risks

- Results drift as engines add signatures; scan the exact file before relying on a number. A fresh
  scan also lacks the engines that have not finished.
