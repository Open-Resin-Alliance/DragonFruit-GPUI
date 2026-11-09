#!/usr/bin/env node
// Runs the shell's automation scenarios: real clicks and keys through GPUI's
// input pipeline, with a state snapshot after every step.
//
//   node gpui/scripts/scenario.mjs                 # all three shipped scenarios
//   node gpui/scripts/scenario.mjs app-bar         # one, by file stem
//   node gpui/scripts/scenario.mjs --scenario path/to/custom.json
//   node gpui/scripts/scenario.mjs --list
//
// Options:
//   --scenario <path>  a scenario file instead of a shipped one
//   --model <path>     the mesh the app opens, overriding the scenario's own
//                      "model" field (a scenario may declare one)
//   --bin <path>       the executable (default: the newest built
//                      dragonfruit-gpui in target/, target-check/, target-verify/)
//   --allow-stale      run even when the executable predates the driver sources
//   --list             print the shipped scenarios and exit
//
// Each run prints the scenario's Markdown report on stdout. The runner exits 0
// only when every scenario exited 0; a failing `expect` makes the app exit 1.
// The negative fixture proves that gate:
//
//   node gpui/scripts/scenario.mjs --scenario scenarios/invalid/wrong-expect.json
//   # must exit 1 and print the expected/actual pair

