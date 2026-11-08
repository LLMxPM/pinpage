// 功能：校验前端与原生窗口尺寸上下限一致，避免只改一侧导致提交被拒绝。
import { readFile } from 'node:fs/promises';
import path from 'node:path';
import process from 'node:process';

const ROOT = path.resolve(import.meta.dirname, '..');
const NAMES = ['MIN_WIDTH', 'MAX_WIDTH', 'MIN_HEIGHT', 'MAX_HEIGHT'];
const problems = [];

async function values(file, pattern) {
  const text = await readFile(path.join(ROOT, file), 'utf8');
  const found = new Map();
  for (const match of text.matchAll(pattern)) found.set(match[1], Number(match[2]));
  return found;
}

const native = await values(
  'src-tauri/src/windows/limits.rs',
  /pub const (MIN_WIDTH|MAX_WIDTH|MIN_HEIGHT|MAX_HEIGHT): f64 = ([\d.]+);/g,
);
const ui = await values(
  'src/features/pins/geometryLimits.ts',
  /export const (MIN_WIDTH|MAX_WIDTH|MIN_HEIGHT|MAX_HEIGHT) = (\d+);/g,
);

for (const name of NAMES) {
  if (!native.has(name)) problems.push(`limits.rs 缺少 ${name}`);
  if (!ui.has(name)) problems.push(`geometryLimits.ts 缺少 ${name}`);
  if (native.has(name) && ui.has(name) && native.get(name) !== ui.get(name)) {
    problems.push(`${name} 不一致：native=${native.get(name)}，ui=${ui.get(name)}`);
  }
}
if (native.get('MIN_WIDTH') >= native.get('MAX_WIDTH')) problems.push('宽度下限必须小于上限');
if (native.get('MIN_HEIGHT') >= native.get('MAX_HEIGHT')) problems.push('高度下限必须小于上限');

if (problems.length) {
  console.error(`窗口尺寸限制检查未通过（${problems.length} 项）：`);
  for (const problem of problems) console.error(`  - ${problem}`);
  process.exit(1);
}
console.log(
  `窗口尺寸限制一致：${native.get('MIN_WIDTH')}～${native.get('MAX_WIDTH')} × ${native.get('MIN_HEIGHT')}～${native.get('MAX_HEIGHT')}`,
);
