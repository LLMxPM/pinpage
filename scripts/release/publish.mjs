// 功能：在 GitHub 草稿中准备全部升级附件，校验后发布，禁止覆盖已发布版本。
import { readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { pathToFileURL } from 'node:url';
import semver from 'semver';
import { ROOT, REPOSITORY, command } from './shared.mjs';
import { checkVersion } from '../version.mjs';

/// 从完整列表按标签查找草稿或正式版；按标签查询的 API 不返回草稿，查询失败必须中止。
export function findRelease(tag, run = command) {
  const pages = JSON.parse(
    run('gh', ['api', '--paginate', '--slurp', `repos/${REPOSITORY}/releases?per_page=100`]),
  );
  return pages.flat().find((release) => release.tag_name === tag) ?? null;
}

/// 生成草稿与更新说明；重试只允许修改草稿，不移动已发布标签。
async function prepare(tag) {
  const existing = findRelease(tag);
  if (existing && !existing.draft) throw new Error('该版本已发布，禁止替换附件；请发布新版本');
  if (!existing)
    command('gh', [
      'release',
      'create',
      tag,
      '--repo',
      REPOSITORY,
      '--verify-tag',
      '--draft',
      '--title',
      `PinPage ${tag}`,
      '--generate-notes',
    ]);
  const release = findRelease(tag);
  if (!release?.draft) throw new Error('未找到可写的 Release 草稿，停止发布');
  const body = (release.body || '').split('<!-- pinpage-installation -->')[0].trimEnd();
  const notes = `${body}\n\n<!-- pinpage-installation -->\n---\n\n- 下载 Windows x64 的 \`*-setup.exe\` 安装包，或使用应用内更新\n- 更新仅通过本 GitHub Release 分发，升级包使用 updater 密钥签名\n- 安装包暂未进行 Windows 代码签名，系统可能提示未知发布者\n- 安装更新会关闭页面，网页尚未保存的状态可能丢失\n`;
  await writeFile(path.join(ROOT, 'release-notes.md'), notes);
  command('gh', ['release', 'edit', tag, '--repo', REPOSITORY, '--notes-file', 'release-notes.md']);
}

/// 上传完成后比较附件名称及大小，全部就绪才公开 Release；旧版补发不抢占 Latest。
async function publish(version, tag) {
  const release = findRelease(tag);
  if (!release?.draft) throw new Error('只允许发布现有草稿');
  const directory = path.join(ROOT, 'src-tauri/target/release/bundle/nsis');
  const manifest = JSON.parse(await readFile(path.join(directory, 'latest.json'), 'utf8'));
  if (manifest.version !== version) throw new Error('升级清单版本与标签不一致');
  const installer = new URL(manifest.platforms['windows-x86_64'].url).pathname.split('/').at(-1);
  const files = [installer, `${installer}.sig`, `${installer}.sha256`, 'latest.json'];
  command('gh', [
    'release',
    'upload',
    tag,
    '--repo',
    REPOSITORY,
    ...files.map((name) => path.join(directory, name)),
    '--clobber',
  ]);
  const assets = findRelease(tag).assets;
  for (const name of files) {
    const bytes = await readFile(path.join(directory, name));
    if (!assets.some((asset) => asset.name === name && asset.size === bytes.length))
      throw new Error(`Release 附件缺失或大小错误：${name}`);
  }
  let latest = false;
  if (!semver.prerelease(version)) {
    try {
      const previous = JSON.parse(command('gh', ['api', `repos/${REPOSITORY}/releases/latest`]));
      const previousVersion = previous.tag_name.replace(/^v/, '');
      if (!semver.valid(previousVersion)) throw new Error('当前 Latest 标签格式无效');
      latest = semver.gt(version, previousVersion);
    } catch (error) {
      if (error.message.includes('HTTP 404')) latest = true;
      else throw error;
    }
  }
  command('gh', [
    'release',
    'edit',
    tag,
    '--repo',
    REPOSITORY,
    '--draft=false',
    `--prerelease=${!!semver.prerelease(version)}`,
    `--latest=${latest}`,
  ]);
}

/// 只在命令行直接执行时准备或发布；导入测试不会调用 GitHub 或修改 Release。
async function main() {
  const version = await checkVersion(process.env.RELEASE_TAG);
  const tag = `v${version}`;
  if (process.argv[2] === 'prepare') await prepare(tag);
  else if (process.argv[2] === 'publish') await publish(version, tag);
  else throw new Error('需要 prepare 或 publish 动作');
}

if (process.argv[1] && import.meta.url === pathToFileURL(path.resolve(process.argv[1])).href) {
  main().catch((error) => {
    console.error(error.message);
    process.exitCode = 1;
  });
}
