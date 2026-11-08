#!/usr/bin/env node
// Extracts the user-facing surface of the TypeScript app into a map the GPUI
// port can be checked against. One row per thing a user can see or press:
//
//   node gpui/scripts/extract-surface.mjs                  # writes surface.json
//   node gpui/scripts/extract-surface.mjs --seed-ledger    # also seeds gpui-coverage.json
//
// The parse is AST-based (the TypeScript compiler API, resolved from the repo's
// node_modules) so the result does not depend on formatting. There is no type
// checker: a JSX attribute is read by name, and labels come from the render
// tree (JSX text, `msg`/`_` templates, `<Trans>` children), then from the
// accessible name, then from the handler.
//
// Scope: every `.tsx` under `src/components/**` and `src/features/**` except
// `__tests__`, `*.test.tsx`, `*.dom-test.tsx` and `*.stories.tsx`, plus
// `src/app/page.tsx`. Test files are reported in `skipped`, not parsed.
//
// `src/features` is walked because the component tree only *mounts* feature
// panels: `PreparePanelStack` renders `<OrganicCutPanel />` and
// `<MeshSmoothingSettingsPanel />`, `SettingsModal` renders `<UpdatesSettingsTab />`,
// and the panel bodies (with every control inside) live under `src/features`.
// Walking components alone captured the mounts and dropped the controls.
//
// Output shape (deterministic, no timestamps):
//   { generatedBy, roots, skipped, collisions, counts, rows: [
//       { id, source, kind, label, action, gates }
//   ] }

import { existsSync, mkdirSync, readFileSync, readdirSync, statSync, writeFileSync } from 'node:fs';
import { execFileSync } from 'node:child_process';
import { createRequire } from 'node:module';
import { basename, dirname, isAbsolute, join, relative, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const scriptDir = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(scriptDir, '..', '..');
const componentsRoot = resolve(repoRoot, 'src', 'components');
const featuresRoot = resolve(repoRoot, 'src', 'features');
const appRoot = resolve(repoRoot, 'src', 'app');
const pagePath = resolve(appRoot, 'page.tsx');

export const surfacePath = resolve(scriptDir, 'surface.json');
export const ledgerPath = resolve(repoRoot, 'docs', 'internal', 'gpui-coverage.json');

export const sourceRoots = [componentsRoot, featuresRoot, pagePath];
const SOURCE_ROOT_LABELS = ['src/components/**/*.tsx', 'src/features/**/*.tsx', 'src/app/page.tsx'];
const SKIP_DIRS = new Set(['__tests__', 'node_modules', 'dist', '.next', '.git']);
const SKIP_FILE = /\.(dom-)?(test|stories)\.tsx$|\.d\.tsx?$/;

const HANDLERS = ['onSubmit', 'onClick', 'onChange', 'onSelect'];
const NAMED_INTERACTIVE = /(Button|Input|Select|Toggle|Switch|Checkbox|Radio|Link|Tab|Slider|Field)$/;
const INTERACTIVE_TAGS = new Set(['button', 'a', 'input', 'select', 'textarea', 'summary', 'option']);
const TEXT_ELEMENT_TAGS = new Set([
  'Trans', 'span', 'strong', 'em', 'b', 'i', 'p', 'label', 'small', 'code', 'abbr',
  'h1', 'h2', 'h3', 'h4', 'h5', 'h6',
]);
const NON_CONTAINER_COMPONENTS = /^(Trans|Icon|[A-Za-z0-9]+Icon|Spinner|Loader[A-Za-z0-9]*|Fragment)$/;
const CONTAINER_KINDS = new Set(['modal', 'panel', 'popover', 'tab', 'notification']);
const INTRINSIC_CONTAINERS = new Set(['div', 'section', 'aside', 'article', 'ul', 'li', 'header', 'footer', 'nav', 'main', 'dialog']);

let tsModule = null;

/** Resolve `typescript` from the repo, allowing a git worktree without its own node_modules. */
export function loadTypeScript() {
  if (tsModule) return tsModule;
  const require = createRequire(import.meta.url);
  const roots = new Set();

  const pushAncestors = (start) => {
    let cur = start;
    for (;;) {
      roots.add(cur);
      const parent = dirname(cur);
      if (parent === cur) break;
      cur = parent;
    }
  };
  pushAncestors(repoRoot);
  pushAncestors(process.cwd());
  if (process.env.TYPESCRIPT_ROOT) roots.add(resolve(process.env.TYPESCRIPT_ROOT));

  try {
    const commonDir = execFileSync('git', ['rev-parse', '--git-common-dir'], {
      cwd: repoRoot,
      encoding: 'utf8',
      stdio: ['ignore', 'pipe', 'ignore'],
    }).trim();
    roots.add(dirname(resolve(repoRoot, commonDir)));
  } catch {
    // Not a git checkout with common-dir support; the ancestors above may work.
  }

  for (const root of roots) {
    try {
      tsModule = require(require.resolve('typescript', { paths: [root] }));
      return tsModule;
    } catch {
      // Try the next candidate root.
    }
  }
  throw new Error(
    `Cannot resolve the "typescript" package. Looked from: ${[...roots].join(', ')}. ` +
      'Run `npm install` at the repo root or set TYPESCRIPT_ROOT to a directory whose node_modules contains typescript.',
  );
}

const toPosix = (p) => p.split('\\').join('/');

const isUnder = (path, root) => {
  const rel = relative(root, path);
  return rel !== '' && !rel.startsWith('..') && !isAbsolute(rel);
};

/**
 * True when `path` is a `.tsx` file the extractor parses. `path` may be
 * absolute or repo-relative. Both `src/components` and `src/features` are in
 * scope; `src/app` is not, except for `page.tsx`.
 */
export function isSurfaceSourcePath(path) {
  const abs = isAbsolute(path) ? resolve(path) : resolve(repoRoot, path);
  if (!abs.endsWith('.tsx') || SKIP_FILE.test(abs)) return false;
  if (abs === pagePath) return true;
  if (!isUnder(abs, componentsRoot) && !isUnder(abs, featuresRoot)) return false;
  return !toPosix(relative(repoRoot, abs)).split('/').some((segment) => SKIP_DIRS.has(segment));
}

const compareStrings = (a, b) => (a < b ? -1 : a > b ? 1 : 0);
const areaOfId = (id) => id.slice(0, id.indexOf('/'));

function tidy(text, limit = 120) {
  if (!text) return '';
  const flat = String(text).replace(/\s+/g, ' ').trim();
  return flat.length > limit ? `${flat.slice(0, limit - 1)}\u2026` : flat;
}

export function slugify(text) {
  const slug = String(text ?? '')
    .normalize('NFKD')
    .replace(/[\u0300-\u036f]/g, '')
    .replace(/&/g, ' and ')
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-+|-+$/g, '')
    .slice(0, 48)
    .replace(/-+$/g, '');
  return slug || 'surface';
}

