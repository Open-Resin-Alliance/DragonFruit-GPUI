//! `src/components/scene/SceneCanvas/PlateNameLabel.tsx` and
//! `src/components/scene/SceneCanvas/PlateSideButtons.tsx`.
//!
//! Both are widgets painted onto the build plate in the React app: a canvas
//! texture on a plane in the plate's own plane, so a model standing in front of
//! them hides them. GPUI has no scene overlay of that kind, so each widget is
//! ported as the same layout in 2D, at the same CSS-pixel metrics, for the
//! caller to place at the plate's projected anchor.
//!
//! # PORT DEVIATIONS
//!
//! * The plane placement (`position`, `facingRotation`, `labelScale`,
//!   `widgetWorldPerCssPixel`) is the viewport's job: these functions return the
//!   widget at its CSS size, not a world-space quad. A model cannot occlude them
//!   — the closest 2D equivalent is the caller's own z-order.
//! * `hover:brightness-125` / `source-atop` white washes are painted by
//!   brightening the drawn colours (text, panel, border) rather than filling the
//!   widget's transparent box, which is what the canvas composite did.
//! * The label's texture is baked at `TEXTURE_SCALE = 4`; GPUI draws text
//!   directly, so the scale is only used for the source's pixel rounding.
//! * The editor's hidden `<input>` (the keyboard target that keeps the painted
//!   field in sync) has no GPUI counterpart in this slice. The painted field, its
//!   selection wash and its caret are ported; the caller owns the draft, the
//!   selection and the 530ms caret blink (`CARET_BLINK_MS`).
//! * `fitFontToWidth` is reproduced with GPUI's text system: the same
//!   proportional shrink with an 8px floor, measured in Arial Bold 36px. Arial is
//!   requested by name; if the platform has no Arial, GPUI substitutes its
//!   fallback and the measured width follows that substitute.
//! * `useCursor(hovered)` becomes `cursor_pointer` on the label.
//! * The plate-widget colours are the dark theme's tokens (`theme.rs`), read
//!   directly instead of through `getComputedStyle`.

use std::rc::Rc;

use gpui::{
    Div, Font, FontWeight, IntoElement, SharedString, Window, div, prelude::*, px, rgb, rgba, svg,
};

use gpui_ui::panel_fields::with_alpha;
use gpui_core::theme::*;

/// The label's `text-[36px] font-bold` in Arial.
const LABEL_FONT_SIZE: f32 = 36.0;
/// `text-[36px] leading-tight`, in CSS pixels.
const LABEL_LINE_HEIGHT_CSS: f32 = 45.0;
/// The name's `px-1` on either side.
const LABEL_PADDING_CSS: f32 = 4.0;
/// `gap-2` between the name and the pencil.
const LABEL_GAP_CSS: f32 = 8.0;
/// The pencil: `h-9 w-9` with an `h-5 w-5` icon, `rounded-[5.5px]`, 1px border.
const PENCIL_BUTTON_CSS: f32 = 36.0;
const PENCIL_ICON_CSS: f32 = 20.0;
const PENCIL_RADIUS_CSS: f32 = 5.5;
/// Widest the name may get before its font is shrunk to fit the texture.
const LABEL_MAX_TEXT_CSS: f32 = 512.0;
/// Canvas pixels per CSS pixel baked into the label's texture.
const TEXTURE_SCALE: f32 = 4.0;

/// The editor's box: `rounded-[5.5px] border px-2 py-1`.
const EDITOR_PADDING_X_CSS: f32 = 8.0;
const EDITOR_PADDING_Y_CSS: f32 = 4.0;
const EDITOR_BORDER_CSS: f32 = 1.0;
const EDITOR_RADIUS_CSS: f32 = 5.5;
/// The narrowest the field gets, in characters.
const EDITOR_MIN_CHARS: usize = 6;
/// The caret's width in CSS pixels.
const EDITOR_CARET_CSS: f32 = 2.0;
/// The browser's caret period; the caller drives the blink.
pub const CARET_BLINK_MS: u64 = 530;

