# Overlay start with mods that wrap the swap chain

Status: **Confirmed** in game on 2.7.2.0 with two third-party mods that wrap the swap chain (both loaded, `LoadDelay` 5000):
the overlay starts. Conventions: [research notes](../../../docs/research/README.md). Code: `vendor/hudhook/src/hooks/dx12.rs`
(`unwrap_dummy_swap_chain`, listed in `vendor/README.md`).

## Problem

With either mod the minimap never drew and the log stopped at `Overlay: hook: Present reached`, with no `first frame
drawn` (a report on Nexus, then reproduced). Without the mods the overlay starts.

## Cause

- hudhook gets the addresses it hooks (`Present`, `ExecuteCommandLists`...) from the vtables of a **dummy swap chain** it
  creates itself (640x480, a dummy window) through `IDXGIFactory::CreateSwapChain`.
- Both mods hook the factory's `CreateSwapChain` and `CreateSwapChainForHwnd` and hand back a **wrapper** object (their logs: `game
  swapchain 640x480 created, wrapped`): the dummy one too. Its `Present` is the wrapper's, in their DLL.
- The hook then sat on the wrapper's `Present`. From there hudhook looks for the command queue in the first 512 pointers of the swap
  chain object it was given, which is the wrapper, not DXGI's swap chain: it never finds it and never completes.

## Fix

After creating the dummy swap chain the module of its `Present` is checked; when it is not `dxgi.dll` a second dummy swap chain is
made with `CreateSwapChainForComposition`, which those mods do not wrap, and its vtable gives DXGI's own functions. The hook log
says so (`dummy swap chain wrapped by <module>`, then `using a composition swap chain instead`). Without a
wrapper nothing changes (the first swap chain is used).

## Open

- Another mod that wraps `CreateSwapChainForComposition` too would still fail (the log line `composition swap chain wrapped by
  <module> too` tells).
- The crash a reporter saw at `LoadDelay` 1000 and 3000 with these mods was not looked at.
