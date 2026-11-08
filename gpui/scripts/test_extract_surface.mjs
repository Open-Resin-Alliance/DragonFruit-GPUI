// Tests for the surface extractor and the coverage gate.
//
//   node --test gpui/scripts/test_extract_surface.mjs
//
// The fixture is parsed through the same AST path as the app; the checker is
// exercised both as a pure function and through its CLI exit code.

import { test } from 'node:test';
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

import { extractFromSource, isSurfaceSourcePath } from './extract-surface.mjs';
import { checkCoverage, areaTotals, oldestAbsent } from './check-coverage.mjs';

const scriptDir = dirname(fileURLToPath(import.meta.url));
const checkerPath = resolve(scriptDir, 'check-coverage.mjs');

const FIXTURE = `
type FixtureProps = {
  isOpen: boolean;
  busy: boolean;
  showDialog: boolean;
  onConfirm: () => void;
  onRetry: () => void;
  onRetryAll: () => void;
  onHiddenChange: () => void;
  setClosed: (closed: boolean) => void;
};

export function Fixture({
  isOpen,
  busy,
  showDialog,
  onConfirm,
  onRetry,
  onRetryAll,
  onHiddenChange,
  setClosed,
}: FixtureProps) {
  if (!isOpen) {
    return <div>Nothing here yet</div>;
  }

  if (busy) return null;

  return (
    <div className="panel">
      <button aria-label="Close panel" onClick={() => setClosed(true)}>
        <XIcon />
      </button>

      <input className="hidden pointer-events-none" onChange={onHiddenChange} />

      {showDialog && (
        <div role="dialog" aria-label="Confirm removal" className="fixed inset-0">
          <button disabled={busy} onClick={onConfirm}>
            Apply
          </button>
        </div>
      )}

      <a href="/docs">Read the docs</a>

      <button onClick={onRetry}>Retry</button>
      <button onClick={onRetryAll}>Retry</button>
    </div>
  );
}
`;

function extractFixture() {
  return extractFromSource(FIXTURE, 'src/components/fixtures/Fixture.tsx');
}

const byId = (rows, id) => rows.find((row) => row.id === id);

test('extracts the full fixture surface', () => {
  const { rows } = extractFixture();
  const ids = rows.map((row) => row.id).sort();
  assert.deepEqual(ids, [
    'fixtures/apply',
    'fixtures/close-panel',
    'fixtures/confirm-removal',
    'fixtures/no-render',
    'fixtures/nothing-here-yet',
    'fixtures/onhiddenchange',
    'fixtures/read-the-docs',
    'fixtures/retry',
    'fixtures/retry--fixture',
  ]);
});

test('an aria-label names an icon-only button', () => {
  const { rows } = extractFixture();
  const close = byId(rows, 'fixtures/close-panel');
  assert.ok(close, 'close button row exists');
  assert.equal(close.kind, 'button');
  assert.equal(close.label, 'Close panel');
  assert.equal(close.action, 'onClick:setClosed');
  assert.equal(close.source, 'src/components/fixtures/Fixture.tsx:31');
  assert.deepEqual(close.gates, []);
});

test('a conditional modal is a container gated by its condition', () => {
  const { rows } = extractFixture();
  const dialog = byId(rows, 'fixtures/confirm-removal');
  assert.ok(dialog, 'dialog row exists');
  assert.equal(dialog.kind, 'modal');
  assert.ok(dialog.gates.includes('when:showDialog'), `gates: ${dialog.gates.join(', ')}`);
});

test('disabled and class-based gates are attached to the control', () => {
  const { rows } = extractFixture();
  const apply = byId(rows, 'fixtures/apply');
  assert.ok(apply, 'apply row exists');
  assert.deepEqual(apply.gates.sort(), ['disabled:busy', 'when:showDialog']);

  const hiddenInput = byId(rows, 'fixtures/onhiddenchange');
  assert.ok(hiddenInput, 'hidden input row exists');
  assert.equal(hiddenInput.label, 'onHiddenChange');
  assert.ok(hiddenInput.gates.includes('class:hidden'), `gates: ${hiddenInput.gates.join(', ')}`);
  assert.ok(hiddenInput.gates.includes('class:pointer-events-none'), `gates: ${hiddenInput.gates.join(', ')}`);
});

