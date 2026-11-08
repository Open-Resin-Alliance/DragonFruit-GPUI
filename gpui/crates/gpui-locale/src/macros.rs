//! The `t!` macro.

/// Translates an English literal into the active locale's text.
///
/// The English text **is** the msgid, so there is no key table to keep in sync:
/// `t!("Add printer")` looks up `"Add printer"` in the catalogue for the current
/// locale and returns the translation, or the literal itself when that locale
/// has none. Because every msgid is a compiled-in literal, the result is a
/// [`SharedString`](crate::SharedString) built without allocating.
///
/// ```
/// use gpui_locale::t;
///
/// // English is the default catalogue, so the msgid comes straight back.
/// assert_eq!(&*t!("Add printer"), "Add printer");
/// ```
///
/// # Interpolation is a later slice
///
/// The macro takes a bare literal and nothing else. Messages with placeholders
/// (a plural, `{value}`, `{0}`) pass through as written, so a call site that
/// needs to fill them must build the number through [`fmt`](crate::fmt) and
/// hand it to a helper that takes the message, or wait for the slice that
/// teaches `t!` about `{…}`. The `@lingui/core/macro` table is ported as-is,
/// without the ICU parser interpolating it would need.
///
/// # Explicit catalogue
///
/// `t!(catalog, "literal")` translates through a catalogue the caller holds
/// instead of the thread-local one, for background work that must not depend on
/// which thread it runs on:
///
/// ```
/// use gpui_locale::{Catalog, Locale, t};
///
/// let german = Catalog::for_locale(&Locale::parse("de"));
/// assert_eq!(&*t!(german, "Add printer"), "Drucker hinzufügen");
/// ```
#[macro_export]
macro_rules! t {
    ($msgid:literal) => {
        $crate::current::translate($msgid)
    };
    ($catalog:expr, $msgid:literal) => {
        $crate::SharedString::new_static($crate::Catalog::text(&$catalog, $msgid))
    };
}
