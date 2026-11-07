//! Design tokens, generated from `src/app/globals.css` by
//! `scripts/generate-gpui-theme.py`. Do not edit by hand.

#![allow(dead_code)]

pub const BACKGROUND: u32 = 0x0b0f14;
pub const FOREGROUND: u32 = 0xe6ebf2;
pub const SURFACE_0: u32 = 0x111216;
pub const SURFACE_1: u32 = 0x1a1b21;
pub const SURFACE_2: u32 = 0x23252e;
pub const BORDER_SUBTLE: u32 = 0x272a33;
pub const BORDER_STRONG: u32 = 0x353944;
pub const TEXT_STRONG: u32 = 0xf8f8fb;
pub const TEXT_MUTED: u32 = 0xc3c7cf;
pub const INDICATOR: u32 = 0xc3c7cf;
pub const ACCENT: u32 = 0xec2a77;
pub const PRIMARY_BUTTON_SURFACE: u32 = 0xc11f61;
pub const ACCENT_HOVER: u32 = 0xd81d67;
pub const ACCENT_CONTRAST: u32 = 0xfff6ff;
pub const ACCENT_SECONDARY: u32 = 0xbaf72e;
pub const SECONDARY_BUTTON_SURFACE: u32 = 0xbdf435;
pub const ACCENT_SECONDARY_HOVER: u32 = 0xa6df29;
pub const ACCENT_SECONDARY_CONTRAST: u32 = 0x182106;
pub const TOPBAR_ACCENT: u32 = 0xec2a77;
pub const DANGER: u32 = 0xe45454;
pub const SUCCESS: u32 = 0x2eb67d;

pub const RADIUS_SM: f32 = 6.0;
pub const RADIUS_MD: f32 = 10.0;
pub const RADIUS_LG: f32 = 14.0;
pub const SPACE_1: f32 = 4.0;
pub const SPACE_2: f32 = 8.0;
pub const SPACE_3: f32 = 12.0;
pub const SPACE_4: f32 = 16.0;
pub const SPACE_5: f32 = 20.0;
pub const SPACE_6: f32 = 24.0;
pub const FONT_XS: f32 = 12.0;
pub const FONT_SM: f32 = 13.0;
pub const FONT_MD: f32 = 14.0;
pub const FONT_LG: f32 = 17.0;
pub const FONT_XL: f32 = 22.0;
pub const TOPBAR_HEIGHT: f32 = 52.0;

// Tailwind v4 utility radii (globals.css only re-declares colours/fonts,
// so `rounded-*` keeps the framework defaults).
pub const ROUNDED_SM: f32 = 4.0;
pub const ROUNDED_MD: f32 = 6.0;
pub const ROUNDED_LG: f32 = 8.0;
pub const ROUNDED_XL: f32 = 12.0;

pub const LINE_HEIGHT_TIGHT: f32 = 1.2;
pub const LINE_HEIGHT_NORMAL: f32 = 1.45;

/// `color-mix(in srgb, a, b)` in sRGB, with `weight_a` the share of `a`.
pub fn mix(a: u32, b: u32, weight_a: f32) -> u32 {
    let channel = |shift: u32| {
        let av = ((a >> shift) & 0xff) as f32;
        let bv = ((b >> shift) & 0xff) as f32;
        (av * weight_a + bv * (1.0 - weight_a)).round() as u32
    };
    (channel(16) << 16) | (channel(8) << 8) | channel(0)
}

/// `color-mix(in srgb, var(--accent-secondary), var(--border-subtle) 42%)`.
pub const ACCENT_SECONDARY_ACTION_BORDER: u32 = 0;
/// `color-mix(in srgb, var(--accent-secondary), var(--surface-1) 92%)`.
pub const ACCENT_SECONDARY_ACTION_BG_92: u32 = 0;
/// `color-mix(in srgb, var(--accent-secondary), var(--surface-1) 90%)`.
pub const ACCENT_SECONDARY_ACTION_BG_90: u32 = 0;