test('early returns produce empty-state rows with the guard as a gate', () => {
  const { rows } = extractFixture();
  const empty = byId(rows, 'fixtures/nothing-here-yet');
  assert.ok(empty, 'empty-state row exists');
  assert.equal(empty.kind, 'empty-state');
  assert.equal(empty.label, 'Nothing here yet');
  assert.ok(empty.gates.includes('early-return:!isOpen'), `gates: ${empty.gates.join(', ')}`);

  const nothing = byId(rows, 'fixtures/no-render');
  assert.ok(nothing, 'null early-return row exists');
  assert.equal(nothing.kind, 'empty-state');
  assert.ok(nothing.gates.includes('early-return:busy'), `gates: ${nothing.gates.join(', ')}`);
});

test('duplicate labels are kept and reported as a collision', () => {
  const { rows, collisions } = extractFixture();
  assert.ok(byId(rows, 'fixtures/retry'), 'first Retry keeps the plain id');
  assert.ok(byId(rows, 'fixtures/retry--fixture'), 'second Retry is disambiguated');
  assert.equal(collisions.length, 1);
  assert.equal(collisions[0].id, 'fixtures/retry');
  assert.equal(collisions[0].resolved, 'fixtures/retry--fixture');
});

test('checker fails on a missing row and passes on a complete ledger', () => {
  const surface = extractFixture();
  const complete = {
    rows: surface.rows.map((row) => ({ id: row.id, source: row.source, status: 'absent', rust: null })),
  };

  const pass = checkCoverage(surface, complete);
  assert.equal(pass.ok, true);
  assert.deepEqual(pass.missing, []);

  const incomplete = { rows: complete.rows.slice(0, -1) };
  const fail = checkCoverage(surface, incomplete);
  assert.equal(fail.ok, false);
  assert.equal(fail.missing.length, 1);
  assert.equal(fail.missing[0].id, surface.rows.at(-1).id);
  assert.ok(fail.missing[0].source);
});

