# vendor/

## hudhook (0.9.3)

Copy of <https://github.com/veeenu/hudhook> 0.9.3 (license in `hudhook/LICENSE`) without its
examples and tests, used through `[patch.crates-io]` in the root `Cargo.toml` by every mod
that depends on `hudhook`.

One local change, everything else is upstream:

- `src/color.rs` (new): `hudhook::color::set` chooses how the DX12 pixel shader converts the
  overlay's sRGB colours for an HDR swap chain (`Sdr` = upstream behaviour, `Auto`, `Hdr10`,
  `ScRgb`), `applied()` tells what a pipeline ended up using.
- `src/renderer/backend/dx12.rs`: the pixel shader comes from `color::pixel_shader`.
- `src/hooks/dx12.rs`: `init_pipeline` calls `color::resolve` with the game's swap chain.
- `src/hooks/dx12.rs` (same file): the hook logs the swap chains, queues and the module of the dummy swap
  chain's `Present` it sees through `color::diag`; `get_target_addrs` checks that `Present` is in `dxgi.dll`.
  A mod that wraps the game's swap chains (some third-party mods hook the factory) wraps the dummy
  one too, so the hook would land on the wrapper and never find the queue in its memory; then a composition
  swap chain, which those mods do not wrap, gives the real addresses (`unwrap_dummy_swap_chain`).
- `src/lib.rs`: `pub mod color;`.

Default is `Sdr`, so a mod that never calls `set` renders exactly as before. Why: see
`mods/journeyhud/docs/hdr.md`.

To update hudhook: copy the new version over, then redo the changes above.