/** Pull every string literal out of an expression (class names hide in `cn(...)`). */
function collectStrings(node, ts, out = []) {
  if (!node) return out;
  if (ts.isStringLiteral(node) || ts.isNoSubstitutionTemplateLiteral(node)) {
    out.push(node.text);
    return out;
  }
  if (ts.isTemplateExpression(node)) {
    out.push(node.head.text);
    for (const span of node.templateSpans) collectStrings(span.literal, ts, out);
    return out;
  }
  if (ts.isJsxExpression(node)) {
    collectStrings(node.expression, ts, out);
    return out;
  }
  ts.forEachChild(node, (child) => collectStrings(child, ts, out));
  return out;
}

function classNameGates(initializer, ts) {
  const gates = new Set();
  for (const text of collectStrings(initializer, ts)) {
    for (const token of text.split(/\s+/)) {
      if (!token) continue;
      if (token === 'hidden' || /(^|:)hidden$/.test(token)) gates.add('class:hidden');
      if (token === 'pointer-events-none' || /(^|:)pointer-events-none$/.test(token)) {
        gates.add('class:pointer-events-none');
      }
    }
  }
  return [...gates];
}

/** Resolve a node that renders user-visible text. Returns null when it does not. */
function resolveText(node, ts, depth = 0) {
  if (!node || depth > 8) return null;
  if (ts.isStringLiteral(node) || ts.isNoSubstitutionTemplateLiteral(node)) return tidy(node.text, 160) || null;
  if (ts.isTemplateExpression(node)) return tidy(node.head.text, 160) || null;
  if (ts.isTaggedTemplateExpression(node)) return resolveText(node.template, ts, depth + 1);
  if (ts.isParenthesizedExpression(node) || ts.isAsExpression(node) || ts.isNonNullExpression(node) || ts.isAwaitExpression(node)) {
    return resolveText(node.expression, ts, depth + 1);
  }
  if (ts.isJsxExpression(node)) return resolveText(node.expression, ts, depth + 1);
  if (ts.isConditionalExpression(node)) {
    const a = resolveText(node.whenTrue, ts, depth + 1);
    const b = resolveText(node.whenFalse, ts, depth + 1);
    if (a && b && a !== b) return tidy(`${a} / ${b}`, 160);
    return a || b;
  }
  if (ts.isCallExpression(node)) {
    const callee = node.expression.getText();
    const isTranslator = callee === '_' || callee === 'msg' || callee === 't' || callee.endsWith('._');
    if (isTranslator) {
      const arg = node.arguments[0];
      if (arg && ts.isObjectLiteralExpression(arg)) {
        for (const prop of arg.properties) {
          if (ts.isPropertyAssignment(prop) && prop.name.getText() === 'message') {
            return resolveText(prop.initializer, ts, depth + 1);
          }
        }
      }
      return resolveText(arg, ts, depth + 1);
    }
    return null;
  }
  if (ts.isArrayLiteralExpression(node)) {
    for (const element of node.elements) {
      const text = resolveText(element, ts, depth + 1);
      if (text) return text;
    }
    return null;
  }
  if (ts.isJsxElement(node)) {
    if (node.openingElement.tagName.getText() === 'Trans' || isIntrinsicTextTag(node.openingElement.tagName.getText())) {
      return childrenText(node.children, ts, depth + 1);
    }
    return childrenText(node.children, ts, depth + 1);
  }
  if (ts.isJsxFragment(node)) return childrenText(node.children, ts, depth + 1);
  if (ts.isJsxSelfClosingElement(node)) return null;
  return null;
}

