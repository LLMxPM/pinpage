// 功能：从已签名 NSIS 安装包生成 GitHub Release 升级清单及 SHA-256 校验文件。
import { readdir, readFile, writeFile } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import path from 'node:path';
import { pathToFileURL } from 'node:url';
import { ROOT, REPOSITORY, parseVersion } from './shared.mjs';
import { checkVersion } from '../version.mjs';

/// 清单只指向本仓库具体版本，签名字段使用 .sig 内容，不使用路径或 URL。
export function createManifest(
  version,
  installer,
  signature,
  notes,
  time = new Date().toISOString(),
) {
  parseVersion(version);
  if (
    !/^PinPage_[0-9A-Za-z.-]+_x64-setup\.exe$/.test(installer) ||
    !installer.includes(`_${version}_`)
  )
    throw new Error('安装包名称与版本不匹配');
  if (!signature.trim() || !/^[A-Za-z0-9+/=\r\n]+$/.test(signature))
    throw new Error('升级签名内容无效');
  return {
    version,
    notes,
    pub_date: time,
    platforms: {
      'windows-x86_64': {
        signature: signature.trim(),
        url: `https://github.com/${REPOSITORY}/releases/download/v${version}/${installer}`,
      },
    },
  };
}

/// 只选择当前版本的 x64 安装包，避免本地旧构建产物混入本次发布。
async function main() {
  const version = await checkVersion(process.env.RELEASE_TAG);
  const directory = path.join(ROOT, 'src-tauri/target/release/bundle/nsis');
  const installers = (await readdir(directory)).filter(
    (name) => name === `PinPage_${version}_x64-setup.exe`,
  );
  if (installers.length !== 1) throw new Error('必须且只能生成一个 Windows x64 安装包');
  const installer = installers[0];
  const [bytes, signature, notes] = await Promise.all([
    readFile(path.join(directory, installer)),
    readFile(path.join(directory, `${installer}.sig`), 'utf8'),
    readFile(path.join(ROOT, 'release-notes.md'), 'utf8'),
  ]);
  const manifest = createManifest(version, installer, signature, notes);
  await writeFile(
    path.join(directory, `${installer}.sha256`),
    `${createHash('sha256').update(bytes).digest('hex')}  ${installer}\n`,
  );
  await writeFile(path.join(directory, 'latest.json'), `${JSON.stringify(manifest, null, 2)}\n`);
  console.log(`已生成升级清单及校验文件：v${version}`);
}

if (process.argv[1] && import.meta.url === pathToFileURL(path.resolve(process.argv[1])).href) {
  main().catch((error) => {
    console.error(error.message);
    process.exitCode = 1;
  });
}
