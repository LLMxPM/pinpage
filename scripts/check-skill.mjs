// 功能：校验唯一页面技能、随包资源与自包含引用，以及仓库示例接入文档的本地链接。
import { readFile, readdir, stat } from 'node:fs/promises';
import path from 'node:path';

const ROOT = path.resolve(import.meta.dirname, '..');
const SKILL = 'skills/pinpage-pages';
const GUIDES = ['AGENTS.md', 'examples/AGENTS.md', 'docs/examples-development.md'];
const problems = [];
const documents = new Map();

/// 递归列出仓库相对文件；允许技能发现目录不存在，拒绝包内链接文件。
async function listFiles(directory) {
  const entries = await readdir(path.join(ROOT, directory), { withFileTypes: true }).catch(
    (error) => {
      if (error.code === 'ENOENT') return [];
      throw error;
    },
  );
  const files = [];
  for (const entry of entries) {
    const file = `${directory}/${entry.name}`;
    if (entry.isSymbolicLink()) problems.push(`${file} 不能使用符号链接或目录联接`);
    else if (entry.isDirectory()) files.push(...(await listFiles(file)));
    else files.push(file);
  }
  return files;
}

/// 读取并缓存文档，移除代码围栏，避免将示例代码当成本地链接或章节。
async function documentText(file) {
  if (!documents.has(file)) {
    const source = await readFile(path.join(ROOT, file), 'utf8');
    documents.set(file, source.replace(/^```[^\n]*\n[\s\S]*?^```[ \t]*$/gm, ''));
  }
  return documents.get(file);
}

/// 按 GitHub 的常用标题形式提取章节锚点，支持中文、行内代码与重复标题。
function headingAnchors(source) {
  const anchors = new Set();
  for (const match of source.matchAll(/^#{1,6}\s+(.+?)\s*#*\s*$/gm)) {
    const base = match[1]
      .trim()
      .toLowerCase()
      .replace(/[^\p{L}\p{M}\p{N}_\-\s]/gu, '')
      .replace(/\s/g, '-');
    let anchor = base;
    for (let suffix = 1; anchors.has(anchor); suffix += 1) anchor = `${base}-${suffix}`;
    anchors.add(anchor);
  }
  return anchors;
}

/// 校验本地文件与章节；技能中的引用必须包含在实际内嵌的资源集合中。
async function checkLinks(file, bundle) {
  const source = await documentText(file);
  for (const match of source.matchAll(/!?\[[^\]\n]*\]\(([^)\n]+)\)/g)) {
    const link = match[1];
    if (/^[a-z][a-z\d+.-]*:|^\/\//i.test(link)) continue;
    const [pathname, fragment] = link.split('#');
    const target = pathname
      ? path.resolve(ROOT, path.dirname(file), decodeURIComponent(pathname))
      : path.join(ROOT, file);
    const relative = path.relative(ROOT, target).replaceAll(path.sep, '/');
    if (bundle && !bundle.has(relative)) {
      problems.push(`${file} 的引用不在技能包内：${link}`);
      continue;
    }
    const info = await stat(target).catch((error) => {
      if (error.code === 'ENOENT') return null;
      throw error;
    });
    if (!info?.isFile()) {
      problems.push(`${file} 的引用文件不存在：${link}`);
      continue;
    }
    if (fragment && relative.endsWith('.md')) {
      const anchors = headingAnchors(await documentText(relative));
      if (!anchors.has(decodeURIComponent(fragment)))
        problems.push(`${file} 的章节不存在：${link}`);
    }
  }
}

const files = await listFiles(SKILL);
const bundle = new Set(files);
const discovered = [
  ...(await listFiles('skills')),
  ...(await listFiles('.agents/skills')),
  ...(await listFiles('.codex/skills')),
].filter((file) => file.endsWith('/SKILL.md'));
if (discovered.length !== 1 || discovered[0] !== `${SKILL}/SKILL.md`)
  problems.push(`仓库应只保留 ${SKILL}/SKILL.md，实际发现：${discovered.join('、') || '无'}`);

const entry = await readFile(path.join(ROOT, SKILL, 'SKILL.md'), 'utf8');
const frontmatter = entry.match(/^---\r?\n([\s\S]*?)\r?\n---/);
if (!frontmatter || !/^name: pinpage-pages\s*$/m.test(frontmatter[1]))
  problems.push('技能入口缺少正确的 name: pinpage-pages');
if (!frontmatter || !/^description: \S.+$/m.test(frontmatter[1]))
  problems.push('技能入口缺少 description');

const native = await readFile(path.join(ROOT, 'crates/pinpage-skills/src/lib.rs'), 'utf8');
const embedded = new Set(
  [
    ...native.matchAll(
      /include_(?:str|bytes)!\("\.\.\/\.\.\/\.\.\/skills\/pinpage-pages\/([^"\n]+)"\)/g,
    ),
  ].map((match) => `${SKILL}/${match[1]}`),
);
for (const file of files) {
  if (!embedded.has(file)) problems.push(`${file} 未编入技能分发包`);
}
for (const file of embedded) {
  if (!bundle.has(file)) problems.push(`技能内嵌资源不存在：${file}`);
}

for (const file of files.filter((file) => file.endsWith('.md'))) await checkLinks(file, bundle);
for (const file of GUIDES) await checkLinks(file);

if (problems.length) {
  console.error(`页面技能检查未通过（${problems.length} 项）：`);
  for (const problem of problems) console.error(`  - ${problem}`);
  process.exit(1);
}
console.log(`页面技能检查通过：唯一技能、${files.length} 个随包文件、自包含引用与示例接入链接有效`);
