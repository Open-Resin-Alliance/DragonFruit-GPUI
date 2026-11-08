# GPUI port — UI audit (W0)

Read-only audit of the already-ported UI against `../src`, run as the last W0
task. Findings are ranked by user-visible impact; W3 executes them.

## Blockers

1. **Locked workflow steps still switch mode** — `chrome/app_bar.rs::step_button`.
   The port dims a locked step (`opacity 0.45`) but gates the click on `disabled`
   alone, so with no model loaded Support/Export/Printing still open an empty
   scene. React (`TopBar.tsx:975-981`) makes a locked step inert and drops its
   hover. Fix: gate click, cursor and hover on `dimmed`.
2. **Support rail tiles do not change the panel** — `chrome/rail.rs::apply_action`
   writes rail-local `support_mode`/`hollowing`, while `app.rs` chooses the
   Support panel from the leftover *Prepare* `active_tool`. Clicking Islands /
   Manual / Automatic repaints the rail and nothing else, and `hollowing_body`
   is unreachable. React drives it from `buildSupportToolRailEntries` →
   `supportRailMode`. Same species as the `translate` bug: a field written and
   never read.
3. **Auto Supports settings dialog is dead UI** — `panels/content.rs::supports_body`
   documents that the header gear is not rendered, and `app.rs` builds the
   `support-auto` panel with no `header_right`, so `show_support_settings = true`
   is never set. `AutoSupportPanel.tsx:307-311` renders the gear.

## Systematic

4. **`rounded-lg` read as 8px** in six settings/preview modules where
   `globals.css`'s `--radius-lg` says 14px — the same misreading fixed elsewhere this cycle.
   Search the remaining `ROUNDED_LG = 8` / literal `8.0` radii.

## Method

Every finding cites the port file:function, the React file:function, and the
difference between them. Sections: `chrome/`, `panels/`, `features/settings/`,
`features/*panels*`, `widgets/`, then an "overhaul candidates" list for modules
where patching is worse than rewriting.