/// One plate-side button: `h-[104px] w-[104px]`, a `h-14 w-14` icon,
/// `rounded-[5.5px]`, 1px border.
pub const BUTTON_SIZE_CSS: f32 = 104.0;
pub const BUTTON_ICON_CSS: f32 = 56.0;
pub const BUTTON_RADIUS_CSS: f32 = 5.5;
pub const BUTTON_BORDER_CSS: f32 = 1.0;
/// The column's `gap-3`.
pub const COLUMN_GAP_CSS: f32 = 12.0;
/// `.plate-trash-button`'s hover red.
const DANGER_COLOR: u32 = 0xef4444;

/// `fitFontToWidth`: the font shrunk just enough for `text` to fit `max_width`,
/// with the source's 8px floor.
fn fit_font_to_width(window: &mut Window, text: &str, max_width: f32, base: f32) -> f32 {
    let width = measure_text(window, text, base);
    if width <= max_width || width == 0.0 {
        return base;
    }
    (base * (max_width / width)).floor().max(8.0)
}

/// The advance width of `text` at `size`, in Arial Bold — the font the scene's
/// canvas text is drawn in.
fn measure_text(window: &mut Window, text: &str, size: f32) -> f32 {
    let text_system = window.text_system().clone();
    let font = Font {
        family: "Arial".into(),
        weight: FontWeight::BOLD,
        ..Default::default()
    };
    let font_id = text_system.resolve_font(&font);
    let mut width = px(0.0);
    for ch in text.chars() {
        width += text_system.layout_width(font_id, px(size), ch);
    }
    f32::from(width)
}

/// An SVG icon with an alpha colour (the plate widgets paint several at 55%).
fn icon_alpha(name: &str, size: f32, color: u32) -> impl IntoElement {
    let path = format!("{}/assets/icons/{name}.svg", env!("CARGO_MANIFEST_DIR"));
    svg()
        .external_path(path)
        .w(px(size))
        .h(px(size))
        .text_color(rgba(color))
}

// ---------------------------------------------------------------------------
// PlateNameLabel
// ---------------------------------------------------------------------------

/// Everything the plate's name widget draws from.
#[derive(Clone, Debug)]
pub struct PlateNameLabelState {
    /// The plate's own name; empty means unnamed.
    pub name: String,
    pub placeholder: String,
    /// Accessible name of the editor: the wording the edit affordance carried.
    pub edit_title: String,
    /// The same, while the plate has no name yet.
    pub empty_title: String,
    pub hovered: bool,
    /// `draft != null` in the source: the editor is up.
    pub editing: bool,
    /// The typed draft, meaningful only while editing.
    pub draft: String,
    /// `(start, end)` of the draft's selection.
    pub selection: (usize, usize),
    /// The painted caret's blink phase, driven by the caller.
    pub caret_on: bool,
}

impl Default for PlateNameLabelState {
    fn default() -> Self {
        Self {
            name: String::new(),
            placeholder: "Plate name".into(),
            edit_title: "Edit plate name".into(),
            empty_title: "Name this plate".into(),
            hovered: false,
            editing: false,
            draft: String::new(),
            selection: (0, 0),
            caret_on: true,
        }
    }
}

#[derive(Clone)]
pub struct PlateNameLabelActions {
    pub on_hover: Rc<dyn Fn(bool)>,
    /// A press on the label while it is not editing.
    pub on_open_editor: Rc<dyn Fn()>,
    /// Enter, or a blur that is not this widget's own press: the trimmed draft.
    pub on_commit: Rc<dyn Fn(String)>,
    /// Escape: drop the draft without committing.
    pub on_cancel: Rc<dyn Fn()>,
}

impl Default for PlateNameLabelActions {
    fn default() -> Self {
        Self {
            on_hover: Rc::new(|_| {}),
            on_open_editor: Rc::new(|| {}),
            on_commit: Rc::new(|_| {}),
            on_cancel: Rc::new(|| {}),
        }
    }
}

