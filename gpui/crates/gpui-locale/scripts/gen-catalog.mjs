#!/usr/bin/env node
// Generates `src/catalog_data.rs` from Lingui's PO catalogues.
//
// The GPUI port cannot read PO files at runtime (no file IO on the hot path),
// so the translated messages are compiled in as sorted `&[(msgid, msgstr)]`
// tables. This script is the only writer of that file: run it from anywhere,
// it finds the catalogues and its own output relative to its location.
//
//   node gpui/crates/gpui-locale/scripts/gen-catalog.mjs
//
// Plain Node, no dependencies beyond `fs`/`path`, matching the rule for the
// string extractor next door.

import { readFileSync, writeFileSync, existsSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const scriptDir = dirname(fileURLToPath(import.meta.url));
const catalogueDir = resolve(scriptDir, "../../../../src/locales");
const outputPath = resolve(scriptDir, "../src/catalog_data.rs");

// `lingui.config.ts` lists the shipped locales and names `en` as the source
// locale. English therefore needs no table: every msgid already is the English
// text, which is what the fallback in `Catalog::text` returns.
const LOCALES = ["es", "de", "fr", "sk", "cs"];

const ESCAPES = {
  n: "\n",
  t: "\t",
  r: "\r",
  a: "\x07",
  b: "\b",
  f: "\f",
  v: "\v",
  '"': '"',
  "\\": "\\",
};

/** The body of a PO quoted string, with PO escapes resolved. */
function unquote(body) {
  let out = "";
  for (let i = 0; i < body.length; i += 1) {
    const char = body[i];
    if (char !== "\\") {
      out += char;
      continue;
    }
    const next = body[i + 1];
    out += ESCAPES[next] ?? next;
    i += 1;
  }
  return out;
}

/**
 * Parses one PO file into `{ context, msgid, msgstr }` triples.
 *
 * Lingui writes one message per block, one `msgid`/`msgstr` line per field and
 * continuation strings directly beneath, so a line state machine is enough.
 */
function parsePo(path) {
  const entries = [];
  let context = null;
  let msgid = null;
  let msgstr = null;
  let field = null;

  const flush = () => {
    if (msgid !== null) entries.push({ context, msgid, msgstr: msgstr ?? "" });
    context = null;
    msgid = null;
    msgstr = null;
    field = null;
  };

  for (const raw of readFileSync(path, "utf8").split("\n")) {
    const line = raw.replace(/\r$/, "");
    if (line.trim() === "") {
      flush();
      continue;
    }
    if (line.startsWith("#")) continue;

    const head = /^(msgctxt|msgid|msgstr) "(.*)"$/.exec(line);
    if (head) {
      const [, kind, body] = head;
      const value = unquote(body);
      if (kind === "msgctxt") {
        context = value;
        field = "context";
      } else if (kind === "msgid") {
        msgid = value;
        field = "msgid";
      } else {
        msgstr = value;
        field = "msgstr";
      }
      continue;
    }

    const continuation = /^"(.*)"$/.exec(line);
    if (continuation) {
      const value = unquote(continuation[1]);
      if (field === "context") context += value;
      else if (field === "msgid") msgid += value;
      else if (field === "msgstr") msgstr += value;
      continue;
    }

    throw new Error(`${path}: unparsed PO line: ${line}`);
  }

  flush();
  return entries.filter((entry) => entry.msgid !== "");
}

/** A Rust string literal for `text`. */
function rustString(text) {
  let out = '"';
  for (const char of text) {
    const code = char.codePointAt(0);
    if (char === "\\") out += "\\\\";
    else if (char === '"') out += '\\"';
    else if (char === "\n") out += "\\n";
    else if (char === "\r") out += "\\r";
    else if (char === "\t") out += "\\t";
    else if (code < 0x20 || code === 0x7f) out += `\\u{${code.toString(16)}}`;
    else out += char;
  }
  return `${out}"`;
}

/**
 * The translated messages of one locale.
 *
 * Untranslated entries are dropped: the msgid is the English text, so the
 * fallback already covers them. Context-carrying entries are dropped too,
 * because `t!` carries only the literal and could not tell `Smooth` the brush
 * from `Smooth` the anti-aliasing preset.
 */
function translations(locale) {
  const path = resolve(catalogueDir, `${locale}.po`);
  if (!existsSync(path)) throw new Error(`missing catalogue: ${path}`);

  const kept = parsePo(path)
    .filter((entry) => entry.context === null && entry.msgstr !== "")
    .map((entry) => [entry.msgid, entry.msgstr]);

  const seen = new Set();
  for (const [msgid] of kept) {
    if (seen.has(msgid)) throw new Error(`${locale}: duplicate msgid ${JSON.stringify(msgid)}`);
    seen.add(msgid);
  }

  // `Catalog` binary-searches, so tables are stored in msgid byte order.
  kept.sort((left, right) => Buffer.compare(Buffer.from(left[0]), Buffer.from(right[0])));
  return kept;
}

const header = `//! The catalogues this crate ships, generated from Lingui's PO output.
//!
//! Source: \`src/locales/{${LOCALES.join(",")}}.po\` at the repository root.
//! \`lingui.config.ts\` lists \`en, es, de, fr, sk, cs\` and names English as the
//! source locale, so there is no English table: every msgid already is the
//! English text and \`Catalog\` falls back to it.
//!
//! Only entries with a non-empty \`msgstr\` are included, and entries carrying a
//! \`msgctxt\` are skipped: \`t!\` passes only the English literal, so a message
//! whose context changes its meaning cannot be resolved yet. Contexts and
//! interpolation are a later slice.
//!
//! Tables are sorted by \`msgid\` byte order, which is what \`Catalog\` binary
//! searches. Regenerate with
//! \`node gpui/crates/gpui-locale/scripts/gen-catalog.mjs\` after
//! \`lingui extract\` refreshes the catalogues.

`;

const parts = [header];
for (const locale of LOCALES) {
  const entries = translations(locale);
  const name = locale.toUpperCase();
  parts.push(`/// \`src/locales/${locale}.po\`: ${entries.length} translated messages.\n`);
  parts.push(`pub(crate) static ${name}: &[(&str, &str)] = &[\n`);
  for (const [msgid, msgstr] of entries) {
    parts.push(`    (${rustString(msgid)}, ${rustString(msgstr)}),\n`);
  }
  parts.push("];\n\n");
}

parts.push(`/// The table for a language subtag.
///
/// Unsupported languages get the source language's empty table, so their
/// lookups return the msgid, which is the English text.
pub(crate) fn table_for(language: &str) -> &'static [(&'static str, &'static str)] {
    match language {
        "es" => ES,
        "de" => DE,
        "fr" => FR,
        "sk" => SK,
        "cs" => CS,
        _ => &[],
    }
}

/// Every language with a shipped catalogue; see \`lingui.config.ts\`.
pub const SUPPORTED_LANGUAGES: &[&str] = &["en", "es", "de", "fr", "sk", "cs"];
`);

writeFileSync(outputPath, parts.join(""), "utf8");

const counts = LOCALES.map((locale) => `${locale}=${translations(locale).length}`);
process.stderr.write(`wrote ${outputPath} (${counts.join(", ")})\n`);
