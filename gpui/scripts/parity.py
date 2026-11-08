#!/usr/bin/env python3
"""Pixel-parity harness for the GPUI port.

Compares a screenshot of the React/Tauri app with one of the GPUI app, region by
region, and writes a Markdown report. Both screenshots are full-window captures;
size differences are reconciled before anything is measured.

Region geometry lives in a JSON manifest (``parity-regions.json`` next to this
file by default) as fractions of the window width/height, so one manifest works
at any resolution. Each region carries the React component it corresponds to,
so a report row points back at source.

Metrics per region (all computed on the reconciled, optionally downscaled pair):

* ``mean``  - mean absolute per-channel difference, over every channel sample
  (``ImageChops.difference`` + ``ImageStat``; 0..255).
* ``p95``   - 95th percentile of the per-pixel delta, where a pixel's delta is
  the largest absolute channel difference among R, G and B (nearest-rank, from
  the delta histogram).
* ``above`` - pixels whose delta is strictly greater than ``--threshold``
  (default 8/255), as a percentage of the region.

Verdicts: ``pass`` < 1%, ``review`` < 5%, ``fail`` >= 5% of pixels differing.

Only Pillow and the standard library are used.
"""

from __future__ import annotations

import argparse
import json
import math
import re
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Sequence

try:
    from PIL import Image, ImageChops, ImageStat
except ImportError:  # pragma: no cover - environment guard
    sys.stderr.write("Pillow is required: python -m pip install Pillow\n")
    raise

DEFAULT_THRESHOLD = 8
PASS_BELOW = 1.0
REVIEW_BELOW = 5.0
AMPLIFY = 4
AUTO_MAX_DIM = 2400
HERE = Path(__file__).resolve().parent
DEFAULT_REGIONS = HERE / "parity-regions.json"


@dataclass(frozen=True)
class Region:
    """A rectangular slice of the window, in 0..1 fractions."""

    name: str
    x: float
    y: float
    w: float
    h: float
    source: str


@dataclass
class Stats:
    total: int
    above: int
    above_pct: float
    mean: float
    p95: int


@dataclass
class RegionResult:
    region: Region
    box: tuple[int, int, int, int]
    stats: Stats
    verdict: str
    note: str | None = None


def load_regions(path: Path) -> list[Region]:
    """Load the manifest. Accepts a bare list or ``{"regions": [...]}``."""
    try:
        raw = json.loads(path.read_text(encoding="utf-8"))
    except FileNotFoundError:
        raise SystemExit(f"parity: region manifest not found: {path}")
    except json.JSONDecodeError as exc:
        raise SystemExit(f"parity: {path} is not valid JSON: {exc}")

    if isinstance(raw, dict):
        entries = raw.get("regions")
    else:
        entries = raw
    if not isinstance(entries, list):
        raise SystemExit(f"parity: {path} must contain a list of regions")

    regions: list[Region] = []
    seen: set[str] = set()
    for index, entry in enumerate(entries):
        if not isinstance(entry, dict):
            raise SystemExit(f"parity: region #{index} is not an object")
        missing = [key for key in ("name", "x", "y", "w", "h", "source") if key not in entry]
        if missing:
            raise SystemExit(f"parity: region #{index} missing {', '.join(missing)}")
        name = str(entry["name"])
        if name in seen:
            raise SystemExit(f"parity: duplicate region name {name!r}")
        seen.add(name)
        try:
            x, y, w, h = (float(entry[key]) for key in ("x", "y", "w", "h"))
        except (TypeError, ValueError):
            raise SystemExit(f"parity: region {name!r} has a non-numeric box")
        if w <= 0 or h <= 0:
            raise SystemExit(f"parity: region {name!r} has a non-positive size")
        if x < 0 or y < 0 or x + w > 1.0001 or y + h > 1.0001:
            raise SystemExit(
                f"parity: region {name!r} is not inside the 0..1 window "
                f"(x={x}, y={y}, w={w}, h={h})"
            )
        regions.append(Region(name, x, y, w, h, str(entry["source"])))
    if not regions:
        raise SystemExit(f"parity: {path} defines no regions")
    return regions


def _round(value: float) -> int:
    return int(math.floor(value + 0.5))


def region_box(region: Region, width: int, height: int) -> tuple[int, int, int, int]:
    """Pixel box for a region, clamped to the canvas."""
    left = max(0, min(width, _round(region.x * width)))
    top = max(0, min(height, _round(region.y * height)))
    right = max(0, min(width, _round((region.x + region.w) * width)))
    bottom = max(0, min(height, _round((region.y + region.h) * height)))
    return left, top, right, bottom


def _delta(a: Image.Image, b: Image.Image) -> tuple[Image.Image, Image.Image]:
    """Return (per-channel difference image, per-pixel max-channel delta)."""
    diff = ImageChops.difference(a, b)
    r, g, bl = diff.split()
    peak = ImageChops.lighter(ImageChops.lighter(r, g), bl)
    return diff, peak


