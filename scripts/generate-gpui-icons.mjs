// Generate one SVG per lucide icon the app imports, so the GPUI port uses the
// exact icon set the components reference.
import { readdirSync, readFileSync, writeFileSync, mkdirSync, existsSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

const srcDir = resolve(process.argv[2] ?? 'src');
const outDir = resolve(process.argv[3] ?? 'gpui/assets/icons');
const lucideDir = resolve(process.argv[4] ?? 'node_modules/lucide-react/dist/esm/icons');

function walk(dir) {
  const out = [];
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    const path = join(dir, entry.name);
    if (entry.isDirectory()) out.push(...walk(path));
    else if (/\.(tsx|ts)$/.test(entry.name)) out.push(path);
  }
  return out;
}

const names = new Set();
for (const file of walk(srcDir)) {
  const text = readFileSync(file, 'utf8');
  const re = /import\s*\{([^}]+)\}\s*from\s*['"]lucide-react['"]/g;
  let match;
  while ((match = re.exec(text))) {
    for (const part of match[1].split(',')) {
      const name = part.trim().split(/\s+as\s+/)[0].trim();
      if (name) names.add(name);
    }
  }
}

const kebab = (name) => name
  .replace(/([a-z0-9])([A-Z])/g, '$1-$2')
  .replace(/([A-Z])([A-Z][a-z])/g, '$1-$2')
  .toLowerCase();

// The package's main module maps every export name (including aliases) to its
// icon module; parse it so renamed icons resolve.
const aliasMap = new Map();
const mainModule = readFileSync(resolve(lucideDir, '..', 'lucide-react.js'), 'utf8');
const exportRe = /export \{([^}]+)\} from '\.\/icons\/([\w-]+)\.js'/g;
let exportMatch;
while ((exportMatch = exportRe.exec(mainModule))) {
  for (const part of exportMatch[1].split(',')) {
    const name = part.trim().replace(/^default as /, '');
    if (name) aliasMap.set(name, exportMatch[2]);
  }
}

mkdirSync(outDir, { recursive: true });
let written = 0;
const missing = [];
for (const name of [...names].sort()) {
  const slug = aliasMap.get(name) ?? kebab(name);
  const file = join(lucideDir, `${slug}.js`);
  if (!existsSync(file)) {
    missing.push(name);
    continue;
  }
  const mod = await import(pathToFileURL(file).href);
  const nodes = mod.__iconNode;
  if (!nodes) {
    missing.push(name);
    continue;
  }
  const body = nodes.map(([tag, attrs]) => {
    const a = Object.entries(attrs)
      .filter(([key]) => key !== 'key')
      .map(([key, value]) => `${key}="${value}"`)
      .join(' ');
    return `<${tag} ${a}/>`;
  }).join('');
  const svg = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">${body}</svg>\n`;
  writeFileSync(join(outDir, `${slug}.svg`), svg);
  written++;
}
console.log(`icons: ${written} written, ${missing.length} missing`);
if (missing.length) console.log('missing:', missing.join(', '));
