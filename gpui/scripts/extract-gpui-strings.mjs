#!/usr/bin/env node
// Extracts every `t!("…")` literal from the GPUI port into a PO-compatible
// list, so the strings that reach a screen can be handed to `lingui extract`
// alongside the React ones.
//
//   node gpui/scripts/extract-gpui-strings.mjs            > gpui-strings.po
//   node gpui/scripts/extract-gpui-strings.mjs --stats    (also prints a summary)
//
// The English literal IS the msgid (that is what `gpui-locale`'s `t!` looks up),
// so every entry is emitted with an empty `msgstr`: this file is a POT, not a
// translation. `gpui-locale/src/catalog_data.rs` is generated from the PO files
// under `src/locales/`, which stay the translation source of truth.
//
// Plain Node: `fs` and `path` only, no dependencies, not wired into CI yet.
//
// What it scans: `gpui/src/**` and `gpui/crates/**`, `.rs` files, both macro
// forms —
//
//   t!("Add printer")
//   t!(catalog, "Add printer")
//
// It is a text scan, not a Rust parser, but it is careful about the cases that
// would otherwise invent messages: comments (including `///` examples) and the
// contents of string literals are masked before matching, so only real call
// sites are read. The catalogue argument is recognised when it is a plain path
// such as `catalog`, `self.catalog` or `crate::state::catalog`.

import { readdirSync, readFileSync, statSync } from "node:fs";
import { dirname, join, relative, resolve, sep } from "node:path";
import { fileURLToPath } from "node:url";

const scriptDir = dirname(fileURLToPath(import.meta.url));
const gpuiDir = resolve(scriptDir, "..");
const roots = [resolve(gpuiDir, "src"), resolve(gpuiDir, "crates")];

const SKIP_DIRS = new Set(["target", "target-check", "target-verify", "node_modules", ".git"]);

/**
 * A copy of `source` of the same length where comments and the contents of
 * string (and char) literals are spaces and newlines are kept, so a regex only
 * sees code and reported line numbers still match the file.
 */
function mask(source) {
  const out = source.split("");
  const blank = (from, to) => {
    for (let i = from; i < to; i += 1) {
      if (out[i] !== "\n") out[i] = " ";
    }
  };

  let i = 0;
  while (i < source.length) {
    const char = source[i];
    const next = source[i + 1];

    if (char === "/" && next === "/") {
      const end = source.indexOf("\n", i);
      const stop = end === -1 ? source.length : end;
      blank(i, stop);
      i = stop;
      continue;
    }

    if (char === "/" && next === "*") {
      let depth = 0;
      let j = i;
      while (j < source.length) {
        if (source[j] === "/" && source[j + 1] === "*") {
          depth += 1;
          j += 2;
        } else if (source[j] === "*" && source[j + 1] === "/") {
          depth -= 1;
          j += 2;
          if (depth === 0) break;
        } else {
          j += 1;
        }
      }
      blank(i, j);
      i = j;
      continue;
    }

    // Raw strings: r"…", r#"…"#, br#"…"#.
    if ((char === "r" && (next === '"' || next === "#")) || (char === "b" && next === "r")) {
      const rawStart = char === "b" ? i + 1 : i;
      let cursor = rawStart + 1;
      let hashes = 0;
      while (source[cursor] === "#") {
        hashes += 1;
        cursor += 1;
      }
      if (source[cursor] === '"') {
        const closing = `"${"#".repeat(hashes)}`;
        const end = source.indexOf(closing, cursor + 1);
        const stop = end === -1 ? source.length : end + closing.length;
        blank(i + 1, stop - closing.length);
        i = stop;
        continue;
      }
    }

    if (char === '"') {
      let j = i + 1;
      let contentStart = j;
      while (j < source.length) {
        if (source[j] === "\\") {
          j += 2;
        } else if (source[j] === '"' || source[j] === "\n") {
          break;
        } else {
          j += 1;
        }
      }
      blank(contentStart, j);
      i = j + 1;
      continue;
    }

    // Char literals (`'{'`) vs lifetimes (`'a`).
    if (char === "'") {
      const literal = /^'(?:\\.|[^'\\\n])'/.exec(source.slice(i, i + 8));
      if (literal) {
        blank(i + 1, i + literal[0].length - 1);
        i += literal[0].length;
        continue;
      }
    }

    i += 1;
  }

  return out.join("");
}