/// The build plate's name, clicked to change it.
///
/// Laid out at the source's CSS metrics: 45px line, 4px side padding, an 8px gap
/// and the 36px pencil tile. The name shrinks to fit 512px exactly as
/// `fitFontToWidth` does; it is painted in `--accent` when named and
/// `--text-muted` while it is still the placeholder.
pub fn plate_name_label(
    state: &PlateNameLabelState,
    actions: &PlateNameLabelActions,
    window: &mut Window,
) -> gpui::AnyElement {
    if state.editing {
        return editor(state, window).into_any_element();
    }

    let text = if state.name.is_empty() {
        state.placeholder.clone()
    } else {
        state.name.clone()
    };
    let font_size = fit_font_to_width(window, &text, LABEL_MAX_TEXT_CSS, LABEL_FONT_SIZE);
    let base_color = if state.name.is_empty() {
        TEXT_MUTED
    } else {
        ACCENT
    };
    // `hover:brightness-125` — the drawn colours lift, the box stays clear.
    let color = if state.hovered {
        mix(base_color, 0xffffff, 0.85)
    } else {
        base_color
    };

    let hover = actions.on_hover.clone();
    let open = actions.on_open_editor.clone();

    div()
        .id("plate-name-label")
        .relative()
        .flex()
        .items_center()
        .gap(px(LABEL_GAP_CSS))
        .px(px(LABEL_PADDING_CSS))
        .h(px(LABEL_LINE_HEIGHT_CSS))
        .cursor_pointer()
        .on_hover(move |hovered, _, _| hover(*hovered))
        .on_click(move |_, _, _| open())
        .child(
            div()
                .font_family("Arial")
                .font_weight(FontWeight::BOLD)
                .text_size(px(font_size))
                .line_height(px(LABEL_LINE_HEIGHT_CSS))
                .text_color(rgb(color))
                .whitespace_nowrap()
                .child(text),
        )
        .child(pencil_tile(state.hovered))
        .into_any_element()
}

/// The pencil beside the name: `h-9 w-9 rounded-[5.5px] border` on
/// `--surface-0` at 55%, the 20px pencil at 55% `--text-muted`.
fn pencil_tile(hovered: bool) -> Div {
    // The panel and its border are painted at 55%; the pencil itself is drawn at
    // full `--text-muted` (the source's `drawPlateWidgetIcon` sits outside the
    // 0.55-alpha save/restore). Hover lifts every drawn colour by 15% white.
    let panel_alpha = 0x8c;
    let border_alpha = 0x8c;
    let icon_color = if hovered {
        mix(TEXT_MUTED, 0xffffff, 0.85)
    } else {
        TEXT_MUTED
    };
    div()
        .w(px(PENCIL_BUTTON_CSS))
        .h(px(PENCIL_BUTTON_CSS))
        .flex_shrink_0()
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(PENCIL_RADIUS_CSS))
        .border_1()
        .border_color(rgba(with_alpha(TEXT_MUTED, border_alpha)))
        .bg(rgba(with_alpha(SURFACE_0, panel_alpha)))
        .child(icon_alpha(
            "pencil",
            PENCIL_ICON_CSS,
            with_alpha(icon_color, 0xff),
        ))
}