function isIntrinsicTextTag(name) {
  return /^(span|strong|em|b|i|p|label|h[1-6]|small|code|abbr)$/.test(name);
}

function childrenText(children, ts, depth = 0) {
  const parts = [];
  for (const child of children) {
    if (ts.isJsxText(child)) {
      const text = tidy(child.text, 160);
      if (text) parts.push(text);
      continue;
    }
    const text = resolveText(child, ts, depth + 1);
    if (text) parts.push(text);
    if (parts.length >= 4) break;
  }
  if (!parts.length) return null;
  return tidy(parts.join(' '), 160);
}

/** The first literal condition text, flattened for use as a gate. */
function conditionText(expr, ts) {
  return tidy(expr.getText(), 120).replace(/^\((.*)\)$/, '$1');
}

function negate(cond) {
  const text = cond.trim();
  if (text.startsWith('!') && !/[\s&|]/.test(text.slice(1))) return text.slice(1);
  return `!(${text})`;
}

/** Handler description: `onClick:handleRun` rather than the whole arrow body. */
function handlerName(expr, ts, depth = 0) {
  if (!expr || depth > 6) return '';
  if (ts.isArrowFunction(expr) || ts.isFunctionExpression(expr)) {
    if (!ts.isBlock(expr.body)) {
      const name = handlerName(expr.body, ts, depth + 1);
      if (name) return name;
    } else {
      let name = '';
      const scan = (n) => {
        if (name) return;
        if (ts.isCallExpression(n)) {
          const callee = n.expression;
          if (ts.isIdentifier(callee) || ts.isPropertyAccessExpression(callee)) {
            name = callee.getText();
            return;
          }
        }
        ts.forEachChild(n, scan);
      };
      scan(expr.body);
      if (name) return tidy(name, 70);
    }
    return '';
  }
  if (ts.isIdentifier(expr)) return tidy(expr.getText(), 70);
  if (ts.isCallExpression(expr)) {
    const callee = expr.expression;
    if (ts.isIdentifier(callee) || ts.isPropertyAccessExpression(callee)) return tidy(callee.getText(), 70);
  }
  if (ts.isConditionalExpression(expr)) {
    return handlerName(expr.whenTrue, ts, depth + 1) || handlerName(expr.whenFalse, ts, depth + 1);
  }
  if (ts.isParenthesizedExpression(expr) || ts.isAsExpression(expr) || ts.isNonNullExpression(expr)) {
    return handlerName(expr.expression, ts, depth + 1);
  }
  return '';
}

/** Classify one JSX element by name/role. */
function classifyElement(name, attrs, ts) {
  const role = attrs.get('role')?.text;
  if (role === 'dialog' || role === 'alertdialog') return 'modal';
  if (role === 'menu' || role === 'listbox') return 'popover';
  if (role === 'tab') return 'tab';
  if (/(Modal|Dialog)$/.test(name) || name === 'dialog') return 'modal';
  if (/(Select|Combobox)$/.test(name) || name === 'select') return 'select';
  if (/(Popover|Dropdown|Flyout)$/.test(name) || /Menu$/.test(name)) return 'popover';
  if (/(Panel|Sidebar|Drawer|Sheet|Pane)$/.test(name)) return 'panel';
  if (/Tab$/.test(name)) return 'tab';
  if (/(Toast|Notification)$/.test(name)) return 'notification';
  if (name === 'form') return 'form';
  if (name === 'button' || /Button$/.test(name)) return 'button';
  if (name === 'a' || attrs.has('href')) return 'link';
  if (name === 'input' || name === 'textarea' || /(Input|TextField|SearchField)$/.test(name)) return 'input';
  if (/(Toggle|Switch|Checkbox|Radio|Slider)$/.test(name)) return 'toggle';
  if (INTRINSIC_CONTAINERS.has(name)) return 'container';
  return 'component';
}

