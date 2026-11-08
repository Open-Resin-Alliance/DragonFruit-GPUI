# GPUI visual parity harness

`gpui/scripts/parity.py` turns "these two windows look different" into numbers.
It compares a full-window screenshot of the React/Tauri app with one of the GPUI
app, region by region, and writes a Markdown report naming the worst region and
its percentage. The companion rules page is
[GPUI Port Parity](gpui-port-parity.md): that one says *what* must match, this
one says *how to measure* whether it does.

```bash
python gpui/scripts/parity.py --ts ts.png --gpui gpui.png \
  --diff-dir diffs/ --out parity.md
```

Requirements: Python 3 and Pillow. No cargo, no npm, no build step.

## Capture both windows

The comparison assumes two screenshots of the **same window at the same size**,
each covering the whole window (no crop, no title bar from one side only).

1. Put both apps in the same state: same tool mode, same panel visibility, same
   loaded model, same theme, same zoom. A region that differs because the
   panels were showing different things is not a parity bug.
2. Size both windows identically (1440x900 is the manifest's reference, but any
   size works — the regions are fractions). The GPUI port and the Tauri window
   accept the same dimensions.
3. Capture each window without including overlapping windows: `Alt+Print` on
   Windows screenshots the focused window, or use `Win+Shift+S` and select the
   window.
4. Keep the display scale identical for both captures (both 100%, or both the
   same scale). If the two images still differ in size, the harness resizes the
   GPUI capture onto the React canvas when the aspect ratios match, or crops
   both to the common area when they do not, and says which in the report
   header. `--scale` (default `auto`) additionally downscales both images by an
   integer factor once the long edge exceeds 2400 px; `auto` keeps 1x below that.

## What is compared

Regions come from `gpui/scripts/parity-regions.json`. Every entry is a name, a
box as **fractions of the window width/height** (`x`, `y`, `w`, `h`, each
0..1), and the React component it corresponds to:

| Region | React source |
|---|---|
| `topbar` | `src/components/layout/TopBar.tsx` |
| `tool-rail` | `src/components/controls/ToolRail.tsx` |
| `models-panel` | `src/components/organisms/panels/ModelsPanel.tsx` |
| `support-panels` | `src/supports/Settings/SupportSidebar.tsx` |
| `settings-modal` | `src/components/settings/SettingsModal.tsx` |
| `viewport` | `src/components/scene/SceneCanvas/SceneCanvas.tsx` |
| `right-rail` | `src/components/controls/CrossSliderPanel.tsx` |
| `status-toasts` | `src/components/organisms/NotificationStack.tsx` |

The geometry is derived from `globals.css`'s `--topbar-height` (52 px),
`ToolRail.tsx`'s `TOOL_RAIL_WIDTH_PX` (66 px), `FloatingPanelStack.tsx`'s
320 px panel width plus 6 px margins, the 48 px `CrossSliderPanel` rail, and
`SettingsModal`'s `max-w-[72rem]` inside `p-5`. At 1440x900 that gives the pixel
boxes in the manifest's `referenceWindow` note. Because the boxes are
fractions, the same manifest works at 1280x720 or 2560x1440 without edits.

Regions deliberately overlap: the settings modal covers the viewport and both
panels when it is open, and its row is meant to be read on a screenshot where
it is open. Capture the modal-closed and modal-open states as separate pairs;
do not expect one run to pass both.

To add a region, add an entry with a name, a fractional box and the React file
it maps to. A region whose box falls off the image is skipped with a warning in
the report's **Warnings** section rather than crashing the run.

## Metrics and tolerance bands

Each region reports:

- **Mean Δ** — mean absolute per-channel difference across R, G and B, 0..255.
- **P95 Δ** — 95th percentile of the per-pixel delta, where a pixel's delta is
  its largest single-channel difference. This is the "how bad is the bad part"
  number: a handful of antialiased glyph edges move P95 far less than a shifted
  panel does.
- **Differing** — percentage of the region's pixels whose delta exceeds the
  threshold, 8/255 by default (`--threshold` overrides it). 8/255 is roughly
  the noise floor of two independent rasterisers drawing the same grey text.

Verdicts: **pass** under 1% differing, **review** under 5%, **fail** at 5% or
more. The bands exist because:

- Text is antialiased differently. Geist is rasterised by the browser on one
  side and by the GPUI/winit text stack on the other; hinting, gamma and
  subpixel positioning differ, so a *correct* port still differs on glyph
  edges. That is why the threshold is the metric and the mean alone is not.
- Icons scaled to fractional sizes land on different subpixel offsets.
- GPU-composited shadows and blends round differently from CPU rasterisation.
  The port has no backdrop blur (see the `PORT DEVIATIONS` blocks), and that
  will show up honestly as a solid region difference, not as noise.
- A genuinely wrong value — a 4 px radius instead of 5.5, an icon at 14 instead
  of 16, a panel 320 px wide instead of 340 — moves far more than 5% of a
  region's pixels or a large block of them at once. The percentages are there
  to separate "different rasteriser" from "different layout".

Both captures must be the same window size; the harness reconciles differences,
but a resized screenshot resamples text and inflates every number, so matching
sizes first is always the better comparison.

## Reading the diff images

`--diff-dir diffs/` writes one greyscale PNG per compared region. The per-pixel
delta (largest channel difference) is multiplied by 4, clamped to white.
Black means identical, grey isolines mean small antialiasing differences,
white means the delta was at least 64/255. Look for:

- **Solid white blocks** — a panel, button or row is in a different place or
  has a different fill. This is the actionable class.
- **White halos around text** — expected AA difference; ignore unless the text
  itself is in a different position, which shows as a doubled ghost.
- **One shifted edge** — padding or width drift; compare against the React
  component named in the row.

The images are per region and named after the region, so a report row and its
diff image share a name.

## The honest limit

This harness only compares pixels. **A control that is dead on both sides
renders identically and passes**, and so does a control wired to the wrong
action as long as the wrong action is never pressed. A button with the right
colours and the wrong hit area passes too. Pixel parity is necessary, not
sufficient: behaviour, focus order, keyboard handling and every interaction
states (hover, pressed, disabled, drag) still need the UI audit that
[GPUI Port Parity](gpui-port-parity.md) describes. Use this harness to stop the
value drift from being argued about by eye, not as a substitute for using the
app.

## Tests

The harness has synthetic-image tests, so they run anywhere Pillow does:

```bash
python -m unittest gpui/scripts/test_parity.py
```

They cover an identical pair (everything passes, summary is zero), a recoloured
block inside one region (that region is flagged, its neighbours stay clean), a
2x-scaled capture (sizes are reconciled and the report says so), a region
outside the image (skipped with a warning) and the `--diff-dir` output.
