//! Local addition to hudhook (not in upstream): colour output of the DX12 overlay.
//!
//! ImGui colours and textures are sRGB values. On an SDR swap chain that is right as is. On
//! an HDR one (HDR10 `R10G10B10A2` in the ST2084 colour space, or scRGB `R16G16B16A16_FLOAT`)
//! the same numbers are read as PQ or linear light, so the overlay comes out far too bright
//! and oversaturated. [`set`] lets the host choose a conversion that the pixel shader applies
//! to every pixel it draws.
//!
//! Set it before the hook is applied; the pipeline reads it when it is built (first frame,
//! and again whenever the game recreates its swap chain).

use parking_lot::Mutex;

use windows::core::Interface;
use windows::Win32::Graphics::Dxgi::Common::{
    DXGI_COLOR_SPACE_RGB_FULL_G2084_NONE_P2020, DXGI_FORMAT, DXGI_FORMAT_R10G10B10A2_UNORM,
    DXGI_FORMAT_R16G16B16A16_FLOAT,
};
use windows::Win32::Graphics::Dxgi::{IDXGIOutput6, IDXGISwapChain};

/// What the host asks for.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Mode {
    /// No conversion (the upstream behaviour).
    Sdr,
    /// Convert when the swap chain is 10-bit or half-float and the monitor is in HDR.
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

/// What the pipeline ended up using, for the host's log.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Applied {
    /// No conversion.
    None,
    /// PQ in Rec.2020.
    Hdr10,
    /// Linear scRGB.
    ScRgb,
}

static SETTINGS: Mutex<Settings> = Mutex::new(Settings { mode: Mode::Sdr, brightness: 1.0, gamma: 1.0 });
static APPLIED: Mutex<(Applied, String)> = Mutex::new((Applied::None, String::new()));

/// Chooses the conversion for pipelines built from now on.
pub fn set(settings: Settings) {
    *SETTINGS.lock() = settings;
}

/// What the last pipeline used, with the facts it decided on (format, monitor colour space).
pub fn applied() -> (Applied, String) {
    APPLIED.lock().clone()
}

/// Whether the monitor the swap chain is on is in HDR (ST2084). `None` when it cannot be
/// told.
fn monitor_is_hdr(swap_chain: &IDXGISwapChain) -> Option<bool> {
    let output = unsafe { swap_chain.GetContainingOutput() }.ok()?;
    let desc = unsafe { output.cast::<IDXGIOutput6>().ok()?.GetDesc1() }.ok()?;
    Some(desc.ColorSpace == DXGI_COLOR_SPACE_RGB_FULL_G2084_NONE_P2020)
}

/// Decides the conversion for this swap chain; called when the pipeline is built.
pub(crate) fn resolve(swap_chain: &IDXGISwapChain, format: DXGI_FORMAT) -> (Applied, f32, f32) {
    let settings = *SETTINGS.lock();
    let monitor = monitor_is_hdr(swap_chain);
    let applied = match settings.mode {
        Mode::Sdr => Applied::None,
        Mode::Hdr10 => Applied::Hdr10,
        Mode::ScRgb => Applied::ScRgb,
        Mode::Auto => match (format, monitor) {
            (DXGI_FORMAT_R10G10B10A2_UNORM, Some(true)) => Applied::Hdr10,
            (DXGI_FORMAT_R16G16B16A16_FLOAT, Some(true)) => Applied::ScRgb,
            _ => Applied::None,
        },
    };
    *APPLIED.lock() = (
        applied,
        format!("mode {:?}, swap chain format {:?}, monitor HDR {:?}", settings.mode, format, monitor),
    );
    (applied, settings.brightness, settings.gamma)
}

/// The pixel shader (HLSL) for the chosen conversion; public so a host can test-compile it.
pub fn pixel_shader(applied: Applied, brightness: f32, gamma: f32) -> String {
    let body = match applied {
        Applied::None => return PS_PLAIN.to_string(),
        // sRGB to linear Rec.709, to Rec.2020, scale to nits, PQ (ST2084) encode.
        Applied::Hdr10 => {
            "float3 lin = srgb_to_linear(c.rgb);\n\
             lin = pow(max(lin, 0.0), GAMMA);\n\
             float3 rec2020 = float3(\n\
               dot(lin, float3(0.627404, 0.329283, 0.043313)),\n\
               dot(lin, float3(0.069097, 0.919540, 0.011362)),\n\
               dot(lin, float3(0.016391, 0.088013, 0.895595)));\n\
             float3 p = pow(saturate(rec2020 * (203.0 * BRIGHTNESS / 10000.0)), 0.1593017578125);\n\
             c.rgb = pow((0.8359375 + 18.8515625 * p) / (1.0 + 18.6875 * p), 78.84375);"
        },
        // scRGB is linear Rec.709 with 1.0 at 80 nits.
        Applied::ScRgb => {
            "float3 lin = srgb_to_linear(c.rgb);\n\
             c.rgb = pow(max(lin, 0.0), GAMMA) * (203.0 * BRIGHTNESS / 80.0);"
        },
    };
    format!(
        "#define BRIGHTNESS {brightness:.6}\n#define GAMMA {gamma:.6}\n{PS_HEAD}\n\
         float3 srgb_to_linear(float3 c) {{\n\
           return (c <= 0.04045) ? c / 12.92 : pow((c + 0.055) / 1.055, 2.4);\n\
         }}\n\n\
         float4 main(PS_INPUT input): SV_Target {{\n\
           float4 c = input.col * texture0.Sample(sampler0, input.uv);\n\
           {body}\n\
           return c;\n\
         }}"
    )
}

const PS_HEAD: &str = r#"
struct PS_INPUT {
  float4 pos: SV_POSITION;
  float4 col: COLOR0;
  float2 uv: TEXCOORD0;
};

SamplerState sampler0: register(s0);
Texture2D texture0: register(t0);"#;

const PS_PLAIN: &str = r#"
    struct PS_INPUT {
      float4 pos: SV_POSITION;
      float4 col: COLOR0;
      float2 uv: TEXCOORD0;
    };

    SamplerState sampler0: register(s0);
    Texture2D texture0: register(t0);

    float4 main(PS_INPUT input): SV_Target {
      float4 out_col = input.col * texture0.Sample(sampler0, input.uv);
      return out_col;
    }"#;
