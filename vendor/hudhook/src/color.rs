//! Local addition to hudhook (not in upstream): colour output of the DX12 overlay.
//!
//! ImGui colours and textures are sRGB values. On an SDR swap chain that is right as is. On
//! an HDR one (HDR10: PQ / ST2084 in Rec.2020, or scRGB: linear Rec.709, 1.0 = 80 nits) the same
//! numbers are read as PQ or linear light, so the overlay comes out far too bright and
//! oversaturated. The pixel shader converts every pixel it draws, driven by [`Settings`].
//!
//! Both the settings and the mode can change while the game runs, nothing is baked into the
//! shader. In [`Mode::Auto`] the mode follows the colour space the game or another mod last
//! gave the swap chain with `IDXGISwapChain3::SetColorSpace1`: a pointer in the shared swap
//! chain vtable is replaced (data only, no code patch, so it cannot clash with other hooks)
//! by a function that notes the value and calls the original. A call made before the patch
//! was installed is not seen, then `Auto` stays on SDR until the next call.

use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU32, AtomicUsize, Ordering};

use parking_lot::Mutex;

use windows::core::{Interface, HRESULT};
use windows::Win32::Graphics::Dxgi::Common::{
    DXGI_COLOR_SPACE_RGB_FULL_G10_NONE_P709, DXGI_COLOR_SPACE_RGB_FULL_G2084_NONE_P2020,
    DXGI_COLOR_SPACE_RGB_STUDIO_G2084_NONE_P2020,
};
use windows::Win32::Graphics::Dxgi::IDXGISwapChain3;
use windows::Win32::System::Memory::{VirtualProtect, PAGE_READWRITE};

/// What the host asks for.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Mode {
    /// No conversion (the upstream behaviour).
    Sdr,
    /// Follow the colour space set on the swap chain (HDR10 or scRGB convert, anything else not).
    Auto,
    /// Force PQ (ST2084) output in Rec.2020.
    Hdr10,
    /// Force linear scRGB output (1.0 = 80 nits).
    ScRgb,
}

/// The host's choice, see [`set`].
#[derive(Clone, Copy, Debug)]
pub struct Settings {
    /// Which conversion to use.
    pub mode: Mode,
    /// Multiplier of the reference white (203 nits): 1.0 gives the overlay the brightness of
    /// the HDR reference white.
    pub brightness: f32,
    /// Exponent applied to the linear colour before encoding; 1.0 leaves it as is.
    pub gamma: f32,
}

/// The conversion in use right now.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Applied {
    /// No conversion.
    None,
    /// PQ in Rec.2020.
    Hdr10,
    /// Linear scRGB.
    ScRgb,
}

/// What the overlay does now and what it knows, for the host's log.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Status {
    /// The conversion in use.
    pub applied: Applied,
    /// The last colour space (`DXGI_COLOR_SPACE_TYPE` value) seen in `SetColorSpace1`, or -1.
    pub color_space: i32,
    /// How many `SetColorSpace1` calls were seen.
    pub calls: u32,
}

static SETTINGS: Mutex<Settings> = Mutex::new(Settings { mode: Mode::Sdr, brightness: 1.0, gamma: 1.0 });
static COLOR_SPACE: AtomicI32 = AtomicI32::new(-1);
static CALLS: AtomicU32 = AtomicU32::new(0);
static ORIGINAL: AtomicUsize = AtomicUsize::new(0);
static PATCHED: AtomicBool = AtomicBool::new(false);

/// Chooses the conversion; takes effect on the next frame.
pub fn set(settings: Settings) {
    *SETTINGS.lock() = settings;
}

fn applied_for(mode: Mode, color_space: i32) -> Applied {
    match mode {
        Mode::Sdr => Applied::None,
        Mode::Hdr10 => Applied::Hdr10,
        Mode::ScRgb => Applied::ScRgb,
        Mode::Auto => {
            if color_space == DXGI_COLOR_SPACE_RGB_FULL_G2084_NONE_P2020.0
                || color_space == DXGI_COLOR_SPACE_RGB_STUDIO_G2084_NONE_P2020.0
            {
                Applied::Hdr10
            } else if color_space == DXGI_COLOR_SPACE_RGB_FULL_G10_NONE_P709.0 {
                Applied::ScRgb
            } else {
                Applied::None
            }
        },
    }
}

/// What the overlay does now.
pub fn status() -> Status {
    let color_space = COLOR_SPACE.load(Ordering::Relaxed);
    Status {
        applied: applied_for(SETTINGS.lock().mode, color_space),
        color_space,
        calls: CALLS.load(Ordering::Relaxed),
    }
}

/// The four numbers the pixel shader reads each frame: mode (0 none, 1 HDR10, 2 scRGB), the
/// brightness multiplier, the gamma exponent and one spare.
pub(crate) fn frame_params() -> [f32; 4] {
    let settings = *SETTINGS.lock();
    let mode = match applied_for(settings.mode, COLOR_SPACE.load(Ordering::Relaxed)) {
        Applied::None => 0.0,
        Applied::Hdr10 => 1.0,
        Applied::ScRgb => 2.0,
    };
    [mode, settings.brightness, settings.gamma, 0.0]
}