/** The value of a Rust string literal body. */
function unescape(body) {
  let out = "";
  for (let i = 0; i < body.length; i += 1) {
    const char = body[i];
    if (char !== "\\") {
      out += char;
      continue;
    }
    const next = body[i + 1];
    i += 1;
    switch (next) {
      case "n":
        out += "\n";
        break;
      case "r":
        out += "\r";
        break;
      case "t":
        out += "\t";
        break;
      case "0":
        out += "\0";
        break;
      case "\\":
        out += "\\";
        break;
      case '"':
        out += '"';
        break;
      case "x":
        out += String.fromCharCode(Number.parseInt(body.slice(i + 1, i + 3), 16));
        i += 2;
        break;
      case "u": {
        const brace = body.indexOf("}", i);
        out += String.fromCodePoint(Number.parseInt(body.slice(i + 2, brace), 16));
        i = brace;
        break;
      }
      default:
        out += next;
    }
  }
  return out;
}

const CALL = /(?<![\w:])t!\s*\(/g;
const PATH = /^[A-Za-z_][A-Za-z0-9_]*(?:(?:\.|::)[A-Za-z_][A-Za-z0-9_]*)*/;

/**
 * The msgid of the `t!` call whose argument list starts at `start`, or
 * `undefined` when the first argument is not a plain string literal.
 */
function readMessage(source, start) {
  let cursor = start;
  const skipSpace = () => {
    while (cursor < source.length && /\s/.test(source[cursor])) cursor += 1;
  };

  skipSpace();
  let literalStart = cursor;

  if (source[cursor] !== '"') {
    const path = PATH.exec(source.slice(cursor));
    if (!path) return undefined;
    cursor += path[0].length;
    skipSpace();
    if (source[cursor] !== ",") return undefined;
    cursor += 1;
    skipSpace();
    literalStart = cursor;
  }

  if (source[literalStart] !== '"') return undefined;

  let end = literalStart + 1;
  while (end < source.length) {
    if (source[end] === "\\") end += 2;
    else if (source[end] === '"') break;
    else end += 1;
  }
  if (source[end] !== '"') return undefined;

  return unescape(source.slice(literalStart + 1, end));
}

function rustFiles(dir) {
  const files = [];
  for (const entry of readdirSync(dir)) {
    if (SKIP_DIRS.has(entry)) continue;
    const path = join(dir, entry);
    const stat = statSync(path);
    if (stat.isDirectory()) files.push(...rustFiles(path));
    else if (entry.endsWith(".rs")) files.push(path);
  }
  return files;
}

const references = new Map();

for (const root of roots) {
  let files;
  try {
    files = rustFiles(root);
  } catch {
    continue;
  }

  for (const file of files) {
    const source = readFileSync(file, "utf8");
    const code = mask(source);
    const shown = relative(gpuiDir, file).split(sep).join("/");

    for (const match of code.matchAll(CALL)) {
      const msgid = readMessage(source, match.index + match[0].length);
      if (msgid === undefined) continue;

      const line = code.slice(0, match.index).split("\n").length;
      let refs = references.get(msgid);
      if (!refs) {
        refs = new Set();
        references.set(msgid, refs);
      }
      refs.add(`gpui/${shown}:${line}`);
    }
  }
}

/** A PO quoted string for `text`. */
function quote(text) {
  return `"${text
    .replaceAll("\\", "\\\\")
    .replaceAll('"', '\\"')
    .replaceAll("\n", "\\n")
    .replaceAll("\r", "\\r")
    .replaceAll("\t", "\\t")}"`;
}

const msgids = [...references.keys()].sort((left, right) =>
  Buffer.compare(Buffer.from(left), Buffer.from(right)),
);

const lines = [
  'msgid ""',
  'msgstr ""',
  '"Project-Id-Version: dragonfruit-gpui\\n"',
  '"Report-Msgid-Bugs-To: \\n"',
  '"MIME-Version: 1.0\\n"',
  '"Content-Type: text/plain; charset=utf-8\\n"',
  '"Content-Transfer-Encoding: 8bit\\n"',
  '"X-Generator: extract-gpui-strings.mjs\\n"',
  "",
];

for (const msgid of msgids) {
  for (const ref of [...references.get(msgid)].sort()) lines.push(`#: ${ref}`);
  lines.push(`msgid ${quote(msgid)}`, 'msgstr ""', "");
}

process.stdout.write(`${lines.join("\n")}`);

if (process.argv.includes("--stats")) {
  process.stderr.write(`${msgids.length} messages from gpui/src and gpui/crates\n`);
}