/**
 * Extract the surface rows from one source string. `filePath` is used for the
 * source reference and the area; it may be a virtual path for tests.
 */
export function extractFromSource(source, filePath, state = null) {
  const ts = loadTypeScript();
  const relPath = toPosix(isAbsolute(filePath) ? relative(repoRoot, filePath) : filePath);
  const area = areaFor(relPath);
  const sf = ts.createSourceFile(relPath, source, ts.ScriptTarget.Latest, true, ts.ScriptKind.TSX);
  const rows = [];
  const usedIds = state?.usedIds ?? new Map();
  const signatures = state?.signatures ?? new Set();
  const collisions = state?.collisions ?? [];

  const lineOf = (node) => sf.getLineAndCharacterOfPosition(node.getStart(sf)).line + 1;
  const sourceOf = (node) => `${relPath}:${lineOf(node)}`;

  function areaFor(rel) {
    if (rel.startsWith('src/app/')) return 'app';
    const abs = resolve(repoRoot, rel);
    // A feature's rows are named for the feature (`organicCut/…`), not for the
    // leaf directory: `features/export/components/ExportPanel.tsx` and
    // `features/slicing/components/SlicingPanel.tsx` share no area.
    if (isUnder(abs, featuresRoot)) {
      const segments = toPosix(relative(featuresRoot, abs)).split('/');
      return segments.length > 1 ? segments[0] : 'features';
    }
    const relToComponents = toPosix(relative(componentsRoot, abs));
    const dir = dirname(relToComponents);
    return dir === '.' || dir === 'src' ? 'components' : toPosix(basename(dir));
  }

  function readAttrs(opening) {
    const attrs = new Map();
    for (const prop of opening.attributes.properties) {
      if (!ts.isJsxAttribute(prop)) continue;
      const name = prop.name.getText(sf);
      const init = prop.initializer;
      let text = null;
      let exprText = '';
      if (init) {
        if (ts.isStringLiteral(init)) {
          text = init.text;
          exprText = init.text;
        } else if (ts.isJsxExpression(init) && init.expression) {
          text = resolveText(init.expression, ts);
          exprText = tidy(init.expression.getText(sf), 70);
        }
      }
      attrs.set(name, { name, node: prop, init, text: text ?? '', exprText });
    }
    return attrs;
  }

  function visibleLabel(children) {
    // Only direct, text-bearing children name a container. Joining every
    // descendant produces "Model volume Est. weight Candidates …", which is a
    // description, not a label.
    for (const child of children) {
      if (ts.isJsxText(child)) {
        const text = tidy(child.text, 160);
        if (text) return text;
        continue;
      }
      if (ts.isJsxExpression(child)) {
        const text = resolveText(child, ts);
        if (text) return text;
        continue;
      }
      if (ts.isJsxElement(child)) {
        const tag = child.openingElement.tagName.getText(sf).split('.').pop();
        if (TEXT_ELEMENT_TAGS.has(tag)) {
          const text = resolveText(child, ts);
          if (text) return text;
        }
      }
    }
    return null;
  }

  function pickLabel(attrs, children, handler, preferName) {
    const nameKeys = ['aria-label', 'ariaLabel', 'title', 'label', 'header', 'heading', 'placeholder'];
    // A modal's body paragraph is not its name; the accessible name or the
    // `title` prop is. Interactive elements still lead with their visible text.
    if (preferName) {
      for (const key of nameKeys) {
        const text = attrs.get(key)?.text;
        if (text) return tidy(text, 160);
      }
      const visible = visibleLabel(children);
      if (visible) return visible;
    } else {
      const visible = visibleLabel(children);
      if (visible) return visible;
      for (const key of nameKeys) {
        const text = attrs.get(key)?.text;
        if (text) return tidy(text, 160);
      }
    }
    if (handler) return tidy(handler, 120);
    return null;
  }

  function collectGates(attrs, ctx, earlyReturn) {
    const gates = [];
    if (ctx.when) gates.push(`when:${tidy(ctx.when, 100)}`);
    if (earlyReturn) gates.push(`early-return:${tidy(earlyReturn, 100)}`);
    const disabled = attrs.get('disabled');
    if (disabled) gates.push(`disabled:${disabled.exprText || disabled.text || 'true'}`);
    const hidden = attrs.get('hidden');
    if (hidden) gates.push(`hidden:${hidden.exprText || hidden.text || 'true'}`);
    const className = attrs.get('className');
    if (className?.init) {
      for (const gate of classNameGates(className.init, ts)) gates.push(gate);
    }
    return [...new Set(gates)];
  }

  function actionFor(attrs, ctx, kind, handler) {
    for (const prop of HANDLERS) {
      const attr = attrs.get(prop);
      if (!attr) continue;
      const name = attr.init
        ? handlerName(ts.isJsxExpression(attr.init) ? attr.init.expression : attr.init, ts)
        : '';
      return `${prop}:${name || attr.text || kind}`;
    }
    if (attrs.has('href')) return `href:${attrs.get('href').text || 'link'}`;
    for (const prop of ['onClose', 'onConfirm', 'onCancel', 'onToggle', 'onOpenChange']) {
      const attr = attrs.get(prop);
      if (!attr) continue;
      const name = attr.init
        ? handlerName(ts.isJsxExpression(attr.init) ? attr.init.expression : attr.init, ts)
        : '';
      return `${prop}:${name || prop}`;
    }
    for (const prop of ['open', 'isOpen', 'expanded']) {
      const attr = attrs.get(prop);
      if (!attr) continue;
      const expr = attr.init && ts.isJsxExpression(attr.init) ? attr.init.expression : attr.init;
      const name = expr ? handlerName(expr, ts) || tidy(expr.getText(sf), 60) : attr.text;
      return `${prop}:${name || 'true'}`;
    }
    if (ctx.when) return `conditional:${tidy(ctx.when, 70)}`;
    return `renders:${kind}`;
  }

  function assignId(base, component, signature, row) {
    if (signatures.has(signature)) return null;
    let id = base;
    if (usedIds.has(base)) {
      const suffix = slugify(component || basename(relPath, '.tsx'));
      let candidate = `${base}--${suffix}`;
      let n = 2;
      while (usedIds.has(candidate)) candidate = `${base}--${suffix}-${n++}`;
      collisions.push({
        id: base,
        resolved: candidate,
        first: usedIds.get(base).source,
        second: row.source,
        label: row.label,
        kind: row.kind,
      });
      id = candidate;
    }
    usedIds.set(id, row);
    signatures.add(signature);
    return id;
  }

  function emitElement(node, opening, ctx, emptyStateCond = null) {
    const tagName = opening.tagName.getText(sf);
    const elementName = tagName.split('.').pop();
    const attrs = readAttrs(opening);
    const kind = emptyStateCond ? 'empty-state' : classifyElement(elementName, attrs, ts);
    const hasHandler = HANDLERS.some((h) => attrs.has(h)) || attrs.has('href');
    const named = ['aria-label', 'ariaLabel', 'title', 'label'].some((key) => attrs.get(key)?.text);
    const interactive = hasHandler || (named && (INTERACTIVE_TAGS.has(elementName) || NAMED_INTERACTIVE.test(elementName)));
    const conditional = Boolean(ctx.when || ctx.earlyReturn);
    const intrinsicTag = /^[a-z]/.test(elementName);
    // A conditional `<p>` is copy, not a surface. Only intrinsic container
    // tags and non-leaf components become containers.
    const container =
      CONTAINER_KINDS.has(kind) ||
      (conditional && !intrinsicTag && !NON_CONTAINER_COMPONENTS.test(elementName)) ||
      (conditional && intrinsicTag && INTRINSIC_CONTAINERS.has(elementName)) ||
      (conditional && kind === 'form');

    if (!emptyStateCond && !interactive && !container) return;

    const children = node.children ?? [];
    const handlerSource = HANDLERS.map((h) => attrs.get(h)).find(Boolean);
    const handler = handlerSource?.init
      ? handlerName(ts.isJsxExpression(handlerSource.init) ? handlerSource.init.expression : handlerSource.init, ts)
      : '';
    const label = pickLabel(attrs, children, handler, CONTAINER_KINDS.has(kind) || (container && !interactive));

    // A conditional wrapper with no text, accessible name or handler is layout,
    // not a surface. Named container kinds (modal/panel/popover/tab) still emit
    // so an untitled shell cannot disappear from the map.
    if (!emptyStateCond && !interactive && !label && !CONTAINER_KINDS.has(kind)) return;

    const finalLabel = label || (!intrinsicTag ? elementName : null) || ctx.component || elementName;
    const action = emptyStateCond
      ? `early-return:${tidy(emptyStateCond, 70)}`
      : actionFor(attrs, ctx, kind, handler);
    const gates = collectGates(attrs, ctx, emptyStateCond);

    const row = {
      _area: area,
      _file: relPath,
      _line: lineOf(opening),
      id: '',
      source: sourceOf(opening),
      kind,
      label: finalLabel,
      action,
      gates,
      _component: ctx.component || '',
    };
    const base = `${area}/${slugify(finalLabel)}`;
    const signature = [kind, row.source, finalLabel, action, gates.join('|')].join('\u0000');
    const id = assignId(base, ctx.component, signature, row);
    if (!id) return;
    row.id = id;
    rows.push(row);
  }

  function emitNullEarlyReturn(ret, ctx, condition) {
    const row = {
      _area: area,
      _file: relPath,
      _line: lineOf(ret),
      id: '',
      source: sourceOf(ret),
      kind: 'empty-state',
      label: '(no render)',
      action: `early-return:${tidy(condition, 70)}`,
      gates: collectGates(new Map(), ctx, condition),
      _component: ctx.component || '',
    };
    const base = `${area}/no-render`;
    const signature = ['empty-state', row.source, row.label, row.action, row.gates.join('|')].join('\u0000');
    const id = assignId(base, ctx.component, signature, row);
    if (!id) return;
    row.id = id;
    rows.push(row);
  }

  function walkJsxChildren(node, ctx) {
    const children = node.children ?? [];
    for (const child of children) walk(child, ctx);
  }

  function walkJsxAttributes(opening, ctx) {
    for (const prop of opening.attributes.properties) {
      if (!ts.isJsxAttribute(prop) || !prop.initializer) continue;
      walk(prop.initializer, ctx);
    }
  }

  function elementHasSurfaceLabel(node) {
    const opening = ts.isJsxElement(node)
      ? node.openingElement
      : ts.isJsxSelfClosingElement(node)
        ? node
        : null;
    if (!opening) return false;
    const attrs = readAttrs(opening);
    for (const key of ['aria-label', 'ariaLabel', 'title', 'label']) {
      if (attrs.get(key)?.text) return true;
    }
    const name = opening.tagName.getText(sf).split('.').pop();
    if (/(Empty|Placeholder|Notice|Message|Banner)/.test(name)) return true;
    return Boolean(visibleLabel(node.children ?? []));
  }

  function handleReturn(ret, ctx) {
    const expr = ret.expression;
    if (!expr) return;
    const isJsx = ts.isJsxElement(expr) || ts.isJsxSelfClosingElement(expr) || ts.isJsxFragment(expr);
    const isNullish =
      expr.kind === ts.SyntaxKind.NullKeyword ||
      (ts.isIdentifier(expr) && expr.text === 'undefined') ||
      ((ts.isAsExpression(expr) || ts.isParenthesizedExpression(expr)) &&
        expr.expression.kind === ts.SyntaxKind.NullKeyword);

    if (ctx.earlyReturn && (isJsx || isNullish) && ctx.fn && functionReturnsJsx(ctx.fn)) {
      if (isJsx) {
        // `if (!locked) return <div>{field}</div>` is a branch, not an empty
        // state. Only a labelled branch (or an explicitly empty one) is a state
        // worth its own ledger row; the unlabelled wrapper stays layout.
        if (!elementHasSurfaceLabel(expr)) {
          walk(expr, { ...ctx, earlyReturn: null });
          return;
        }
        const opening = ts.isJsxElement(expr) ? expr.openingElement : ts.isJsxSelfClosingElement(expr) ? expr : null;
        if (opening) {
          emitElement(expr, opening, ctx, ctx.earlyReturn);
          walkJsxAttributes(opening, ctx);
          if (ts.isJsxElement(expr) || ts.isJsxFragment(expr)) walkJsxChildren(expr, ctx);
        } else {
          walkJsxChildren(expr, ctx);
        }
        return;
      }
      emitNullEarlyReturn(ret, ctx, ctx.earlyReturn);
      return;
    }

    walk(expr, ctx);
  }

  function handleBranch(stmt, ctx, condition) {
    const ret = singleReturn(stmt, ts);
    if (ret) {
      handleReturn(ret, { ...ctx, earlyReturn: condition });
      return;
    }
    walk(stmt, ctx);
  }

  function withWhen(ctx, condition) {
    if (!condition) return ctx;
    const when = ctx.when ? `${ctx.when} && ${condition}` : condition;
    return { ...ctx, when: tidy(when, 120) };
  }

  function functionName(node) {
    if ((ts.isFunctionDeclaration(node) || ts.isFunctionExpression(node)) && node.name) return node.name.getText(sf);
    if (ts.isMethodDeclaration(node) && node.name) return node.name.getText(sf);
    return null;
  }

  function visitFunction(node, ctx, name) {
    const inner = {
      when: null,
      earlyReturn: null,
      component: name || ctx.component || null,
      fn: node,
    };
    if (node.body) walk(node.body, inner);
  }

  function walk(node, ctx) {
    if (!node) return;

    if (ts.isFunctionDeclaration(node) || ts.isMethodDeclaration(node)) {
      visitFunction(node, ctx, functionName(node));
      return;
    }

    if (ts.isVariableDeclaration(node)) {
      const init = node.initializer;
      if (init && (ts.isArrowFunction(init) || ts.isFunctionExpression(init))) {
        const name = ts.isIdentifier(node.name) ? node.name.getText(sf) : null;
        visitFunction(init, ctx, name);
        return;
      }
    }

    if (ts.isArrowFunction(node) || ts.isFunctionExpression(node)) {
      visitFunction(node, ctx, functionName(node));
      return;
    }

    if (ts.isIfStatement(node)) {
      walk(node.expression, ctx);
      const condition = conditionText(node.expression, ts);
      handleBranch(node.thenStatement, withWhen(ctx, condition), condition);
      if (node.elseStatement) {
        const negated = negate(condition);
        handleBranch(node.elseStatement, withWhen(ctx, negated), negated);
      }
      return;
    }

    if (ts.isConditionalExpression(node)) {
      walk(node.condition, ctx);
      const condition = conditionText(node.condition, ts);
      walk(node.whenTrue, withWhen(ctx, condition));
      walk(node.whenFalse, withWhen(ctx, negate(condition)));
      return;
    }

    if (ts.isBinaryExpression(node)) {
      const operator = node.operatorToken.getText(sf);
      if (operator === '&&' || operator === '||') {
        walk(node.left, ctx);
        const condition = conditionText(node.left, ts);
        walk(node.right, withWhen(ctx, operator === '&&' ? condition : negate(condition)));
        return;
      }
    }

    if (ts.isJsxElement(node)) {
      const opening = node.openingElement;
      emitElement(node, opening, ctx);
      walkJsxAttributes(opening, ctx);
      walkJsxChildren(node, ctx);
      return;
    }

    if (ts.isJsxSelfClosingElement(node)) {
      emitElement(node, node, ctx);
      walkJsxAttributes(node, ctx);
      return;
    }

    if (ts.isJsxFragment(node)) {
      walkJsxChildren(node, ctx);
      return;
    }

    ts.forEachChild(node, (child) => walk(child, ctx));
  }

  walk(sf, { when: null, earlyReturn: null, component: null, fn: null });

  rows.sort(
    (a, b) =>
      compareStrings(a._area, b._area) ||
      compareStrings(a._file, b._file) ||
      a._line - b._line ||
      compareStrings(a.id, b.id),
  );

  const cleanRows = rows.map(({ id, source, kind, label, action, gates }) => ({ id, source, kind, label, action, gates }));
  return { rows: cleanRows, collisions, parseDiagnostics: sf.parseDiagnostics?.length ?? 0 };
}

