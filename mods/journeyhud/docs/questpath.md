# Compatibility with QuestPath

Status: **Open**. A way to avoid the crash on ModEngine3 is known and works; the cause and a fix inside the mod
were not verified. Conventions: [research notes](../../../docs/research/README.md). Code: `ui::install`,
`General.LoadDelay`.

## Question

JourneyHUD together with QuestPath exited the game at start-up on ModEngine3 (reported by a user). Why, and how
can it be avoided?

## Observations

- ME2: runs normally. ME3: crash at load; the JourneyHUD log stops before `Overlay: DX12 hooks applied`.
- `Exception: access violation, read 0xFFFFFFFFFFFFFFFF, at QuestPath.dll+0x18108` appears on both ME2 and ME3,
  on QuestPath's thread right after its `flag-manager search`: QuestPath probes memory and catches the fault
  itself, harmless.
- `Exception: stack overflow at GameOverlayRenderer64.dll` (the Steam overlay) only on ME3: a recursion in the
  Steam hook, dying before hudhook finishes.
- **Reproduced locally**: an ME3 profile with only JourneyHUD, QuestPath and a window-resize mod (no delay, no
  ERSS-FG) crashes every time: `stack overflow at GameOverlayRenderer64.dll+0xDE197`, the recursion inside the
  Steam overlay. The `outermost:` line of the crash handler shows the thread that overflowed is one created by
  JourneyHUD (`JourneyHUD.dll+0x133235`, then `+0x73677`, `+0x15059`, `+0x1AC98B`, `+0x1AC722`, then `ntdll`),
  and the log lacks `Overlay: DX12 hooks applied`: it is the thread installing the DX12 hooks. The RVAs were not
  resolved to function names (no PDB tool). The same build runs on ME2.

## Cause (**Hypothesis**)

hudhook creates a temporary swap chain to read the DX12 vtable while QuestPath hooks DXGI creation
(`DXGI creation hooks installed`) and the Steam overlay hooks it too: three layers of hooks calling each other.
Each mod alone is fine; ME3 loads the DLLs almost simultaneously, so the timing coincides.

## Avoiding it

- In the `.me3` file give JourneyHUD a delay (tried, the crash is gone):

```toml
[[natives]]
path = "dll/JourneyHUD.dll"
initializer.delay.ms = 3000
```

- Or use `General.LoadDelay` (milliseconds, default 5000) in `JourneyHUD.toml`: the mod then waits before hooking the swap
  chain on its own thread, so the player does not have to edit the `.me3` file. Not tried with ME2, ME3 and
  QuestPath together in the final form.

## Rejected / untried

An approach that wraps the swap chain instead of hooking `Present` (as another overlay mod does) avoids this
class of clash in principle; it was studied statically only and not built.
