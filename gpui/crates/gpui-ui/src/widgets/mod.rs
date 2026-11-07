//! Ported primitives from `src/components/atoms` and `src/components/ui`.
//!
//! Every value here comes from the source component's class strings and the
//! tokens in `theme.rs`; nothing is invented. Where GPUI has no equivalent for a
//! CSS feature (transitions, gradients, `text-overflow: fade`, portals, backdrop
//! blur) the nearest faithful substitute is used and called out on the item.
//!
//! Tailwind v4's `rounded-*` utilities read `--radius-*`, and `globals.css`
//! overrides those properties in `:root` (`--radius-sm: 6px`,
//! `--radius-md: 10px`, `--radius-lg: 14px`), so in the running app
//! `rounded-sm`/`md`/`lg` are 6/10/14 — not Tailwind's stock 4/6/8. `xl` is not
//! overridden and stays 12. Bare `rounded` is 4 and `rounded-[Npx]` is N.
//! The hand-written `.ui-input`, `.ui-button` and `.ui-panel` rules read the
//! same properties through `var(--radius-*)`, so they use `theme::RADIUS_*`.
//!
//! ## PORT DEVIATIONS
//!
//! Platform gaps, all recorded at the item they affect:
//!
//! - **Transitions / keyframes** — `transition-colors`, `transition-transform`,
//!   `animate-pulse`, the toast fade-and-rise and the indeterminate progress
//!   sweep are all static here; GPUI has no CSS animation. Every element is
//!   drawn in its settled state.
//! - **`backdrop-filter`** — `backdrop-blur-sm` on the dialog backdrop, the
//!   select menu (`blur(6px) saturate(112%)`) and `backdrop-blur-[1px]` on the
//!   blocking overlay are omitted; the translucent fill is kept.
//! - **Portals** — `Tooltip`, `ContextMenu`, `SelectDropdown`'s menu and
//!   `StructuredDialogModal` are portalled to `<body>` at the source. GPUI has
//!   no portal, so each renders where the caller places it (the select menu and
//!   the tooltip as absolute children of their trigger).
//! - **Measurement** — no `getBoundingClientRect` pass, so the viewport clamps,
//!   the upward flip, the right-edge flip and the cursor-following placement
//!   that the source computes after mount are not reproduced. `MouseTooltip`
//!   takes the cursor point from the caller instead of listening to `mousemove`.
//! - **`letter-spacing`** — `tracking-wide` has no GPUI equivalent and is not
//!   applied (segmented pills, select headings, blocking-overlay elapsed line).
//! - **`title` attributes / accessible names** — GPUI widgets carry no
//!   `aria-*`/`title`, so those are dropped rather than faked.
//! - **`text-overflow: fade`** — `FadeText` clips (`overflow: hidden`,
//!   `white-space: nowrap`, `text-overflow: clip`) without the trailing mask.
//! - **`currentColor`** — an `svg()` element in GPUI does not inherit text
//!   colour, so every icon colour is passed explicitly.
//! - **`shadow-lg` / `shadow-xl` / `shadow-2xl`** — GPUI's `shadow_*` helpers
//!   carry Tailwind's exact values, so those are used as-is. The custom
//!   `box-shadow` strings in the sources are transcribed layer by layer.
//! - **Unitless `line-height`** — CSS inherits `1.45` / `1.3` as a *number* and
//!   recomputes it per element; GPUI's `line_height` is a length, so the value
//!   set on a container is inherited as-is. Elements that change their own font
//!   size therefore set their own line-height, and a container's ratio is
//!   resolved against the container's font size.
//! - **Text editing** — `NumberInput`, `ScrollableNumberField`,
//!   `CompactNumberField` and `MiniStepperField` render the frame, the value and
//!   the steppers. GPUI has no editable text field in this port, so typing, the
//!   locale parse/format, the wheel step and the blur commit stay with the
//!   caller; the step callbacks are the atoms' only interaction.
//! - **`w-[min(520px,92vw)]`** — `BlockingOverlay`'s panel is `92%` of the
//!   overlay (the window) capped at 520px, which is the same box.
//! - **Select menu height** — the source measures the trigger and caps the menu
//!   at `max(120, space below)`; with no measure pass the port caps it at 320px.
//! - **`title` / `aria-*` on options** — `SegmentedOption.title` and the
//!   `aria-label`s the sources pass have no GPUI counterpart.
//! - **Cascade layers (confirmed)** — Tailwind v4 emits its utilities inside
//!   `@layer utilities`, and `globals.css` declares `.ui-input`, `.ui-select`
//!   and `.ui-button` **unlayered**, so those three rules outrank every normal
//!   utility on the same element. Consequences the port applies:
//!   `.ui-input` keeps radius 6px, `padding: 6px 10px`, `font-size: 13px` and
//!   `line-height: 1.3` against `rounded-[4px]`, `px-*`, `py-0`, `pr-10`,
//!   `text-sm` and `text-xs`; `.ui-button` keeps `padding: 5px 11px 7px 11px`
//!   and `font-size: 14px` against `px-2`/`px-2.5`/`px-3`/`px-4` and
//!   `text-[11px]`/`text-xs`/`text-sm`, so a `Button` size class only supplies a
//!   height. `!important` utilities (`!h-7`, `!p-2`, `!bg-[…]`) still win, which
//!   is why the `IconButton` and `ScrollableNumberField` stepper padding applies.
//!   `.ui-panel`'s `h3` rule is likewise unlayered and `!important` on
//!   font-size, so panel titles stay 14px with the 1.2 line-height.

mod constants;

pub mod button;
pub mod card;
pub mod color;
pub mod controls;
pub mod dialog;
pub mod feedback;
pub mod icon;
pub mod input;
pub mod layout;
pub mod menu;
pub mod number_field;
pub mod select;
pub mod select_menu;
pub mod tooltip;

pub use button::*;
pub use card::*;
pub use color::*;
pub use controls::*;
pub use dialog::*;
pub use feedback::*;
pub use icon::*;
pub use input::*;
pub use layout::*;
pub use menu::*;
pub use number_field::*;
pub use select::*;
pub use select_menu::*;
pub use tooltip::*;

pub use gpui_core::state::ToastTone;
