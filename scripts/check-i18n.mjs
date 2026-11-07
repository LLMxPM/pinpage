// 功能：校验中英文资源的键、参数与语法约束，并检查代码引用的消息键是否落库。
import { readdir, readFile } from 'node:fs/promises';
import path from 'node:path';
import process from 'node:process';

const ROOT = path.resolve(import.meta.dirname, '..');
const LOCALES = ['zh-CN', 'en-US'];
// native 资源会被 Rust 直接读取，键必须落在这些顶层命名空间内。
const NATIVE_SPACES = ['tray', 'window', 'picker', 'names', 'restore', 'errors'];

const problems = [];

function fail(file, detail) {
  problems.push(`${file}: ${detail}`);
}

/// 递归展开 JSON 资源为「点分键 -> 文案」，同时记录非字符串值。
function flatten(value, prefix, file, out) {
  if (typeof value === 'string') {
    out.set(prefix, value);
    return;
  }
  if (value && typeof value === 'object' && !Array.isArray(value)) {
    for (const [key, child] of Object.entries(value))
      flatten(child, prefix ? `${prefix}.${key}` : key, file, out);
    return;
  }
  fail(file, `${prefix || '<root>'} 的翻译值必须是字符串`);
}

/// 提取具名插值参数，同时兼容 {name} 与 {name, plural, ...} 两种写法。
function paramsOf(text) {
  const names = new Set();
  for (const match of text.matchAll(/\{\s*([A-Za-z_][A-Za-z0-9_]*)/g)) names.add(match[1]);
  return [...names].sort();
}

async function readCatalog(locale, space) {
  const file = `locales/${locale}/${space}.json`;
  const text = await readFile(path.join(ROOT, 'locales', locale, `${space}.json`), 'utf8');
  const messages = new Map();
  flatten(JSON.parse(text), '', file, messages);
  return { file, messages };
}

/// 比较同一命名空间在两个语言下的键集合与参数集合。
async function compare(space) {
  const loaded = [];
  for (const locale of LOCALES) loaded.push(await readCatalog(locale, space));
  const [base, other] = loaded;
  for (const key of base.messages.keys()) {
    if (!other.messages.has(key)) fail(other.file, `缺少键 ${key}`);
  }
  for (const key of other.messages.keys()) {
    if (!base.messages.has(key)) fail(base.file, `存在多余键 ${key}`);
  }
  for (const entry of loaded) {
    for (const [key, text] of entry.messages)
      if (!text.trim()) fail(entry.file, `键 ${key} 的翻译为空`);
  }
  for (const [key, text] of base.messages) {
    const otherText = other.messages.get(key);
    if (otherText === undefined) continue;
    const left = paramsOf(text).join(',');
    const right = paramsOf(otherText).join(',');
    if (left !== right) {
      fail(
        other.file,
        `键 ${key} 的参数集合不一致：${base.file}=[${left}] ${other.file}=[${right}]`,
      );
    }
  }
  return loaded;
}

/// native 资源只允许普通文字与具名参数，禁止复数、链接消息与任何标记语言。
function checkNative(entries) {
  for (const entry of entries) {
    for (const [key, text] of entry.messages) {
      if (!NATIVE_SPACES.includes(key.split('.')[0]))
        fail(entry.file, `键 ${key} 的顶层命名空间不受支持`);
      if (text.includes('|')) fail(entry.file, `键 ${key} 含复数分隔符，native 资源不支持复数`);
      if (text.includes('@')) fail(entry.file, `键 ${key} 含链接消息标记 @`);
      if (/[<>]/.test(text)) fail(entry.file, `键 ${key} 含尖括号，native 资源不允许标记语言`);
      if (/\{\s*[0-9]/.test(text)) fail(entry.file, `键 ${key} 使用位置参数，必须改为具名参数`);
      if (/\{\s*[A-Za-z0-9_]+\s*,/.test(text)) fail(entry.file, `键 ${key} 使用 Vue 专用语法`);
      if (text.includes('\\"') || text.includes('\\n'))
        fail(entry.file, `键 ${key} 含转义控制内容`);
    }
  }
}

async function walk(dir, out) {
  for (const item of await readdir(dir, { withFileTypes: true })) {
    const full = path.join(dir, item.name);
    if (item.isDirectory()) await walk(full, out);
    else out.push(full);
  }
  return out;
}

/// Rust 侧只以字符串字面量引用 native 键，逐个确认已落库。
/// 测试夹具会刻意使用不存在的键来验证回退路径，因此跳过测试文件。
async function checkRustKeys(nativeZh) {
  const files = await walk(path.join(ROOT, 'src-tauri'), []);
  const pattern = new RegExp(`"((?:${NATIVE_SPACES.join('|')})\\.[A-Za-z0-9_]+)"`, 'g');
  const isTest = (name) =>
    name.endsWith('_tests.rs') || /[\\/](?:tests)[\\/]/.test(name.replace(/\\/g, '/'));
  for (const file of files.filter((item) => item.endsWith('.rs') && !isTest(item))) {
    const text = await readFile(file, 'utf8');
    for (const match of text.matchAll(pattern)) {
      if (!nativeZh.has(match[1])) {
        fail(path.relative(ROOT, file), `引用的 native 键 ${match[1]} 不存在于 zh-CN/native.json`);
      }
    }
  }
}

/// 前端字面量使用的 ui / native 键同样必须存在，避免运行时才暴露缺键。
async function checkFrontendKeys(ui, native) {
  const files = await walk(path.join(ROOT, 'src'), []);
  for (const file of files.filter((item) => /\.(ts|vue)$/.test(item))) {
    const text = await readFile(file, 'utf8');
    for (const match of text.matchAll(/['"`](ui\.[A-Za-z0-9_.]+)['"`]/g)) {
      const key = match[1].slice('ui.'.length);
      if (!ui.has(key))
        fail(path.relative(ROOT, file), `使用的翻译键 ${match[1]} 不存在于 zh-CN/ui.json`);
    }
    for (const match of text.matchAll(
      /['"`]native\.((?:[A-Za-z0-9_]+\.){1,2}[A-Za-z0-9_]+)['"`]/g,
    )) {
      if (!native.has(match[1]))
        fail(path.relative(ROOT, file), `使用的翻译键 native.${match[1]} 不存在`);
    }
  }
}

const ui = await compare('ui');
const native = await compare('native');
checkNative(native);
await checkRustKeys(native[0].messages);
await checkFrontendKeys(ui[0].messages, native[0].messages);

if (problems.length) {
  console.error(`i18n 检查未通过（${problems.length} 项）：`);
  for (const problem of problems) console.error(`  - ${problem}`);
  process.exit(1);
}
console.log(`i18n 检查通过：ui ${ui[0].messages.size} 键，native ${native[0].messages.size} 键。`);
