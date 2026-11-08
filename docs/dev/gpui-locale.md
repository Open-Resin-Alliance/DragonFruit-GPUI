# GPUI locale

`gpui/crates/gpui-locale` is the port's text and number layer: a catalog keyed
by English source text, the unit formatter ported from
`src/utils/unitFormatting.ts`, and the number helpers from
`src/utils/localeNumber.ts`.

The port needed this at the start, not after the panels. The React UI still
contains 26 hardcoded decimal formats and 29 literal unit strings, which is why
`src/utils/unitFormatting.ts` exists: English text and `.` decimals were being
baked into components faster than they could be removed. Repeating that in Rust
would have made the same cleanup twice as expensive.

## Why English is not enough

`toFixed` and bare `String(n)` always print `.` as the decimal separator, and a
unit written next to them never reaches the catalog. A Spanish UI then shows
`28.03 MB` beside `28,03` elsewhere, and a Russian or Hebrew UI shows the wrong
symbol and spacing. The unit message owns both the symbol and the number's
position, so translators can write `28 %` or `28%`, `мм` or `מ״מ`.

Display only. Input values, CSS, cache keys, file names, serialization and logs
keep the parseable form (`toFixed`/`String`); a field that shows a locale number
reads it back through the locale-aware parser.

## The `t!` macro: the English literal is the key

```rust
let title = t!("Save scene");
```

The literal is the message id. There is no generated key, no numbering step and
no separate identifiers file: the call site reads as the sentence it renders,
and extraction is a search for `t!` invocations. A catalog is a map from that
English text to its translation; English itself has no table because every
msgid resolves to itself.

The shipped tables in `gpui/crates/gpui-locale/src/catalog_data.rs` follow from
that choice:

- Only entries with a non-empty translation are included. An untranslated entry
  costs nothing, because lookup falls back to the msgid — which is already the
  English text.
- Tables are sorted by msgid byte order and searched with a binary search, so
  lookup does not depend on hash iteration order.
- Entries with a `msgctxt` are skipped. The macro carries only the English
  literal, so two meanings of the same word (the `Auto` mode and the `Auto`
  lift toggle) cannot be told apart yet; contexts come back together with
  interpolation.
- An unsupported language falls back to English rather than failing to start.
- Only the changes a user makes are stored in settings; the catalogs are
  compiled assets, not runtime state.

`t!` deliberately does not concatenate fragments. One sentence is one message
with placeholders, because word order differs per language. When interpolation
lands, the msgid stays the source sentence, not a formatted string.

## Units

There is one message per unit, each carrying the value placeholder. This is the
`UNIT_MESSAGES` table from `src/utils/unitFormatting.ts`, and the Rust port keeps
the same set and the same spellings:

| Unit | Message | Used for |
|---|---|---|
| `mm` | `{value} mm` | length |
| `mm2` | `{value} mm²` | area |
| `mm3` | `{value} mm³` | volume |
| `cm3` | `{value} cm³` | volume |
| `um` | `{value} μm` | layer height |
| `ml` | `{value} ml` | resin volume |
| `mmPerPx` | `{value} mm/px` | millimetres per pixel |
| `g` | `{value} g` | weight |
| `B` | `{value} B` | byte count |
| `KB` | `{value} KB` | byte count |
| `MB` | `{value} MB` | byte count |
| `GB` | `{value} GB` | byte count |
| `TB` | `{value} TB` | byte count |
| `s` | `{value} s` | duration |
| `ms` | `{value} ms` | duration |
| `fps` | `{value} fps` | frame rate |
| `perSecond` | `{value} /s` | rate |
| `percent` | `{value}%` | percentage |
| `degree` | `{value}°` | angle |
| `times` | `{value}x` | multiplier |
| `thousands` | `{value}K` | compact count |
| `millions` | `{value}M` | compact count |

The Rust formatter mirrors the two React entry points:

- `UnitFormat::quantity(value, fraction_digits, unit)` in `units.rs` formats the
  number for the locale, then places it in that unit's message.
- `UnitFormat::unit(unit, text)` places an already-formatted number in the
  message, for a value formatted some other way.

Rounding and grouping come from `src/utils/localeNumber.ts`:
`formatFixed` is the locale-aware `toFixed`, `formatFieldNumber` is what an
editable field shows (no grouping, no trailing zeros, never `-0`),
`decimalSeparator` is the character the locale writes, and `parseDecimal` reads
a field back — accepting the locale separator or `.` and returning `NaN` when
the whole text is not a number. The same object carries `number`, `field` and
`parse`, so a caller cannot mix a locale's formatter with another locale's
parser.

Examples pinned by `src/utils/__tests__/unitFormatting.dom-test.tsx`:

| Call | English | Spanish |
|---|---|---|
| `unit('mm', '1.5')` | `1.5 mm` | `1,5 mm` |
| `unit('percent', '28')` | `28%` | `28 %` |
| `quantity(12.345, 2, 'mm2')` | `12.35 mm²` | `12,35 mm²` |
| `quantity(0.05, 0, 'um')` | `0 μm` | `0 μm` |

## The extractor

The catalog is generated, never hand-edited:

1. The extractor scans the Rust sources for `t!` literals and collects the
   English msgids.
2. The catalog generator joins those msgids with the PO files under
   `src/locales/` (`es`, `de`, `fr`, `sk`, `cs`; English is the source locale).
3. It emits `catalog_data.rs`, with only non-empty translations, sorted by
   msgid, and drops entries that carry a `msgctxt`.
4. Running the extractor after adding a string leaves the new entry untranslated
   in every catalog; translations are added through the translation pipeline,
   not by editing Rust.

A new unit is added to the unit table with its translator comment, not written
next to a number at the call site. A new sentence is written once and extracted.

## Constraints

- **No bare user-visible literals** in the GPUI tree; a label goes through
  `t!`.
- **No fragments.** One message per sentence, with placeholders for values.
- **No `toFixed` for display.** A visible number goes through the locale
  formatter; serialization and logs keep the parseable form.
- **No hand-edited catalog.** `catalog_data.rs` is generated and sorted; a
  manual edit is overwritten by the next extraction.

## Sources

- `gpui/crates/gpui-locale/src/catalog.rs` — the sorted catalogue and its
  binary-search lookup.
- `gpui/crates/gpui-locale/src/catalog_data.rs` — shipped tables, msgid keys,
  msgctxt handling, supported languages.
- `gpui/crates/gpui-locale/src/macros.rs` — the `t!` macro: the English literal
  is the msgid.
- `gpui/crates/gpui-locale/src/fmt.rs` — `LocaleFormat`, `NumberOptions`:
  `number`, `fixed`, `field`, `parse`, `decimal_separator`.
- `gpui/crates/gpui-locale/src/units.rs` — `Unit`, `UnitFormat`, `quantity`,
  `unit`.
- `src/utils/unitFormatting.ts` — `UNIT_MESSAGES`, `Unit`, `UnitFormat`,
  `quantity`, `unit`.
- `src/utils/localeNumber.ts` — `formatFixed`, `formatFieldNumber`,
  `decimalSeparator`, `parseDecimal`, `LocaleFormat`.
- `src/utils/__tests__/unitFormatting.dom-test.tsx` — the locale examples.
- `docs/dev/localization.md` — the rules the React app follows.
- [`gpui-full-port-plan.md`](gpui-full-port-plan.md) — the 26/29 count and the
  decision to start i18n in English.
