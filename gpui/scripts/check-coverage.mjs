#!/usr/bin/env node
// Gate between the extracted surface and the coverage ledger.
//
//   node gpui/scripts/check-coverage.mjs
//   node gpui/scripts/check-coverage.mjs --surface gpui/scripts/surface.json \
//                                        --ledger docs/internal/gpui-coverage.json
//
// Fails (exit 1) when an id in surface.json has no row in the ledger, so a new
// button cannot be forgotten. Otherwise prints per-area totals and the ten
// oldest `absent` rows and exits 0.
//
// "Oldest" is ledger order: the ledger is seeded ordered by area, and a row
// that is still `absent` has been waiting since it entered the file. A row may
// carry a `since` date; rows with dates sort before rows without, so age is
// preserved once agents start stamping rows.

import { existsSync, readFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const scriptDir = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(scriptDir, '..', '..');

export const defaultSurfacePath = resolve(scriptDir, 'surface.json');
export const defaultLedgerPath = resolve(repoRoot, 'docs', 'internal', 'gpui-coverage.json');

const compareStrings = (a, b) => (a < b ? -1 : a > b ? 1 : 0);

export function areaOfId(id) {
  const slash = id.indexOf('/');
  return slash === -1 ? '(none)' : id.slice(0, slash);
}

/**
 * Pure check: every surface row must have a ledger row. Returns the missing
 * rows and any duplicate ledger ids; `ok` is true only when nothing is missing.
 */
export function checkCoverage(surface, ledger) {
  const ledgerRows = Array.isArray(ledger) ? ledger : (ledger?.rows ?? []);
  const surfaceRows = Array.isArray(surface) ? surface : (surface?.rows ?? []);
  const ledgerIds = new Set();
  const duplicates = [];
  for (const row of ledgerRows) {
    if (!row?.id) continue;
    if (ledgerIds.has(row.id)) duplicates.push(row.id);
    ledgerIds.add(row.id);
  }
  const missing = surfaceRows
    .filter((row) => row?.id && !ledgerIds.has(row.id))
    .map((row) => ({ id: row.id, source: row.source, kind: row.kind, label: row.label }));
  return { ok: missing.length === 0, missing, duplicates, surfaceTotal: surfaceRows.length, ledgerTotal: ledgerRows.length };
}

/** Per-area totals with a status breakdown. */
export function areaTotals(ledger) {
  const rows = Array.isArray(ledger) ? ledger : (ledger?.rows ?? []);
  const totals = new Map();
  for (const row of rows) {
    if (!row?.id) continue;
    const area = areaOfId(row.id);
    if (!totals.has(area)) totals.set(area, { area, total: 0, statuses: {} });
    const entry = totals.get(area);
    entry.total += 1;
    const status = row.status ?? 'unknown';
    entry.statuses[status] = (entry.statuses[status] ?? 0) + 1;
  }
  return [...totals.values()].sort((a, b) => compareStrings(a.area, b.area));
}

/** The ten absent rows that have been waiting longest. */
export function oldestAbsent(ledger, limit = 10) {
  const rows = Array.isArray(ledger) ? ledger : (ledger?.rows ?? []);
  const absent = rows.filter((row) => row?.status === 'absent');
  const sorted = absent
    .map((row, index) => ({ row, index }))
    .sort((a, b) => {
      const aSince = a.row.since ?? null;
      const bSince = b.row.since ?? null;
      if (aSince && bSince) return compareStrings(String(aSince), String(bSince)) || a.index - b.index;
      if (aSince) return -1;
      if (bSince) return 1;
      return a.index - b.index;
    })
    .map(({ row }) => row);
  return sorted.slice(0, limit);
}

function parseArgv(argv) {
  const options = { surface: defaultSurfacePath, ledger: defaultLedgerPath };
  for (let i = 0; i < argv.length; i += 1) {
    const arg = argv[i];
    if (arg === '--surface' || arg === '--ledger') {
      const value = argv[i + 1];
      if (!value) throw new Error(`${arg} needs a path`);
      options[arg.slice(2)] = resolve(process.cwd(), value);
      i += 1;
    } else if (arg.startsWith('--surface=')) {
      options.surface = resolve(process.cwd(), arg.slice('--surface='.length));
    } else if (arg.startsWith('--ledger=')) {
      options.ledger = resolve(process.cwd(), arg.slice('--ledger='.length));
    } else if (arg === '--help' || arg === '-h') {
      options.help = true;
    } else {
      throw new Error(`Unknown argument: ${arg}`);
    }
  }
  return options;
}

export function readJson(path, what) {
  if (!existsSync(path)) throw new Error(`${what} not found: ${path}`);
  return JSON.parse(readFileSync(path, 'utf8'));
}

function main() {
  const options = parseArgv(process.argv.slice(2));
  if (options.help) {
    process.stdout.write('Usage: node gpui/scripts/check-coverage.mjs [--surface <path>] [--ledger <path>]\n');
    return 0;
  }

  let surface;
  let ledger;
  try {
    surface = readJson(options.surface, 'surface.json');
    ledger = readJson(options.ledger, 'ledger');
  } catch (error) {
    process.stderr.write(`${error.message}\n`);
    return 1;
  }

  const result = checkCoverage(surface, ledger);
  const totals = areaTotals(ledger);

  process.stdout.write(`surface: ${result.surfaceTotal} rows   ledger: ${result.ledgerTotal} rows\n`);
  for (const entry of totals) {
    const breakdown = Object.entries(entry.statuses)
      .sort(([a], [b]) => compareStrings(a, b))
      .map(([status, count]) => `${status} ${count}`)
      .join(', ');
    process.stdout.write(`  ${entry.area.padEnd(18)} ${String(entry.total).padStart(5)}  ${breakdown}\n`);
  }

  const absent = oldestAbsent(ledger, 10);
  if (absent.length) {
    process.stdout.write('\noldest absent rows:\n');
    for (const row of absent) {
      process.stdout.write(`  ${row.id.padEnd(56)} ${row.source ?? ''}\n`);
    }
  }

  if (result.duplicates.length) {
    process.stderr.write(`\nwarning: ${result.duplicates.length} duplicate ledger id(s): ${result.duplicates.slice(0, 5).join(', ')}\n`);
  }

  if (!result.ok) {
    process.stderr.write(`\n${result.missing.length} surface id(s) missing from the ledger:\n`);
    for (const row of result.missing) {
      process.stderr.write(`  MISSING ${row.id}  (${row.source ?? '?'})\n`);
    }
    process.stderr.write('\nAdd the row to docs/internal/gpui-coverage.json (status \"absent\") or rerun extract-surface.mjs --seed-ledger.\n');
    return 1;
  }

  process.stdout.write('\nall surface ids are in the ledger\n');
  return 0;
}

const invokedDirectly = process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url);
if (invokedDirectly) {
  try {
    process.exitCode = main();
  } catch (error) {
    process.stderr.write(`${error.message}\n`);
    process.exitCode = 1;
  }
}