/// Index of `SetColorSpace1` in the `IDXGISwapChain3` vtable (IUnknown 3, IDXGIObject 4,
/// IDXGIDeviceSubObject 1, IDXGISwapChain 10, 1: 11, 2: 7, then GetCurrentBackBufferIndex,
/// CheckColorSpaceSupport, SetColorSpace1).
const SET_COLOR_SPACE1_SLOT: usize = 38;

type SetColorSpace1Fn = unsafe extern "system" fn(this: *mut c_void, color_space: i32) -> HRESULT;

unsafe extern "system" fn set_color_space1(this: *mut c_void, color_space: i32) -> HRESULT {
    // SAFETY: `ORIGINAL` holds the function the slot had before the patch, set before the patch.
    let original: SetColorSpace1Fn = unsafe { std::mem::transmute(ORIGINAL.load(Ordering::Acquire)) };
    let result = unsafe { original(this, color_space) };
    if result.is_ok() {
        COLOR_SPACE.store(color_space, Ordering::Relaxed);
        CALLS.fetch_add(1, Ordering::Relaxed);
    }
    result
}

/// Replaces the `SetColorSpace1` slot in the vtable of `swap_chain` (shared by every swap
/// chain of its class) with a function that records the colour space. Once only.
pub(crate) fn patch_set_color_space1(swap_chain: &IDXGISwapChain3) {
    if PATCHED.swap(true, Ordering::AcqRel) {
        return;
    }
    // SAFETY: a COM object starts with a pointer to its vtable; the slot index is that of
    // `IDXGISwapChain3::SetColorSpace1`, the page is made writable while the pointer is swapped.
    unsafe {
        let vtable = *(swap_chain.as_raw() as *const *mut usize);
        let slot = vtable.add(SET_COLOR_SPACE1_SLOT);
        let original = *slot;
        if original == set_color_space1 as SetColorSpace1Fn as usize {
            return;
        }
        let mut old = Default::default();
        if VirtualProtect(slot as *const c_void, std::mem::size_of::<usize>(), PAGE_READWRITE, &mut old).is_err() {
            return;
        }
        ORIGINAL.store(original, Ordering::Release);
        *slot = set_color_space1 as SetColorSpace1Fn as usize;
        let _ = VirtualProtect(slot as *const c_void, std::mem::size_of::<usize>(), old, &mut old);
    }
}

/// The pixel shader (HLSL), same for every mode; public so a host can test-compile it.
///
/// It reads the mode, brightness and gamma from the root constants (register b0): the vertex
/// shader's projection matrix has four unused numbers in its third row, the renderer puts
/// [`frame_params`] there.
pub fn pixel_shader() -> String {
    r#"
    cbuffer pixelBuffer : register(b0) {
      float4 row0;
      float4 row1;
      float4 params;
      float4 row3;
    };

    struct PS_INPUT {
      float4 pos: SV_POSITION;
      float4 col: COLOR0;
      float2 uv: TEXCOORD0;
    };

    SamplerState sampler0: register(s0);
    Texture2D texture0: register(t0);

    float3 srgb_to_linear(float3 c) {
      return (c <= 0.04045) ? c / 12.92 : pow((c + 0.055) / 1.055, 2.4);
    }

    float4 main(PS_INPUT input): SV_Target {
      float4 c = input.col * texture0.Sample(sampler0, input.uv);
      if (params.x > 0.5) {
        float3 lin = pow(max(srgb_to_linear(c.rgb), 0.0), params.z);
        if (params.x < 1.5) {
          // HDR10: Rec.709 to Rec.2020, scale to nits, PQ (ST2084) encode.
          float3 rec2020 = float3(
            dot(lin, float3(0.627404, 0.329283, 0.043313)),
            dot(lin, float3(0.069097, 0.919540, 0.011362)),
            dot(lin, float3(0.016391, 0.088013, 0.895595)));
          float3 p = pow(saturate(rec2020 * (203.0 * params.y / 10000.0)), 0.1593017578125);
          c.rgb = pow((0.8359375 + 18.8515625 * p) / (1.0 + 18.6875 * p), 78.84375);
        } else {
          // scRGB: linear Rec.709, 1.0 is 80 nits.
          c.rgb = lin * (203.0 * params.y / 80.0);
        }
      }
      return c;
    }"#
        .to_string()
}

/// Lines about what the DX12 hook sees, for the host's log (hudhook has no logger the host can
/// read). Each distinct line is kept once.
static DIAG: Mutex<(Vec<String>, Vec<String>)> = Mutex::new((Vec::new(), Vec::new()));

pub(crate) fn diag(message: String) {
    let mut diag = DIAG.lock();
    if diag.1.len() < 64 && !diag.1.contains(&message) {
        diag.1.push(message.clone());
        diag.0.push(message);
    }
}

/// The lines the hook added since the last call.
pub fn take_diag() -> Vec<String> {
    std::mem::take(&mut DIAG.lock().0)
}
