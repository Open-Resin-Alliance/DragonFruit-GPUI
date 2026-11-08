# Coverage report — after the four area audits

Inputs: `settings-findings.md`, `chrome-panels-findings.md`,
`profiles-printing-findings.md`, `transforms-overlays-findings.md`.
Machine ids: `gpui/scripts/surface.json` (2,130 rows). Ledger:
`docs/internal/gpui-coverage.json`. Gate: `node gpui/scripts/check-coverage.mjs`.

Status vocabulary as the ledger uses it: `correct` and `deliberate` fold to
`verified`; `wrong` folds to `verified` when the audit says it was fixed and to
`absent` when it was not; `dead` and `absent` stay themselves. Every surface the
four audits never mention was left `absent`.

## Headline

| | before | after |
|---|---|---|
| rows in the ledger | 2,130 | 2,130 |
| `verified` | 0 | **603** |
| `dead` | 0 | **19** |
| `absent` | 2,130 | 1,243 |
| verified share | 0.0 % | **32.3 %** |

622 rows changed status (603 verified + 19 dead); 1,064 rows were reached by a
rule, of which 442 were confirmed `absent` by the audits themselves (unported
dialogs, fleet/remote surfaces, custom-theme editors, `SharedPanelStack`).

Rows carry the audit evidence: `rust` is the port file the audit names,
`behaviour` is the finding line (with the findings file implied), `pixels` holds
the numeric recipe where the audit gives one, and `unread` names the field a
`dead` row writes and nothing reads.

## Per area

```
node gpui/scripts/check-coverage.mjs

surface: 1865 rows   ledger: 1865 rows
verified: 32.3% of the ledger

area          rows  verified  dead  absent  verified%
------------  ----  --------  ----  ------  ---------
SceneCanvas     37         8     0      29       21.6
app             30         3     0      27       10.0
atoms            5         1     0       4       20.0
autoSupport     39         0     0      39        0.0
controls       248        91    13     144       36.7
debug           16         0     0      16        0.0
gizmo            1         1     0       0      100.0
layout          64        28     3      33       43.8
meshSettings     4         0     0       4        0.0
modals         281         0     0     281        0.0
organisms       14        14     0       0      100.0
panels         104         6     0      98        5.8
printers        19        10     0       9       52.6
rotate           1         0     0       1        0.0
scene           50         0     0      50        0.0
selection        1         0     0       1        0.0
settings       924       419     1     504       45.3
ui              27        22     2       3       81.5
```

`modals` (281), `scene` (50), `autoSupport` (39), `debug` (16) and `meshSettings`
(4) are untouched: no audit in the set covers those owners. `settings` dominates
the verified total (419 of 603) simply because it is the largest extracted area
and one of the four audits is the settings module.

## Dead rows, grouped by the field left unread

Ranked by group size (`check-coverage.mjs` prints this list):