function singleReturn(stmt, ts) {
  if (ts.isReturnStatement(stmt)) return stmt;
  if (ts.isBlock(stmt)) {
    const meaningful = stmt.statements.filter((s) => !ts.isEmptyStatement(s));
    if (meaningful.length === 1 && ts.isReturnStatement(meaningful[0])) return meaningful[0];
  }
  return null;
}

const jsxFunctionCache = new WeakMap();

function functionReturnsJsx(fn, ts = loadTypeScript()) {
  if (!fn) return false;
  if (jsxFunctionCache.has(fn)) return jsxFunctionCache.get(fn);
  let found = false;
  const scan = (node) => {
    if (found || !node) return;
    if (ts.isJsxElement(node) || ts.isJsxSelfClosingElement(node) || ts.isJsxFragment(node)) {
      found = true;
      return;
    }
    if (
      ts.isFunctionDeclaration(node) ||
      ts.isFunctionExpression(node) ||
      ts.isArrowFunction(node) ||
      ts.isMethodDeclaration(node)
    ) {
      return;
    }
    ts.forEachChild(node, scan);
  };
  if (fn.body) scan(fn.body);
  jsxFunctionCache.set(fn, found);
  return found;
}

function* walkFiles(path, skipped) {
  const stat = statSync(path);
  if (stat.isFile()) {
    if (!path.endsWith('.tsx')) return;
    if (!isSurfaceSourcePath(path)) {
      skipped.push(toPosix(relative(repoRoot, path)));
      return;
    }
    yield path;
    return;
  }
  for (const entry of readdirSync(path, { withFileTypes: true }).sort((a, b) => a.name.localeCompare(b.name))) {
    const child = join(path, entry.name);
    if (entry.isDirectory()) {
      if (SKIP_DIRS.has(entry.name)) {
        if (entry.name === '__tests__') skipped.push(`${toPosix(relative(repoRoot, child))}/ (directory)`);
        continue;
      }
      yield* walkFiles(child, skipped);
    } else {
      yield* walkFiles(child, skipped);
    }
  }
}

