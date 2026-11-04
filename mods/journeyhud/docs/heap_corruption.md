# Game crash with heap corruption (0xC0000374)

Status: **Open**. Seen twice; both stacks hold only `eldenring.exe` and `ntdll.dll` (no frame of the mod, and
the crash handler stops at the game's first frame), so a link to the mod is not shown. No reproduction.
Conventions: [research notes](../../../docs/research/README.md).

## Observations

- **First**: the game was freeing a `MicrosoftDiskFileOperator`.
- **Second** (ModEngine2, ERSS-FG off; loaded: ERNativeUI, JourneyHUD, SpeedMultiplier, FasterRevival, a camera
  fix, an ultrawide fix, Toolbox): `heap corruption at ntdll.dll+0x112FC5`, `rbx = 0xC0000374`
  (`STATUS_HEAP_CORRUPTION`), stack `ntdll.dll+0x112FC5 < ntdll.dll+0x95A65 < ntdll.dll+0xAE02 < ntdll.dll+0x12338A
  < ntdll.dll+0x2754C < eldenring.exe+0x254E298 < eldenring.exe+0x25234FE < eldenring.exe+0x1F65B6F <
  eldenring.exe+0x1EDFAC6 < eldenring.exe+0x1EDFD22 < eldenring.exe+0x242AE6F < eldenring.exe+0x242AEA4`. The
  original log was overwritten by later runs.

## What is known

- `0xC0000374` is detected at a free or an allocation; the writer (an overrun, a double free) usually ran
  earlier, so the frame that detects it is not the suspect.
- A different engine crash (an access violation reading `0x30`, not heap corruption) was also seen on the same
  machine before JourneyHUD existed: see [map.md](map.md), "When reading starts".

## Next

When it happens again keep the log (do not start the game again at once) and note which mods are loaded; try
JourneyHUD alone. An untried idea: enable Page Heap (`gflags /p /enable eldenring.exe /full`) to catch the
overrun where it happens (offline only, without Easy Anti-Cheat).