import { spawn } from 'node:child_process';
import { existsSync, readdirSync, statSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const scriptDir = dirname(fileURLToPath(import.meta.url));
const crateRoot = resolve(scriptDir, '..');
const scenarioDir = resolve(crateRoot, 'scenarios');
const executable =
  process.platform === 'win32' ? 'dragonfruit-gpui.exe' : 'dragonfruit-gpui';

// A gate that runs a stale executable reports yesterday's code. Every target
// directory this worktree uses is a candidate and the newest wins; `--bin`
// overrides the choice.
const binaryCandidates = ['target', 'target-check', 'target-verify'].map((dir) =>
  resolve(crateRoot, dir, 'debug', executable),
);

// The sources a scenario exercises. A binary older than either cannot be
// trusted to contain the driver, so the run stops instead of reporting a pass.
const scenarioSources = ['src/automation.rs', 'src/app.rs'].map((path) =>
  resolve(crateRoot, path),
);

function newestOf(paths) {
  let newest = 0;
  for (const path of paths) {
    try {
      newest = Math.max(newest, statSync(path).mtimeMs);
    } catch {
      // A missing file simply does not contribute an mtime.
    }
  }
  return newest;
}

function defaultBinary() {
  const existing = binaryCandidates.filter((path) => existsSync(path));
  if (existing.length === 0) return binaryCandidates[0];
  return existing.sort((a, b) => newestOf([b]) - newestOf([a]))[0];
}

// The scenarios this port ships, in the order the report lists them.
export const shipped = [
  'app-bar',
  'support-rail',
  'new-scene',
  'deselect',
  'profiles-modal',
];

function usage(message) {
  if (message) console.error(`scenario: ${message}`);
  console.error(
    'usage: node gpui/scripts/scenario.mjs [app-bar|support-rail|new-scene ...] ' +
      '[--scenario <path>] [--model <path>] [--bin <path>] [--list]',
  );
  process.exit(2);
}

/**
 * Resolve a path argument from the caller's cwd, then the crate, then the
 * scenario directory, so the documented root-relative form works as well as a
 * bare file name.
 */
function resolveInput(value) {
  const candidates = [
    resolve(process.cwd(), value),
    resolve(crateRoot, value),
    resolve(scenarioDir, value),
  ];
  return candidates.find((candidate) => existsSync(candidate)) ?? candidates[0];
}

function parseArgs(argv) {
  const options = {
    list: false,
    binary: defaultBinary(),
    model: null,
    allowStale: false,
    paths: [],
    names: [],
  };
  for (let index = 0; index < argv.length; index += 1) {
    const argument = argv[index];
    const equals = argument.indexOf('=');
    const [flag, inline] = equals === -1 ? [argument, undefined] : [argument.slice(0, equals), argument.slice(equals + 1)];
    const value = () => {
      if (inline !== undefined) return inline;
      index += 1;
      if (index >= argv.length) usage(`${flag} needs a value`);
      return argv[index];
    };
    switch (flag) {
      case '--scenario':
        options.paths.push(resolveInput(value()));
        break;
      case '--model':
        options.model = resolveInput(value());
        break;
      case '--bin':
        options.binary = resolve(process.cwd(), value());
        break;
      case '--list':
        options.list = true;
        break;
      case '--allow-stale':
        options.allowStale = true;
        break;
      default:
        if (argument.startsWith('-')) usage(`unknown option ${argument}`);
        options.names.push(argument);
    }
  }
  return options;
}

function run(binary, scenario, model) {
  const args = ['--scenario', scenario];
  // Without `--model` the app falls back to the scenario's own "model" field,
  // which is what makes `--scenario <file>` work on its own.
  if (model) args.push(model);
  return new Promise((resolveExit) => {
    const child = spawn(binary, args, {
      cwd: crateRoot,
      stdio: 'inherit',
    });
    child.on('error', (error) => {
      console.error(`scenario: could not start ${binary}: ${error.message}`);
      resolveExit(127);
    });
    child.on('exit', (code, signal) => {
      resolveExit(signal ? 1 : (code ?? 1));
    });
  });
}

async function main() {
  const options = parseArgs(process.argv.slice(2));

  if (options.list) {
    const files = readdirSync(scenarioDir)
      .filter((name) => name.endsWith('.json'))
      .map((name) => name.slice(0, -'.json'.length))
      .sort();
    for (const name of files) console.log(name);
    return 0;
  }

  const scenarios = [];
  for (const path of options.paths) scenarios.push({ label: path, path });
  for (const name of options.names) {
    const path = resolve(scenarioDir, `${name}.json`);
    if (!existsSync(path)) usage(`no such scenario: ${name}`);
    scenarios.push({ label: name, path });
  }
  if (scenarios.length === 0) {
    for (const name of shipped) {
      scenarios.push({ label: name, path: resolve(scenarioDir, `${name}.json`) });
    }
  }
  for (const { path } of scenarios) {
    if (!existsSync(path)) usage(`no such scenario file: ${path}`);
  }
  if (options.model && !existsSync(options.model)) usage(`no such model: ${options.model}`);
  if (!existsSync(options.binary)) {
    console.error(
      `scenario: ${options.binary} is missing; build it first (cargo build -p dragonfruit-gpui)`,
    );
    return 2;
  }
  // A binary that predates the driver silently runs the previous code (or no
  // scenario at all). Refuse rather than report a meaningless pass.
  const binaryTime = newestOf([options.binary]);
  const sourceTime = newestOf(scenarioSources);
  if (!options.allowStale && sourceTime > binaryTime) {
    const age = ((sourceTime - binaryTime) / 1000).toFixed(1);
    console.error(
      `scenario: ${options.binary} is ${age}s older than src/automation.rs or src/app.rs; ` +
        'rebuild before running (cargo build -p dragonfruit-gpui), or pass --allow-stale',
    );
    return 2;
  }
  console.log(`scenario: binary ${options.binary}`);

  let failed = 0;
  for (const scenario of scenarios) {
    console.log(`\n=== ${scenario.label} ===`);
    const code = await run(options.binary, scenario.path, options.model);
    if (code !== 0) failed += 1;
  }

  if (failed === 0) {
    console.log(`\nscenario: ${scenarios.length} passed`);
  } else {
    console.error(`\nscenario: ${failed} of ${scenarios.length} failed`);
  }
  return failed === 0 ? 0 : 1;
}

process.exitCode = await main();
