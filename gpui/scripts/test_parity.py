"""Synthetic-image tests for the parity harness.

Run with::

    python -m unittest gpui/scripts/test_parity.py

No pytest and no real screenshots: every case builds its pixels with Pillow.
"""

from __future__ import annotations

import sys
import tempfile
import unittest
from pathlib import Path

from PIL import Image

sys.path.insert(0, str(Path(__file__).resolve().parent))

import parity  # noqa: E402

BACKGROUND = (128, 128, 128)
SIZE = (200, 200)
# Inside the default manifest's models-panel (x 0.0458..0.2764, y 0.0578..0.5578
# of a 200x200 canvas, i.e. pixels 9..55 by 12..112) and clear of every other
# region's edge.
BLOCK_ORIGIN = (20, 40)
BLOCK_SIZE = 10
BLOCK_COLOR = (0, 0, 0)


def solid(size=SIZE, color=BACKGROUND):
    return Image.new("RGB", size, color)


def with_block(size=SIZE, origin=BLOCK_ORIGIN, color=BLOCK_COLOR):
    image = solid(size)
    for y in range(origin[1], origin[1] + BLOCK_SIZE):
        for x in range(origin[0], origin[0] + BLOCK_SIZE):
            image.putpixel((x, y), color)
    return image


def rows(report: str):
    """Yield (name, cells) for the Markdown table's data rows."""
    for line in report.splitlines():
        if not line.startswith("| ") or line.startswith("| Region"):
            continue
        cells = [cell.strip() for cell in line.strip().strip("|").split("|")]
        yield cells[0], cells


def row_for(report: str, name: str):
    for row_name, cells in rows(report):
        if row_name == name:
            return cells
    raise AssertionError(f"region {name!r} missing from report:\n{report}")


def run(ts: Path, gpui: Path, out: Path, *extra: str) -> str:
    code = parity.main(
        ["--ts", str(ts), "--gpui", str(gpui), "--out", str(out), *extra]
    )
    assert code == 0
    return out.read_text(encoding="utf-8")


class ParityHarnessTests(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.tmp = Path(self._tmp.name)
        self.ts = self.tmp / "ts.png"
        self.gpui = self.tmp / "gpui.png"
        self.out = self.tmp / "report.md"

    def tearDown(self):
        self._tmp.cleanup()

    def test_identical_pair_passes_every_region(self):
        solid().save(self.ts)
        solid().save(self.gpui)
        report = run(self.ts, self.gpui, self.out)

        for row_name, cells in rows(report):
            self.assertEqual(cells[-1], "**pass**", f"{row_name} not passing: {cells}")
            self.assertEqual(cells[4], "0.00%", f"{row_name} still differs: {cells}")
        self.assertIn("**Summary:** 0 of 40,000", report)

    def test_block_only_flags_its_own_region(self):
        solid().save(self.ts)
        with_block().save(self.gpui)
        report = run(self.ts, self.gpui, self.out)

        block_row = row_for(report, "models-panel")
        self.assertEqual(block_row[-1], "**review**")
        # 100 recoloured pixels out of the panel's ~46x100 pixel box.
        self.assertNotEqual(block_row[4], "0.00%")

        for name in ("topbar", "tool-rail", "viewport", "support-panels"):
            self.assertEqual(row_for(report, name)[-1], "**pass**", name)

        # Worst-first ordering puts the flagged region at the top of the table.
        self.assertEqual(next(rows(report))[0], "models-panel")
        self.assertIn("**Worst region:** `models-panel`", report)

    def test_scaled_capture_is_reconciled_and_noted(self):
        solid().save(self.ts)
        solid((SIZE[0] * 2, SIZE[1] * 2)).save(self.gpui)
        report = run(self.ts, self.gpui, self.out)

        self.assertIn("resized to the React capture's 200x200", report)
        self.assertIn("**Summary:** 0 of 40,000", report)
        self.assertEqual(row_for(report, "viewport")[-1], "**pass**")

    def test_region_outside_image_is_skipped_not_fatal(self):
        solid().save(self.ts)
        solid().save(self.gpui)
        manifest = self.tmp / "regions.json"
        manifest.write_text(
            """
            {
              "regions": [
                {"name": "inside", "x": 0.0, "y": 0.0, "w": 0.5, "h": 0.5,
                 "source": "src/components/layout/TopBar.tsx"},
                {"name": "outside", "x": 1.0, "y": 0.0, "w": 0.0001, "h": 1.0,
                 "source": "src/components/layout/TopBar.tsx"}
              ]
            }
            """,
            encoding="utf-8",
        )
        report = run(self.ts, self.gpui, self.out, "--regions", str(manifest))

        self.assertIn("**pass**", row_for(report, "inside")[-1])
        self.assertNotIn("outside", [name for name, _ in rows(report)])
        self.assertIn("`outside` lies outside", report)
        self.assertIn("skipped", report)

    def test_diff_dir_writes_one_image_per_compared_region(self):
        solid().save(self.ts)
        with_block().save(self.gpui)
        diff_dir = self.tmp / "diffs"
        run(self.ts, self.gpui, self.out, "--diff-dir", str(diff_dir))

        written = sorted(path.name for path in diff_dir.glob("*.png"))
        self.assertIn("models-panel.png", written)
        self.assertIn("topbar.png", written)
        self.assertEqual(len(written), 8)
        with Image.open(diff_dir / "models-panel.png") as diff:
            self.assertEqual(diff.mode, "L")
            # Non-differing pixels stay black; the block's delta is amplified.
            self.assertEqual(diff.getpixel((0, 0)), 0)
            self.assertGreater(diff.getextrema()[1], 0)


if __name__ == "__main__":
    unittest.main()
