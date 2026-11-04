# The overlay is too bright on an HDR display

Status: **Confirmed** on one HDR setup (ERSS-FG with `IsHDR` and `Force10Bit`): `Auto` picks HDR10 and
the minimap is no longer burnt orange. Brightness against the game's own HUD was not compared with
screenshots. Conventions: [research notes](../../../docs/research/README.md). Code: `[Rendering] HDRMode`,
`vendor/hudhook` (see `vendor/README.md`), `ui::apply_hdr`.

## Question

With the swap chain in `DXGI_FORMAT_R10G10B10A2_UNORM` and ERSS-FG's HDR output, the overlay is brighter and
more saturated than the game's HUD (on a 4K screenshot: mean luma of the minimap area 59.8 against 41.8 for
the scene, saturation 178 against 148). Lowering `HDRBrightnessHUD` of ERSS-FG darkens the game's HUD too.

## Cause (inferred from the measurements and ERSS-FG's settings)

ImGui colours and icon texels are 8-bit sRGB; hudhook draws straight into the back buffer without a colour
conversion. On an SDR swap chain that is right. On an HDR10 swap chain (ST2084, Rec.2020) the same numbers
are read as PQ, so white 1.0 becomes 10,000 nit, and sRGB read as Rec.2020 is oversaturated. On scRGB
(linear FP16, 1.0 = 80 nit) it is also much brighter.

## Fix

hudhook has a fixed pixel shader, so a copy of hudhook 0.9.3 lives in `vendor/hudhook` with a changed DX12
pixel shader (`src/color.rs`): after `vertex colour * texel`, convert

- **HDR10**: sRGB to linear, Rec.709 to Rec.2020 matrix, multiply by `203 nit * HDRBrightness`, PQ encode;
- **scRGB**: sRGB to linear, multiply by `203 / 80 * HDRBrightness`.

203 nit is the HDR reference white (BT.2408), so the overlay's white matches the paper white of a standard
HUD. `HDRGamma` is an exponent applied to the linear value before encoding. Alpha blending is still done in
the encoded space, as in SDR.

| `HDRMode` | Meaning |
|---|---|
| `Auto` (default) | follows the colour space the game or another mod last set on the swap chain with `SetColorSpace1`: G2084 (HDR10) converts as `HDR10`, G10 Rec.709 (scRGB) as `scRGB`, anything else is not converted. It follows an HDR toggle at once. |
| `SDR` | no conversion, as before |
| `HDR10` / `scRGB` | force that type |

**How `Auto` learns the colour space.** DXGI cannot read a swap chain's colour space, so the copied hudhook
(`color.rs`) swaps the `SetColorSpace1` slot (38) pointer in the vtable of a temporary swap chain. It changes
data only and patches no code, so it does not conflict with the hooks of the Steam overlay or other mods. The
vtable lives in `dxgi.dll` and is shared, so every swap chain of that class goes through a function that
records the value and calls the original. Two limits: a call made **before** the hook is installed is not
seen (`Auto` stays `SDR` until the next call, or set `HDR10` by hand); and a call made on another swap chain
class (a mod's own proxy) is not seen, which the log shows.

The mode, brightness and gamma are not hard-coded into the shader: the renderer puts them in the third row of
the projection matrix (unused by the vertex shader, z = 0) and the pixel shader reads them every frame, so
changing the keys and pressing the reload key takes effect without a restart. The log has
`Overlay: colour output <type> (swap chain colour space N, set M time(s), HDRMode ...)` whenever it changes.

A mod that uses hudhook but never calls `color::set` stays `Sdr` and draws as before.

## Rejected path

Reading `IsHDR` from ERSS-FG's config file: its file and key names can be changed by its author.

## ERSS-FG's own HDR (static analysis of a dump of 4.14.1)

The DLL is packed, so it was dumped from the running game and analysed statically. RVAs are of 4.14.1 and
change per build. **Confirmed** = from code, **Inferred** = a guess.

- ERSS-FG is a ReShade add-on with an embedded FidelityFX-style swap chain (log lines `InitSwapChain`,
  `UpdateSwapChainColorSpace`). Its configuration is a global struct: `OverrideColorSpace` (0x312c33),
  `Force10Bit` (0x312c36), `IsHDR` (0x312c39). Confirmed.
- Its HDR checkbox (`IsHDR`) says the signal is HDR (its tooltip says Windows HDR must be on). Pressing it
  rebuilds the pipeline through 0xbc930 and calls 0x192750, which calls `SetColorSpace1(12)` (G2084 P2020, i.e.
  HDR10) on a global object (0x327268); unticking restores the original colour space. Confirmed.
- `SetColorSpace1(12)` at swap chain creation (0x16ed30) runs only when `OverrideColorSpace` is on. Confirmed.
- It converts the HDR signal for Frame Generation and draws the UI and HUD into its own swap chain
  (`HDRBrightnessHUD`). Confirmed that this exists; the formula was not decoded. The overlay draws just before
  the real `Present`, so it most likely lands **after** these passes. Inferred.
- **Consequence**: the object 0x327268 is ERSS-FG's own swap chain (a Streamline proxy or a FidelityFX swap
  chain), a different class from the one the `SetColorSpace1` hook sees, so no call is seen when its HDR box is
  toggled, and no general DXGI signal gives its HDR state (the format is always `R10G10B10A2` because of
  `Force10Bit`, Windows HDR stays off). The only way is to read its internal flag at a fixed RVA of one build,
  which is fragile, so it is **not done**: with ERSS-FG set `HDRMode = HDR10` by hand when its HDR box is on.

## Open risks

- ERSS-FG may apply its own output mapping after the overlay ("lifted by the output mapping" in a user
  report); then the 203 nit constant needs another factor, tuned with `HDRBrightness` and `HDRGamma`.
- Brightness and colour against the game's HUD were not compared with HDR screenshots.

## Related

Enabling DLSS-G crashed the default load order at `ERSS-FG.dll+0x13d87c` while ERSS-FG creates its Streamline
proxy for `IDXGIFactory`; delaying the DLL by about 20 seconds (`delay,20000` in the `.me3` file) fixed it. Not
reproduced here; same family as [questpath.md](questpath.md) (hudhook's DXGI and DX12 hooks meeting other
mods).