/// The painted editor: the field's box, the draft, the selection wash and the
/// caret, all at the label's own position and scale.
fn editor(state: &PlateNameLabelState, window: &mut Window) -> gpui::Stateful<Div> {
    let text = if state.draft.is_empty() {
        state.placeholder.clone()
    } else {
        state.draft.clone()
    };
    let char_width = measure_text(window, "0", LABEL_FONT_SIZE);
    // `Math.ceil(Math.max(6, (draft || placeholder).length + 2) * charCss)`.
    let characters = (text.chars().count() + 2).max(EDITOR_MIN_CHARS);
    let box_width = (characters as f32 * char_width).ceil();
    let box_height = LABEL_LINE_HEIGHT_CSS + EDITOR_PADDING_Y_CSS * 2.0;
    let text_x = EDITOR_BORDER_CSS + EDITOR_PADDING_X_CSS;
    let text_width = box_width - text_x * 2.0;

    // A character of slack keeps the caret off the border.
    let font_size = fit_font_to_width(window, &text, text_width - char_width, LABEL_FONT_SIZE);

    let (start, end) = (
        state.selection.0.min(text.chars().count()),
        state.selection.1.min(text.chars().count()),
    );
    // `widthTo(index)` at the fitted size, measured once per edge.
    let mut width_to = |index: usize| -> f32 {
        let slice: String = text.chars().take(index).collect();
        measure_text(window, &slice, font_size)
    };
    let start_width = width_to(start);
    let end_width = width_to(end);

    let mut field = div()
        .id("plate-name-editor")
        .relative()
        .w(px(box_width))
        .h(px(box_height))
        .rounded(px(EDITOR_RADIUS_CSS))
        .border_1()
        .border_color(rgb(ACCENT))
        .bg(rgba(with_alpha(SURFACE_0, 0xe6)))
        .overflow_hidden();

    // The selection's wash, under the text.
    if end > start {
        field = field.child(
            div()
                .absolute()
                .left(px(text_x + start_width))
                .top(px(EDITOR_PADDING_Y_CSS))
                .w(px(end_width - start_width))
                .h(px(LABEL_LINE_HEIGHT_CSS))
                .bg(rgba(with_alpha(ACCENT, 0x4d))),
        );
    }

    field = field.child(
        div()
            .absolute()
            .left(px(text_x))
            .top(px(EDITOR_PADDING_Y_CSS))
            .w(px(text_width))
            .h(px(LABEL_LINE_HEIGHT_CSS))
            .flex()
            .items_center()
            .font_family("Arial")
            .font_weight(FontWeight::BOLD)
            .text_size(px(font_size))
            .line_height(px(LABEL_LINE_HEIGHT_CSS))
            .text_color(rgb(if state.draft.is_empty() {
                TEXT_MUTED
            } else {
                TEXT_STRONG
            }))
            .whitespace_nowrap()
            .child(text),
    );

    // The caret, gone while something is selected.
    if state.caret_on && end == start {
        field = field.child(
            div()
                .absolute()
                .left(px(text_x + start_width))
                .top(px(EDITOR_PADDING_Y_CSS))
                .w(px(EDITOR_CARET_CSS))
                .h(px(LABEL_LINE_HEIGHT_CSS))
                .bg(rgb(TEXT_STRONG)),
        );
    }

    field
}

/// `commitDraft`: trim the draft and commit it when it differs from the plate's
/// name.
///
/// The source runs this from the hidden `<input>` on Enter and on blur. That
/// input is not ported (see the module deviations), so the caller's input layer
/// calls this instead — the trim-and-compare rule is the source's, not a
/// simplification.
pub fn commit_plate_name(state: &PlateNameLabelState, actions: &PlateNameLabelActions) {
    if !state.editing {
        return;
    }
    let next = state.draft.trim();
    if next != state.name {
        (actions.on_commit)(next.to_string());
    }
}

/// Escape: drop the draft without committing it.
pub fn cancel_plate_name(actions: &PlateNameLabelActions) {
    (actions.on_cancel)();
}

// ---------------------------------------------------------------------------
// PlateSideButtons
// ---------------------------------------------------------------------------

/// Everything the plate's side buttons draw from.
#[derive(Clone, Debug)]
pub struct PlateSideButtonsState {
    /// The add-plate button's hover wording while it is inert.
    pub add_coming_soon_title: Option<SharedString>,
    /// Absent while the app has nowhere to keep a second plate.
    pub can_add: bool,
    pub locked: bool,
    pub arrange_disabled: bool,
    pub clear_disabled: bool,
}

impl Default for PlateSideButtonsState {
    fn default() -> Self {
        Self {
            add_coming_soon_title: None,
            can_add: false,
            locked: false,
            arrange_disabled: false,
            clear_disabled: false,
        }
    }
}

#[derive(Clone)]
pub struct PlateSideButtonsActions {
    pub on_add: Rc<dyn Fn()>,
    pub on_toggle_lock: Rc<dyn Fn()>,
    pub on_arrange_plate: Rc<dyn Fn()>,
    pub on_clear_plate: Rc<dyn Fn()>,
}

impl Default for PlateSideButtonsActions {
    fn default() -> Self {
        Self {
            on_add: Rc::new(|| {}),
            on_toggle_lock: Rc::new(|| {}),
            on_arrange_plate: Rc::new(|| {}),
            on_clear_plate: Rc::new(|| {}),
        }
    }
}

