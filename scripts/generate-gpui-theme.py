"""Generate gpui/src/theme.rs from the app's design tokens in globals.css."""
import re
import sys
from pathlib import Path

root = Path(__file__).resolve().parent.parent
css = (root / 'src/app/globals.css').read_text(encoding='utf-8')
block = re.search(r':root\s*\{(.*?)\}', css, re.S).group(1)
tokens = {}
for line in block.splitlines():
    m = re.match(r'\s*--([\w-]+):\s*([^;]+);', line)
    if m:
        tokens[m.group(1)] = m.group(2).strip()

def hex_of(name):
    value = tokens[name]
    assert value.startswith('#'), (name, value)
    return int(value[1:], 16)

def px_of(name):
    value = tokens[name]
    return float(value.replace('px', ''))

colors = {
    'BACKGROUND': 'background',
    'FOREGROUND': 'foreground',
    'SURFACE_0': 'surface-0',
    'SURFACE_1': 'surface-1',
    'SURFACE_2': 'surface-2',
    'BORDER_SUBTLE': 'border-subtle',
    'BORDER_STRONG': 'border-strong',
    'TEXT_STRONG': 'text-strong',
    'TEXT_MUTED': 'text-muted',
    'INDICATOR': 'indicator',
    'ACCENT': 'accent',
    'PRIMARY_BUTTON_SURFACE': 'primary-button-surface',
    'ACCENT_HOVER': 'accent-hover',
    'ACCENT_CONTRAST': 'accent-contrast',
    'ACCENT_SECONDARY': 'accent-secondary',
    'SECONDARY_BUTTON_SURFACE': 'secondary-button-surface',
    'ACCENT_SECONDARY_HOVER': 'accent-secondary-hover',
    'ACCENT_SECONDARY_CONTRAST': 'accent-secondary-contrast',
    'TOPBAR_ACCENT': 'topbar-accent',
    'DANGER': 'danger',
    'SUCCESS': 'success',
}

lines = [
    '//! Design tokens, generated from `src/app/globals.css` by',
    '//! `scripts/generate-gpui-theme.py`. Do not edit by hand.',
    '',
    '#![allow(dead_code)]',
    '',
]
for const, token in colors.items():
    lines.append(f'pub const {const}: u32 = 0x{hex_of(token):06x};')
lines.append('')
for const, token in {
    'RADIUS_SM': 'radius-sm',
    'RADIUS_MD': 'radius-md',
    'RADIUS_LG': 'radius-lg',
    'SPACE_1': 'space-1',
    'SPACE_2': 'space-2',
    'SPACE_3': 'space-3',
    'SPACE_4': 'space-4',
    'SPACE_5': 'space-5',
    'SPACE_6': 'space-6',
    'FONT_XS': 'font-size-xs',
    'FONT_SM': 'font-size-sm',
    'FONT_MD': 'font-size-md',
    'FONT_LG': 'font-size-lg',
    'FONT_XL': 'font-size-xl',
    'TOPBAR_HEIGHT': 'topbar-height',
}.items():
    value = px_of(token)
    literal = f'{value:g}'
    if '.' not in literal:
        literal += '.0'
    lines.append(f'pub const {const}: f32 = {literal};')
lines.append(f'pub const LINE_HEIGHT_TIGHT: f32 = {float(tokens["line-height-tight"]):g};')
lines.append(f'pub const LINE_HEIGHT_NORMAL: f32 = {float(tokens["line-height-normal"]):g};')
lines.append('')
lines.append('''/// `color-mix(in srgb, a, b)` in sRGB, with `weight_a` the share of `a`.
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
''')
out = root / 'gpui/src/theme.rs'
out.write_text('\n'.join(lines), encoding='utf-8')
print(f'wrote {out}')
