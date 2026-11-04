# Research notes

How the mods' reverse-engineering findings are written down. A research page should let a
reader reproduce a finding, judge how far to trust it, and continue after a game update.
Mod-specific pages live in `mods/<name>/docs/`; this page defines the conventions they share.

Nothing here is a public API. Addresses, layouts and analytical names are private details of
the mods and may change with any game build.

## Evidence labels

Every lasting claim carries one of these labels:

- **Confirmed**: reproduced from a static artifact, a controlled run in game, or both, with the
  tested build named.
- **Inferred**: several observations agree, but a type, meaning, owner or thread has not been
  observed directly.
- **Hypothesis**: a specific explanation or test that still needs evidence.
- **Rejected**: a controlled result disproved it. Keep rejected paths on the page so nobody
  repeats them.

Confirmed means confirmed on the named build. It does not mean stable across game versions.

## Reference builds

Findings are recorded against one of these executables. RVAs are only valid for the build
they were observed on.

| Version | Size (bytes) | SHA-256 | PE timestamp | Image size |
|---|---:|---|---|---|
| 2.6.2.0 | 86998096 | `34102B1C08BB5F769A724427A6F70FE29B3B732C31CF73693F861C48D3492DDB` | `0x69E21187` | `0x5E01800` |
| 2.7.0.0 | 87024720 | `D1A84083C6C7C7902162FF098F7D86812839AA6B3575959398857E539C488134` | `0x69E9C9B9` | `0x5E09600` |
| 2.7.1.0 | 87042128 | `1A3547101327F65D0C76DA2F9190AC0AA66871EA42BAE2AECC61E11A8B597891` | `0x6A96B418` | `0x5E0DA00` |
| 2.7.2.0 | 86973008 | `F9118FEA2CFD8FA3901E8060BFADB1552CB2471771DED8A5B662C4E298FFB328` | `0x6AABF7E7` | `0x5DFD600` |

Get the identity of a local executable with:

```powershell
(Get-Item $gameExe).VersionInfo.ProductVersion
Get-FileHash -Algorithm SHA256 $gameExe
```

## Addresses are evidence, not interfaces

| Record | Meaning | Use |
|---|---|---|
| Runtime address | Module base plus an offset, different on every launch (ASLR). | A temporary trace only. Subtract the base before writing it down. |
| RVA | Runtime address minus the module base, seen on one build. | Navigate that exact build. Never copy it to another build. |
| AOB pattern | A byte pattern with wildcards for relocation-sensitive bytes. | A candidate resolver. Valid only when it matches exactly once on every supported build. |

A pattern that matches twice is unresolved, not close enough. Wildcard `call rel32`,
RIP-relative displacements and absolute addresses. Do not wildcard stack layout, field offsets,
branch structure or constants just to force a match. Check uniqueness across all builds with
`scripts/ida/run.ps1 aob -Version all`.

## Experiment card

Before probing, write down one question and how the test is bounded:

```text
question:
game build:
mod build:
entry state:
exact player actions:
control case:
expected observation:
time limit and capture limit:
cleanup:
result and evidence label:
```

A negative result is useful when the actions and limits that produced it are recorded.

When testing, disable other DLLs that could produce the same behavior. A result seen with an
unrelated mod active is not evidence.

## Page structure

A mod's research page for one feature follows this shape. Leave out sections that have nothing
to say.

1. **Status**: one line with the evidence label and the build tested.
2. **Question**: what the page set out to find out.
3. **Mechanism**: how it works, with the key tables (IDs, offsets, hook points).
4. **Derivation**: how each address or pattern was found and on which builds it is unique.
5. **Verification**: what was tested in game, how, and the result.
6. **Rejected paths and failures**: what did not work and why, including crashes and their cause.
7. **Open risks**: what is still unverified.

Write conclusions, not diaries: no dates, no chat history, no names of people or other mods.
Describe a third-party mod as "a third-party mod" and say what it does.

## What belongs in the repository

Keep these local, outside git (the `dumps/` and `.local/` folders are ignored):

- `eldenring.exe` and every copy or patched copy of it;
- IDA or Ghidra databases and bulk decompiler or disassembly output;
- extracted game assets and raw game strings;
- probe logs, memory dumps and caches.

The repository may hold original scripts, short pseudocode, derived patterns, build
identities, tables of IDs, and conclusions about how the game behaves.
