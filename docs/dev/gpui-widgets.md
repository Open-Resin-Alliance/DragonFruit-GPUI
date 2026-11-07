# GPUI widgets

`gpui/crates/gpui-ui` is the port's only widget library. A panel decides *what*
it contains; this crate decides *how* a card, a button, a control or a dialog
looks. The React tree this port mirrors re-implements the same card header and
button class in dozens of components; the port must not inherit that.

## The rule

- **A shared element is defined once, in `gpui-ui`.** If a second module needs
  the same card header, number row, segmented control or dialog shell, the
  widget moves into `gpui-ui` and both call it. Copying the recipe into a
  feature module is the defect.
- **Feature modules compose.** They pass content, variants and values; they do
  not re-derive radii, shadows or hover fills.
- **A local helper is a bug waiting to drift.** The port reached 17 independent
  `.ui-button` constructions with horizontal padding at 6, 8, 10, 11, 12, 14 and
  16 px before this rule was written down; the same classes render differently
  in five panels today.

## Public surface

`gpui_ui::ui` re-exports everything:

| Module | Carries |
|---|---|
| `button` | `ButtonVariant`, `ButtonSize`, `button`, `button_disabled`, **`ButtonSpec`** + `button_with`/`button_with_content` |
| `icon` | `IconTone`, `icon_chip*`, `icon_button*` |
| `card` | `card`, `card_header`, `card_header_with_right`, **`card_header_with`**, **`sub_card`/`sub_card_with`** |
| `layout` | `setting_row`, `panel_collapse_toggle`, `fade_text`, **`badge`/`badge_plain`** |
| `controls` | `toggle*`, `segmented_control` |
| `number_field` | `number_stepper`, `scrollable_number_field`, `compact_number_field` |
| `select`, `select_menu`, `menu`, `tooltip`, `dialog`, `feedback`, `input`, `color` | as named |

`gpui_ui::panel_fields` keeps the field builders that predate the split; new
work belongs in `widgets/`, not there.

## Using `ButtonSpec`

Every panel button in the React tree is `.ui-button` plus one or two utilities.
`ButtonSpec` states only the deviation:

```rust
use gpui_ui::ui::{self, ButtonSpec, ButtonSize, ButtonVariant};

let reset = ui::button_with(
    "reset",
    "Reset",
    &ButtonSpec::new(ButtonVariant::Secondary, ButtonSize::Sm)
        .padding_x(10.0)
        .font_size(12.0),
);

let save = ui::button_with(
    "save",
    "Save",
    &ButtonSpec::new(ButtonVariant::TintedAccent, ButtonSize::Md)
        .full()
        .icon("check", 14.0, theme::ACCENT),
);

let disabled = ui::button_with(
    "export",
    "Export",
    &ButtonSpec::new(ButtonVariant::Secondary, ButtonSize::Sm).disabled(),
);
```

`.disabled()` applies `.ui-button:disabled` — 82 % text-muted over the dimmed
surface. A disabled button never gets a hand-written colour.

## Cards

`ui::card()` is the panel shell (`rounded-lg`, 14 px, `--surface-0`).
`ui::sub_card()` is the `rounded-md border p-2` block inside it (10 px), and
`ui::sub_card_with(Some((tone, share)), pad)` is the accent-tinted variant. A
panel that needs a different radius is asking for a new widget, not a local
`.rounded(px(..))`.

`ui::card_header_with(title, leading, right)` is the only header: `leading` is
the icon block, `right` the absolutely positioned action slot.

## Constraints

- Radii are `gpui_core::theme::ROUNDED_*` (sm 6, md 10, lg 14, xl 12 — from
  `globals.css`). Never declare a local `ROUNDED_*`/`RADIUS_*`; it shadows the
  theme through glob imports — that is how `rounded-lg` painted 8 px for a while.
- Colours are `gpui_core::theme` tokens. A literal is only acceptable when the
  source is a literal that has no token, and then it carries a citation.
- `gpui-ui` never depends on the binary. Anything that needs the entity stays in
  `gpui/src/**` and takes content as an argument.
- New widgets need a doc comment naming the React source they port.

## Related

- [`gpui-crate-split.md`](gpui-crate-split.md) — the crate and module layout.
- [`gpui-port-parity.md`](gpui-port-parity.md) — what the port covers.
