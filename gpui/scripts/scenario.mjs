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
//   --model <path>     the mesh the app opens (default: test-cube.stl)
//   --bin <path>       the executable (default: target/debug/dragonfruit-gpui)
//   --list             print the shipped scenarios and exit
//
// Each run prints the scenario's Markdown report on stdout. The runner exits 0
// only when every scenario exited 0; a failing `expect` makes the app exit 1.
// The negative fixture proves that gate:
//
//   node gpui/scripts/scenario.mjs --scenario scenarios/invalid/wrong-expect.json
//   # must exit 1 and print the expected/actual pair

import { spawn } from 'node:child_process';
import { existsSync, readdirSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const scriptDir = dirname(fileURLToPath(import.meta.url));
const crateRoot = resolve(scriptDir, '..');
const scenarioDir = resolve(crateRoot, 'scenarios');

const defaultBinary = resolve(
  crateRoot,
  'target',
  'debug',
  process.platform === 'win32' ? 'dragonfruit-gpui.exe' : 'dragonfruit-gpui',
);
const defaultModel = resolve(crateRoot, 'test-cube.stl');

/** The scenarios this port ships, in the order the report lists them. */
export const shipped = ['app-bar', 'support-rail', 'new-scene'];

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
    binary: defaultBinary,
    model: defaultModel,
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
      default:
        if (argument.startsWith('-')) usage(`unknown option ${argument}`);
        options.names.push(argument);
    }
  }
  return options;
}

function run(binary, scenario, model) {
  return new Promise((resolveExit) => {
    const child = spawn(binary, ['--scenario', scenario, model], {
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
  if (!existsSync(options.model)) usage(`no such model: ${options.model}`);
  if (!existsSync(options.binary)) {
    console.error(
      `scenario: ${options.binary} is missing; build it first (cargo build -p dragonfruit-gpui)`,
    );
    return 2;
  }

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