test('checker CLI exits 1 for a missing row and 0 for a complete ledger', () => {
  const dir = mkdtempSync(join(tmpdir(), 'gpui-surface-'));
  try {
    const surface = extractFixture();
    const surfacePath = join(dir, 'surface.json');
    const ledgerPath = join(dir, 'ledger.json');
    writeFileSync(surfacePath, JSON.stringify(surface));

    const complete = {
      version: 1,
      rows: surface.rows.map((row) => ({ id: row.id, source: row.source, status: 'absent', rust: null })),
    };
    writeFileSync(ledgerPath, JSON.stringify(complete));

    const ok = spawnSync(process.execPath, [checkerPath, '--surface', surfacePath, '--ledger', ledgerPath], {
      encoding: 'utf8',
    });
    assert.equal(ok.status, 0, ok.stderr);
    assert.match(ok.stdout, /all surface ids are in the ledger/);

    const missingRow = surface.rows[surface.rows.length - 1];
    writeFileSync(
      ledgerPath,
      JSON.stringify({ version: 1, rows: complete.rows.filter((row) => row.id !== missingRow.id) }),
    );
    const failed = spawnSync(process.execPath, [checkerPath, '--surface', surfacePath, '--ledger', ledgerPath], {
      encoding: 'utf8',
    });
    assert.equal(failed.status, 1);
    assert.match(failed.stderr, /MISSING/);
    assert.match(failed.stderr, new RegExp(missingRow.id.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')));
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test('area totals and oldest-absent ordering are derived from the ledger', () => {
  const ledger = {
    rows: [
      { id: 'b/one', status: 'absent' },
      { id: 'a/two', status: 'absent', since: '2026-01-02' },
      { id: 'a/three', status: 'wired' },
      { id: 'a/four', status: 'absent', since: '2026-01-01' },
    ],
  };
  const totals = areaTotals(ledger);
  assert.deepEqual(
    totals.map((entry) => [entry.area, entry.total]),
    [['a', 3], ['b', 1]],
  );
  assert.equal(totals[0].statuses.wired, 1);

  const oldest = oldestAbsent(ledger, 3).map((row) => row.id);
  assert.deepEqual(oldest, ['a/four', 'a/two', 'b/one']);
});

// A feature directory holds whole panels that the component tree only mounts
// (`<OrganicCutPanel />` in PreparePanelStack, `<UpdatesSettingsTab />` in
// SettingsModal), so walking `src/components` alone captured the mounts and
// dropped every control inside the bodies.
test('feature files are in scope and app files other than page.tsx are not', () => {
  assert.equal(isSurfaceSourcePath('src/features/organicCut/OrganicCutPanel.tsx'), true);
  assert.equal(isSurfaceSourcePath('src/features/slicing/components/SlicingPanel.tsx'), true);
  assert.equal(isSurfaceSourcePath('src/features/updater/UpdatesSettingsTab.tsx'), true);
  assert.equal(isSurfaceSourcePath('src/components/settings/PerformanceSettingsTab.tsx'), true);
  assert.equal(isSurfaceSourcePath('src/app/page.tsx'), true);

  assert.equal(isSurfaceSourcePath('src/app/layout.tsx'), false, 'only page.tsx is in scope under src/app');
  assert.equal(isSurfaceSourcePath('src/supports/Settings/SupportSidebar.tsx'), false, 'src/supports is out of scope');
  assert.equal(isSurfaceSourcePath('src/features/organicCut/types.ts'), false, 'only .tsx is parsed');
  assert.equal(isSurfaceSourcePath('src/features/organicCut/OrganicCutPanel.test.tsx'), false);
  assert.equal(isSurfaceSourcePath('src/features/organicCut/OrganicCutPanel.dom-test.tsx'), false);
  assert.equal(isSurfaceSourcePath('src/features/organicCut/OrganicCutPanel.stories.tsx'), false);
  assert.equal(isSurfaceSourcePath('src/features/organicCut/__tests__/OrganicCutPanel.dom-test.tsx'), false);
});

// The four gaps were file-discovery, not traversal: each wrapper shape below
// already reaches its controls. The fixture pins that so a future traversal
// change cannot silently start dropping them.
const WRAPPER_FIXTURE = `
export default function DefaultExport() {
  return (
    <div>
      <button onClick={go}>Default Go</button>
    </div>
  );
}

export function TypedReturn(): JSX.Element {
  return (
    <div>
      <button onClick={go}>Typed Go</button>
    </div>
  );
}

export const Memoized = memo(
  forwardRef((props: Props, ref: Ref<HTMLElement>) => (
    <div>
      <button onClick={go}>Memo Go</button>
    </div>
  )),
);

export function ConditionalMount({ open }: { open: boolean }) {
  const body = open ? <SettingsTab title="Conditional Tab" /> : null;
  return <div>{body}</div>;
}
`;

test('default exports, typed returns, memo/forwardRef wrappers and conditional mounts keep their controls', () => {
  const { rows } = extractFromSource(WRAPPER_FIXTURE, 'src/components/fixtures/Wrappers.tsx');
  const ids = rows.map((row) => row.id).sort();
  assert.deepEqual(ids, ['fixtures/conditional-tab', 'fixtures/default-go', 'fixtures/memo-go', 'fixtures/typed-go']);

  const tab = byId(rows, 'fixtures/conditional-tab');
  assert.equal(tab.kind, 'tab');
  assert.ok(tab.gates.includes('when:open'), `gates: ${tab.gates.join(', ')}`);
});

test('a feature panel is named for its feature, not for its leaf directory', () => {
  const source = `
export function SlicingPanel() {
  return (
    <div>
      <button onClick={sliceAll}>Slice All</button>
    </div>
  );
}
`;
  const { rows } = extractFromSource(source, 'src/features/slicing/components/SlicingPanel.tsx');
  assert.deepEqual(
    rows.map((row) => row.id),
    ['slicing/slice-all'],
  );
  assert.equal(rows[0].source, 'src/features/slicing/components/SlicingPanel.tsx:5');
});