/** Parse every in-scope file. */
export function extractAll() {
  const ts = loadTypeScript();
  const skipped = [];
  const rows = [];
  const collisions = [];
  const diagnostics = [];
  const files = [];
  const state = { usedIds: new Map(), signatures: new Set(), collisions: [] };

  for (const root of sourceRoots) {
    for (const file of walkFiles(root, skipped)) files.push(file);
  }
  // `src/app` contains more than page.tsx; only page.tsx is in scope.
  for (const entry of readdirSync(appRoot, { withFileTypes: true }).sort((a, b) => a.name.localeCompare(b.name))) {
    if (entry.isFile() && entry.name.endsWith('.tsx') && resolve(appRoot, entry.name) !== pagePath) {
      skipped.push(toPosix(relative(repoRoot, resolve(appRoot, entry.name))));
    }
  }

  for (const file of files.sort()) {
    const source = readFileSync(file, 'utf8');
    const result = extractFromSource(source, file, state);
    rows.push(...result.rows);
    if (result.parseDiagnostics) diagnostics.push({ file: toPosix(relative(repoRoot, file)), count: result.parseDiagnostics });
  }
  collisions.push(...state.collisions);

  rows.sort((a, b) => compareStrings(areaOfId(a.id), areaOfId(b.id)) || compareStrings(a.id, b.id));

  return { rows, collisions, skipped: [...new Set(skipped)].sort(), diagnostics, ts };
}

