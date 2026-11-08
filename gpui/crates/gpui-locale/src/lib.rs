//! Locale-aware user-visible text for the GPUI port: catalogues, the `t!`
//! macro, and the number/unit formatters.
//!
//! Ports these React sources:
//!
//! - `src/utils/unitFormatting.ts` — the `Unit` union, the `UNIT_MESSAGES`
//!   table and `UnitFormat` ([`units`]).
//! - `src/utils/localeNumber.ts` — `formatNumber`, `formatFixed`,
//!   `formatFieldNumber`, `decimalSeparator`, `parseDecimal` and the
//!   `LocaleFormat` object they hang off ([`fmt`]).
//! - `src/hooks/useLocaleFormat.ts` — the `fmt` a component gets for the
//!   active locale ([`UnitFormat::for_locale`]).
//! - `src/locales/*.po` and `lingui.config.ts` — the shipped catalogues and the
//!   locale list ([`catalog`], [`Locale`]).
//!
//! What is deliberately different from the React side:
//!
//! - **No React Compiler memoisation.** The TS caches one formatter per locale
//!   because object identity drives memoisation. Here [`LocaleFormat`] and
//!   [`UnitFormat`] are cheap values; keep one around and pass it, or build one
//!   where the locale is known.
//! - **No runtime catalogue loading.** [`Catalog`] reads a compiled, sorted
//!   `&[(msgid, text)]` table, so a lookup is a binary search and never touches
//!   the filesystem. `src/locales/*.po` are compiled in by
//!   `scripts/gen-catalog.mjs`; `lingui extract`'s PO output stays the source
//!   of truth.
//! - **`Intl.NumberFormat` is reimplemented, not called.** The six shipped
//!   locales need only a decimal separator, grouping and the rounding mode
//!   `Intl` uses (round half away from zero on the shortest decimal form of the
//!   double); [`fmt`] documents the measured behaviour it reproduces.

pub mod catalog;
pub mod current;
pub mod fmt;
pub mod locale;
pub mod units;

mod catalog_data;
mod macros;

pub use catalog::Catalog;
pub use catalog_data::SUPPORTED_LANGUAGES;
pub use current::{current_catalog, set_current_catalog, set_locale};
pub use fmt::{LocaleFormat, NumberOptions};
pub use locale::Locale;
pub use units::{Unit, UnitFormat};

/// The cheaply clonable string GPUI widgets take, re-exported so [`t!`] can
/// name it hygienically.
pub use gpui::SharedString;
