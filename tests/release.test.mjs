// 功能：验证版本同步与标签约束，以及 GitHub 更新清单必须绑定具体版本和签名。
import assert from 'node:assert/strict';
import { mkdtemp, mkdir, readFile, writeFile, rm } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import test from 'node:test';
import { checkVersion, setVersion } from '../scripts/version.mjs';
import { parseVersion } from '../scripts/release/shared.mjs';
import { createManifest } from '../scripts/release/artifacts.mjs';
import { findRelease } from '../scripts/release/publish.mjs';

test('版本同步只修改应用字段，保持依赖版本；标签和锁文件不一致必须报错', async () => {
  const root = await mkdtemp(path.join(os.tmpdir(), 'pinpage-version-'));
  try {
    await mkdir(path.join(root, 'src-tauri'));
    await writeFile(
      path.join(root, 'package.json'),
      JSON.stringify({ version: '0.1.0', dependencies: { demo: '1.0.0' } }),
    );
    await writeFile(path.join(root, 'src-tauri/tauri.conf.json'), '{"version":"0.1.0"}');
    await writeFile(
      path.join(root, 'src-tauri/Cargo.toml'),
      '[package]\nname = "pinpage"\nversion = "0.1.0"\n[dependencies]\ndemo = "1.0.0"\n',
    );
    await writeFile(
      path.join(root, 'src-tauri/Cargo.lock'),
      '[[package]]\nname = "demo"\nversion = "1.0.0"\n[[package]]\nname = "pinpage"\nversion = "0.1.0"\n',
    );
    await setVersion('0.2.0', root);
    assert.equal(await checkVersion('v0.2.0', root), '0.2.0');
    assert.match(
      await readFile(path.join(root, 'src-tauri/Cargo.lock'), 'utf8'),
      /name = "demo"\nversion = "1.0.0"/,
    );
    assert.equal(
      JSON.parse(await readFile(path.join(root, 'package.json'), 'utf8')).dependencies.demo,
      '1.0.0',
    );
    await assert.rejects(checkVersion('v0.1.0', root));
    await writeFile(
      path.join(root, 'src-tauri/Cargo.lock'),
      '[[package]]\nname = "pinpage"\nversion = "0.1.0"\n',
    );
    await assert.rejects(checkVersion(undefined, root));
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

test('版本使用完整 SemVer，拒绝补零、非法预发布与构建元数据', () => {
  for (const value of ['0.2.0', '1.0.0-beta.1']) assert.equal(parseVersion(value), value);
  for (const value of ['v0.2.0', '01.2.0', '1.2', '1.0.0-01', '1.0.0+build', undefined])
    assert.throws(() => parseVersion(value));
});

test('更新清单固定仓库、平台和版本附件，签名必须是内容', () => {
  const manifest = createManifest(
    '0.2.0',
    'PinPage_0.2.0_x64-setup.exe',
    'c2lnbmF0dXJl',
    '更新说明',
  );
  assert.equal(
    manifest.platforms['windows-x86_64'].url,
    'https://github.com/LLMxPM/pinpage/releases/download/v0.2.0/PinPage_0.2.0_x64-setup.exe',
  );
  assert.equal(manifest.platforms['windows-x86_64'].signature, 'c2lnbmF0dXJl');
  for (const signature of ['', 'file.sig', 'https://example.com/signature'])
    assert.throws(() => createManifest('0.2.0', 'PinPage_0.2.0_x64-setup.exe', signature, ''));
  assert.throws(() => createManifest('0.2.0', 'PinPage_0.1.0_x64-setup.exe', 'c2ln', ''));
  assert.throws(() => createManifest('0.2.0', '../PinPage_0.2.0_x64-setup.exe', 'c2ln', ''));
});

test('Release 查询覆盖后续分页中的草稿，并保留正式版本的覆盖保护所需状态', () => {
  const published = { id: 1, tag_name: 'v0.1.0', draft: false, assets: [] };
  const draft = { id: 2, tag_name: 'v0.2.0', draft: true, body: '待发布', assets: [] };
  const response = () => JSON.stringify([[published], [draft]]);
  assert.deepEqual(findRelease('v0.2.0', response), draft);
  assert.deepEqual(findRelease('v0.1.0', response), published);
  assert.equal(findRelease('v0.3.0', response), null);
});

test('Release 列表的权限与网络错误直接中止，不被误判为可以新建草稿', () => {
  const unavailable = new Error('HTTP 404: 无法读取仓库 Release');
  assert.throws(
    () =>
      findRelease('v0.2.0', () => {
        throw unavailable;
      }),
    (error) => error === unavailable,
  );
});