export function countsFor(rows) {
  const byArea = {};
  const byKind = {};
  for (const row of rows) {
    const area = row.id.slice(0, row.id.indexOf('/'));
    byArea[area] = (byArea[area] ?? 0) + 1;
    byKind[row.kind] = (byKind[row.kind] ?? 0) + 1;
  }
  const sortObject = (obj) => Object.fromEntries(Object.entries(obj).sort(([a], [b]) => compareStrings(a, b)));
  return { total: rows.length, byArea: sortObject(byArea), byKind: sortObject(byKind) };
}

/**
 * Add new surface ids to the ledger without touching an existing row's status or
 * evidence. Existing rows keep their position — the file is hand-annotated by
 * the area audits, and `check-coverage` reads ledger order as age — while new
 * rows are appended in area/id order, so the diff of a re-run is additions only.
 */
export function seedLedger(rows, path = ledgerPath) {
  let ledger = { version: 1, note: 'Surface coverage for the GPUI port. Status: absent -> rendered -> wired -> verified.', rows: [] };
  if (existsSync(path)) {
    ledger = JSON.parse(readFileSync(path, 'utf8'));
    ledger.rows ??= [];
  }
  const existing = new Map(ledger.rows.map((row) => [row.id, row]));
  const addedRows = [];
  for (const row of rows) {
    if (existing.has(row.id)) continue;
    const entry = {
      id: row.id,
      source: row.source,
      kind: row.kind,
      label: row.label,
      action: row.action,
      gates: row.gates,
      rust: null,
      status: 'absent',
      pixels: null,
      behaviour: null,
    };
    existing.set(row.id, entry);
    addedRows.push(entry);
  }
  addedRows.sort((a, b) => compareStrings(areaOfId(a.id), areaOfId(b.id)) || compareStrings(a.id, b.id));
  ledger.rows = [...ledger.rows, ...addedRows];
  mkdirSync(dirname(path), { recursive: true });
  writeFileSync(path, `${JSON.stringify(ledger, null, 2)}\n`);
  return { added: addedRows.length, total: ledger.rows.length, path };
}