/// One plate-side button: the rounded panel, its 56px icon, and the pointer
/// feedback the canvas painted itself.
///
/// `locked` wears the accent panel; `destructive` turns the bin red under the
/// pointer; `enabled == false` dims the whole button to 55% and leaves it inert.
fn plate_button(
    id: &'static str,
    icon_name: &'static str,
    locked: bool,
    destructive: bool,
    enabled: bool,
    on_click: Rc<dyn Fn()>,
) -> gpui::Stateful<Div> {
    let (border_alpha, background) = if !enabled {
        (0x66, rgba(with_alpha(SURFACE_0, 0x73)))
    } else if locked {
        (0xb3, rgb(mix(SURFACE_1, ACCENT, 0.85)))
    } else {
        (0x73, rgba(with_alpha(SURFACE_0, 0x73)))
    };

    let tint = if locked { ACCENT } else { TEXT_MUTED };
    let icon_color = if locked { TEXT_STRONG } else { TEXT_MUTED };

    // The bin's red hover is the source's `0.12` danger wash over the panel it
    // already had — `surface0`, since the bin is never the locked plate's.
    let hover_background = if destructive {
        rgb(mix(SURFACE_0, DANGER_COLOR, 0.88))
    } else {
        rgb(mix(SURFACE_0, 0xffffff, 0.92))
    };
    let hover_border = if destructive {
        rgba(with_alpha(DANGER_COLOR, 0x99))
    } else {
        rgba(with_alpha(TEXT_MUTED, 0x8c))
    };

    let mut button = div()
        .id(id)
        .group(id)
        .w(px(BUTTON_SIZE_CSS))
        .h(px(BUTTON_SIZE_CSS))
        .flex_shrink_0()
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(BUTTON_RADIUS_CSS))
        .border_1()
        .border_color(rgba(with_alpha(tint, border_alpha)))
        .bg(background);

    let icon_element = svg()
        .external_path(format!(
            "{}/assets/icons/{icon_name}.svg",
            env!("CARGO_MANIFEST_DIR")
        ))
        .w(px(BUTTON_ICON_CSS))
        .h(px(BUTTON_ICON_CSS))
        .text_color(rgb(icon_color))
        .group_hover(id, move |style| {
            style.text_color(rgb(if destructive { DANGER_COLOR } else { icon_color }))
        });

    button = button.child(icon_element);

    if enabled {
        button = button
            .cursor_pointer()
            .hover(move |style| style.bg(hover_background).border_color(hover_border))
            .on_click(move |_, _, _| on_click());
        button
    } else {
        button.opacity(0.55).cursor_not_allowed()
    }
}

/// The column of three buttons: add, lock/unlock, arrange.
pub fn plate_side_column(
    state: &PlateSideButtonsState,
    actions: &PlateSideButtonsActions,
) -> impl IntoElement {
    let add = actions.on_add.clone();
    let lock = actions.on_toggle_lock.clone();
    let arrange = actions.on_arrange_plate.clone();
    let locked = state.locked;

    div()
        .flex()
        .flex_col()
        .gap(px(COLUMN_GAP_CSS))
        .child(plate_button(
            "plate-add",
            "plus",
            false,
            false,
            state.can_add,
            add,
        ))
        .child(plate_button(
            "plate-lock",
            if locked { "lock" } else { "lock-open" },
            locked,
            false,
            true,
            lock,
        ))
        .child(plate_button(
            "plate-arrange",
            "layout-grid",
            false,
            false,
            !state.arrange_disabled,
            arrange,
        ))
}

/// The bin that stands on the plate's front edge.
pub fn plate_bin(
    state: &PlateSideButtonsState,
    actions: &PlateSideButtonsActions,
) -> impl IntoElement {
    let clear = actions.on_clear_plate.clone();
    plate_button(
        "plate-clear",
        "trash-2",
        false,
        true,
        !state.clear_disabled,
        clear,
    )
}

/// Both widget groups.
///
/// The source anchors the column on the plate's back edge and the bin on its
/// front edge (`columnAnchor` / `clearAnchor`); with no world anchors here they
/// are stacked, one column gap apart, for the caller to place (see the module
/// deviations).
pub fn plate_side_buttons(
    state: &PlateSideButtonsState,
    actions: &PlateSideButtonsActions,
) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap(px(COLUMN_GAP_CSS))
        .child(plate_side_column(state, actions))
        .child(plate_bin(state, actions))
}