| # | rows | unread field | example row | why it can never arrive |
|---|---|---|---|---|
| 1 | 3 | `support blocker store` | `controls/clearsupportblockers` | no blocker store: `blocked_count` is always 0, so Clear is disabled and Blockers paints into nothing (transforms-overlays §10) |
| 2 | 2 | `arrange engine` | `controls/arranging-arrange-all` | `tool_panels.rs:707-711` — no arrange engine, the action only re-renders |
| 3 | 2 | `duplicate engine` | `controls/duplicating-confirm-duplicate` | `tool_panels.rs:774-777` — no duplicate engine |
| 4 | 2 | `view_type` | `controls/onchange--viewtypedropdown` | `app.view_type` never reaches the viewport/shader pass, so the view-mode dropdown and trigger are inert (chrome-panels #1/#54) |
| 5 | 1 | `transform.local_space` | `controls/local` | `on_local_space_change` writes `store.transform.local_space`; the gizmo is built from world axes only (transforms-overlays §10) |
| 6 | 1 | `transform.auto_lift` | `controls/auto--transformcontrols` | the flag is stored and never consumed; `useAutoLift` is unported (§10) |
| 7 | 1 | `transform.selection_rotation_mode` | `controls/group` | starts `None`, is only written by this button, so its own gate never satisfies (§2/§10) |
| 8 | 1 | `overhang_view` | `layout/overhangs-view-on-overhangs-view-off` | stored and mirrored into `AppBarState`, nothing consumes it (chrome-panels #32) |
| 9 | 1 | `select_fleet_unit` | `layout/offline-online` | the fleet row sets it, nothing reads it; `printer_picker::badge()` returns an empty fleet (chrome-panels #17) |
| 10 | 1 | `general.tool_layout` | `settings/toolbar-position` | the control writes `gpui_store.general.tool_layout`; the rail never reads it back (chrome-panels #52 / new work 8) |
| 11 | 1 | `gpui_locale::set_locale` | `ui/language` | `set_locale` is never called anywhere in `gpui/src`, so choosing a language changes nothing (settings cross-cutting) |
| 12 | 1 | `model details` | `controls/model-details` | no details dialog; the tooltip shows the polygon clause only (chrome-panels #79) |
| 13 | 1 | `model visibility` | `controls/hide-show` | no per-model visibility; the label is always Hide (#80) |
| 14 | 1 | `editor menu actions` | `ui/editor-context-menu` | only delete acts; cut/copy/paste/repair/support verbs close the menu (#60) |

Four dead surfaces the audits name have **no machine id at all**, so they cannot
appear in the list above or in the ledger:

- `show_monitor` — chrome-panels #20. `app.rs` passes `monitor: Default::default()`
  and the field is unread, but the Monitor button is not an extracted surface
  (only its tooltip text, `layout/selected-printer-is-offline-open-printer-monitor`,
  is, and that one is `absent` per #13).
- `cut engine` (Split `Cut`) and `snap_to_edges` (Split `Snap to Edges`) —
  transforms-overlays §10. The whole `OrganicCutPanel` body has no extracted rows;
  only its mount, `panels/organiccutpanel`, exists.
- `highlight_color` (Smoothing hex box / picker) — same reason: the smoothing
  body has no extracted rows, only `panels/meshsmoothingsettingspanel`.

`overlay_view` in the ticket text is `overhang_view` in the port
(`gpui/src/app.rs:254`); the ledger uses the code's name.

## Audit findings that could not be matched to a machine id

The four files carry 417 verdict-bearing table rows. Promoting them to rules gave
**123 named surfaces** (106 component/row rules + 17 findings with no id). All
106 rules matched at least one row — 1,064 rows in total. **17 findings (13.8 %)
had no machine id**, and one further class was folded rather than matched
individually: the 294 table rows that are sub-surfaces of an already-matched
component inherit that component's verdict and are not separately resolvable at
the ledger's granularity.

Rules by audit: settings 33 (375 rows), chrome-panels 27 (176), profiles-printing
25 (383), transforms-overlays 21 (130).

The 17, with examples of why:

| finding | why no id |
|---|---|
| settings: radii across every tab | a cross-cutting fix (30 call sites), no single surface |
| settings: `color-mix` direction | ditto; the mixes are inside widgets |
| settings: i18n sweep | module-wide, nothing renders a translated string |
| settings: rail description tooltip | `TabDescription` (SettingsModal.tsx:272-343) is extracted only as `settings/onselect` |
| settings: second scroll position | a layout policy (`usesInternalTabScrollLayout`), not an element |
| settings: Slicing tab | no `SlicingSettingsTab` row exists in `surface.json` |
| settings: Updates tab body | only the rail entry `settings/updatessettingstab` exists |
| settings: Hotkeys empty-section filter | a filter over cards, no element |
| settings: Toolbar-position rail reader | the reader lives in `rail.rs`, which has no row |
| chrome: bar tooltips / monitor visibility | the Monitor button, step hints and badge titles are text, not surfaces (#13/#20/#23) |
| chrome: Prepare/Export plate-scope derivation | composition, no element (#93) |
| transforms: `OrganicCutPanel` body | no extracted rows (only the mount) |
| transforms: `MeshSmoothingSettingsPanel` body | no extracted rows (only the mount) |
| transforms: Split `Cut` / `Snap to Edges` | inside the unextracted Organic Cut body |
| transforms: protractor dial / `RotationHintTooltip` | absent surfaces have no rows to match |
| transforms: plate-locked / new-input-device / leaf-fanning / marquee toasts | only the generic `organisms/toast` shells are extracted |
| profiles: `requestClose` gating + Multi-Tank mismatch dialog body | dialog state machine, no element |

That rate is the interesting number: **about one in seven audit findings names
something the surface extractor cannot see.** The pattern is consistent — the
extractor captures rendered elements, so it misses (a) cross-cutting visual
recipes, (b) behaviour that hangs off a widget's handlers, and (c) whole
components whose spec surfaces live in a file the extractor skipped
(`SlicingSettingsTab`, `UpdatesSettingsTab`, `OrganicCutPanel`,
`MeshSmoothingSettingsPanel`). Closing those four extraction gaps would remove
six of the seventeen.

## Judgement calls worth knowing

- `settings/toolbar-position` is `dead`, not `verified`. The settings audit calls
  the row correct (it renders and writes the draft); the chrome audit says the
  rail never reads `general.tool_layout`. A control nothing reads does nothing,
  so the reader-side verdict wins and the disagreement is recorded in the row's
  `behaviour`.
- `controls/group` is `dead`, not `absent`. Transforms-overlays §2 calls the
  toggle "gated on `selection_rotation_mode` — correct", §10 lists it as dead,
  and §2's reachability note says it cannot render. §10 is the dead list, and
  `selection_rotation_mode` is one of the ticket's named groups, so the row is
  `dead` with that field.
- `settings/handletoggle` stays `absent`: settings-c marks the Experiments
  persistence shape `wrong` **and not fixed**, which folds to `absent`.
- `settings/remember-window-positions` is `verified` although the audit says
  "partly fixed": the toggle and the Reset button were fixed in the audit and the
  remaining gap (the `app.rs:406` startup sync) is a different file's surface, so
  it is recorded in `behaviour` rather than demoting the row.
- Backups/`LocalBackupsSettingsTab` rows are `absent` except the two cards: the
  audit states the history rows, snapshot-content modal and both confirmations
  are unreachable with no bridge.
- `ProfileSettingsModal.tsx` is split by source line into shell, printer list,
  material browser, editor, dialogs and the fleet/remote/material-library blocks,
  because that one file serves four different audits' verdicts.