function main() {
  const { rows, collisions, skipped, diagnostics } = extractAll();
  const counts = countsFor(rows);
  const output = {
    generatedBy: 'gpui/scripts/extract-surface.mjs',
    roots: SOURCE_ROOT_LABELS,
    skipped,
    collisions,
    diagnostics,
    counts,
    rows,
  };
  writeFileSync(surfacePath, `${JSON.stringify(output, null, 2)}\n`);

  process.stdout.write(`surface.json: ${counts.total} rows from ${counts.byArea ? Object.keys(counts.byArea).length : 0} areas\n`);
  for (const [area, count] of Object.entries(counts.byArea)) process.stdout.write(`  ${area.padEnd(18)} ${count}\n`);
  process.stdout.write('by kind:\n');
  for (const [kind, count] of Object.entries(counts.byKind)) process.stdout.write(`  ${kind.padEnd(14)} ${count}\n`);
  if (collisions.length) {
    process.stderr.write(`\n${collisions.length} id collision(s) resolved with a component suffix:\n`);
    for (const collision of collisions) {
      process.stderr.write(`  ${collision.id} -> ${collision.resolved} (${collision.first} vs ${collision.second})\n`);
    }
  }
  if (skipped.length) process.stderr.write(`\nskipped ${skipped.length} out-of-scope path(s)\n`);
  if (diagnostics.length) process.stderr.write(`parse diagnostics in ${diagnostics.length} file(s)\n`);

  if (process.argv.includes('--seed-ledger')) {
    const seeded = seedLedger(rows);
    process.stdout.write(`ledger: ${seeded.total} rows (${seeded.added} added) at ${toPosix(relative(repoRoot, seeded.path))}\n`);
  }
}

const invokedDirectly = process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url);
if (invokedDirectly) main();