def measure(a: Image.Image, b: Image.Image, threshold: int) -> Stats:
    diff, peak = _delta(a, b)
    total = a.width * a.height
    if total == 0:
        return Stats(0, 0, 0.0, 0.0, 0)

    histogram = peak.histogram()
    above = sum(histogram[threshold + 1:])

    cumulative = 0
    cutoff = 0.95 * total
    p95 = 255
    for value, count in enumerate(histogram):
        cumulative += count
        if cumulative >= cutoff:
            p95 = value
            break

    channel_means = ImageStat.Stat(diff).mean
    mean = sum(channel_means) / len(channel_means)
    return Stats(
        total=total,
        above=above,
        above_pct=100.0 * above / total,
        mean=mean,
        p95=p95,
    )


def verdict_for(above_pct: float) -> str:
    if above_pct < PASS_BELOW:
        return "pass"
    if above_pct < REVIEW_BELOW:
        return "review"
    return "fail"


def _resample(image: Image.Image, size: tuple[int, int]) -> Image.Image:
    method = getattr(Image, "Resampling", Image).LANCZOS
    return image.resize(size, method)


def reconcile(
    ts: Image.Image, gpui: Image.Image, scale: str
) -> tuple[Image.Image, Image.Image, list[str]]:
    """Make both images the same size, then apply the optional downscale.

    A matching aspect ratio resizes the GPUI capture onto the React canvas. A
    different aspect ratio crops both to the top-left common area instead, so no
    resampling invents pixels.
    """
    notes: list[str] = []
    a = ts.convert("RGB")
    b = gpui.convert("RGB")
    if a.size != b.size:
        (aw, ah), (bw, bh) = a.size, b.size
        aspect_a = aw / ah
        aspect_b = bw / bh
        if abs(aspect_a - aspect_b) <= 0.01 * max(aspect_a, aspect_b):
            b = _resample(b, a.size)
            notes.append(
                f"GPUI capture {bw}x{bh} was resized to the React capture's "
                f"{aw}x{ah} (aspect ratios within 1%)."
            )
        else:
            common = (min(aw, bw), min(ah, bh))
            a = a.crop((0, 0, *common))
            b = b.crop((0, 0, *common))
            notes.append(
                f"Captures had different aspect ratios ({aw}x{ah} vs {bw}x{bh}); "
                f"both were cropped to the {common[0]}x{common[1]} common area."
            )

    if scale == "auto":
        longest = max(a.width, a.height, b.width, b.height)
        factor = max(1, math.ceil(longest / AUTO_MAX_DIM))
    else:
        factor = int(scale)
    if factor > 1:
        size = (max(1, a.width // factor), max(1, a.height // factor))
        a = _resample(a, size)
        b = _resample(b, size)
        notes.append(
            f"Both captures were downscaled by {factor}x to {size[0]}x{size[1]} "
            f"(at most {AUTO_MAX_DIM}px on the long edge) before measuring."
        )
    return a, b, notes


def _slug(name: str) -> str:
    return re.sub(r"[^A-Za-z0-9._-]+", "-", name).strip("-") or "region"


def _diff_image(a: Image.Image, b: Image.Image) -> Image.Image:
    _diff, peak = _delta(a, b)
    return peak.point(lambda value: min(255, value * AMPLIFY))


def compare(
    ts_path: Path,
    gpui_path: Path,
    regions: Sequence[Region],
    threshold: int,
    scale: str,
    diff_dir: Path | None,
) -> tuple[str, list[RegionResult], list[str], Stats]:
    for path in (ts_path, gpui_path):
        if not path.is_file():
            raise SystemExit(f"parity: screenshot not found: {path}")

    with Image.open(ts_path) as ts_image, Image.open(gpui_path) as gpui_image:
        ts_size, gpui_size = ts_image.size, gpui_image.size
        a, b, notes = reconcile(ts_image, gpui_image, scale)

    if diff_dir is not None:
        diff_dir.mkdir(parents=True, exist_ok=True)

    window_stats = measure(a, b, threshold)
    results: list[RegionResult] = []
    warnings: list[str] = []

    for region in regions:
        box = region_box(region, a.width, a.height)
        left, top, right, bottom = box
        if right <= left or bottom <= top:
            warnings.append(
                f"region `{region.name}` lies outside the {a.width}x{a.height} "
                "comparison canvas and was skipped"
            )
            continue

        expected = (
            _round(region.x * a.width),
            _round(region.y * a.height),
            _round((region.x + region.w) * a.width),
            _round((region.y + region.h) * a.height),
        )
        note = None
        if box != expected:
            note = "clamped to the comparison canvas"
            warnings.append(
                f"region `{region.name}` extended past the image and was clamped "
                "to the visible area"
            )

        crop_a = a.crop(box)
        crop_b = b.crop(box)
        stats = measure(crop_a, crop_b, threshold)
        results.append(
            RegionResult(
                region=region,
                box=box,
                stats=stats,
                verdict=verdict_for(stats.above_pct),
                note=note,
            )
        )
        if diff_dir is not None:
            _diff_image(crop_a, crop_b).save(
                diff_dir / f"{_slug(region.name)}.png"
            )

    results.sort(key=lambda result: result.stats.above_pct, reverse=True)

    header = [
        "# Visual parity report",
        "",
        f"- React capture: `{ts_path}` ({ts_size[0]}x{ts_size[1]})",
        f"- GPUI capture: `{gpui_path}` ({gpui_size[0]}x{gpui_size[1]})",
    ]
    if notes:
        header.extend(f"- {note}" for note in notes)
    else:
        header.append("- Size reconciliation: none needed (identical dimensions).")
    header.append(
        f"- Metric: per-pixel delta is the largest absolute R/G/B channel "
        f"difference; a pixel *differs* when that delta exceeds {threshold}/255."
    )
    header.append(
        f"- Verdicts: **pass** < {PASS_BELOW:g}% differing, **review** < "
        f"{REVIEW_BELOW:g}%, **fail** >= {REVIEW_BELOW:g}%."
    )
    if diff_dir is not None:
        header.append(
            f"- Diff images: `{diff_dir}` (per region, greyscale, delta amplified "
            f"{AMPLIFY}x)."
        )
    header.append("")

    table = [
        "| Region | Source | Mean \u0394 (0-255) | P95 \u0394 | Differing | Verdict |",
        "|---|---|---:|---:|---:|---|",
    ]
    for result in results:
        stats = result.stats
        differing = f"{stats.above_pct:.2f}%"
        if result.note:
            differing += f" ({result.note})"
        table.append(
            f"| {result.region.name} | `{result.region.source}` | "
            f"{stats.mean:.2f} | {stats.p95} | {differing} | "
            f"**{result.verdict}** |"
        )
    if not results:
        table.append("| _(none)_ | | | | | |")

    summary = [
        "",
        f"**Summary:** {window_stats.above:,} of {window_stats.total:,} window "
        f"pixels differ by more than {threshold}/255 "
        f"({window_stats.above_pct:.2f}% of the window).",
    ]
    if results:
        worst = results[0]
        summary.append(
            f"**Worst region:** `{worst.region.name}` "
            f"({worst.region.source}) at {worst.stats.above_pct:.2f}% differing, "
            f"mean \u0394 {worst.stats.mean:.2f}, P95 \u0394 {worst.stats.p95}."
        )
    else:
        summary.append("**Worst region:** none compared.")

    parts = header + table + summary
    if warnings:
        parts.extend(["", "## Warnings", ""] + [f"- {warning}" for warning in warnings])
    parts.append("")
    return "\n".join(parts), results, warnings, window_stats


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description="Compare a React screenshot with a GPUI screenshot region by region.",
        epilog=(
            "Region manifests use 0..1 fractions of the window width/height, so the "
            "same manifest works at any resolution. Defaults to parity-regions.json "
            "next to this script."
        ),
    )
    parser.add_argument("--ts", required=True, type=Path, help="full-window React capture")
    parser.add_argument("--gpui", required=True, type=Path, help="full-window GPUI capture")
    parser.add_argument(
        "--regions",
        type=Path,
        default=DEFAULT_REGIONS,
        help=f"region manifest (default: {DEFAULT_REGIONS.name})",
    )
    parser.add_argument(
        "--out", type=Path, default=None, help="write the Markdown report here (default: stdout)"
    )
    parser.add_argument(
        "--diff-dir",
        type=Path,
        default=None,
        help="write one amplified diff image per region into this directory",
    )
    parser.add_argument(
        "--threshold",
        type=int,
        default=DEFAULT_THRESHOLD,
        help=f"per-pixel channel delta counted as differing (default: {DEFAULT_THRESHOLD})",
    )
    parser.add_argument(
        "--scale",
        default="auto",
        help=(
            "downscale factor applied to both captures after reconciliation; "
            f"'auto' keeps 1x until the long edge exceeds {AUTO_MAX_DIM}px (default: auto)"
        ),
    )
    return parser


def parse_args(argv: Sequence[str] | None) -> argparse.Namespace:
    parser = build_parser()
    args = parser.parse_args(argv)
    if args.scale != "auto":
        try:
            factor = int(args.scale)
        except ValueError:
            parser.error("--scale must be 'auto' or a positive integer")
        if factor < 1:
            parser.error("--scale must be 'auto' or a positive integer")
        args.scale = str(factor)
    if not 0 <= args.threshold <= 255:
        parser.error("--threshold must be between 0 and 255")
    return args


def main(argv: Sequence[str] | None = None) -> int:
    args = parse_args(argv)
    regions = load_regions(args.regions)
    report, _results, warnings, _window = compare(
        args.ts, args.gpui, regions, args.threshold, args.scale, args.diff_dir
    )
    if args.out is not None:
        args.out.parent.mkdir(parents=True, exist_ok=True)
        args.out.write_text(report, encoding="utf-8")
    else:
        if hasattr(sys.stdout, "reconfigure"):
            sys.stdout.reconfigure(encoding="utf-8")
        sys.stdout.write(report)
    for warning in warnings:
        sys.stderr.write(f"parity: warning: {warning}\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
