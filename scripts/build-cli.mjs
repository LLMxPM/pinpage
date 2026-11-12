// 功能：构建随安装包分发的 CLI，保持版本一致并把可执行文件准备给 NSIS。
import { copyFile, mkdir } from 'node:fs/promises';
import { spawnSync } from 'node:child_process';
import path from 'node:path';
import { ROOT } from './release/shared.mjs';
import { checkVersion } from './version.mjs';

await checkVersion();
const target = process.env.TAURI_ENV_TARGET_TRIPLE;
const args = [
  'build',
  '--manifest-path',
  'src-tauri/Cargo.toml',
  '-p',
  'pinpage-cli',
  '--release',
  '--locked',
];
if (target) args.push('--target', target);
const build = spawnSync('cargo', args, { cwd: ROOT, stdio: 'inherit', windowsHide: true });
if (build.status !== 0) process.exit(build.status ?? 1);
const directory = path.resolve(ROOT, process.env.CARGO_TARGET_DIR || 'src-tauri/target');
const executable = path.join(directory, ...(target ? [target] : []), 'release', 'pinpage-cli.exe');
const version = spawnSync(executable, ['--version'], { encoding: 'utf8', windowsHide: true });
if (version.status !== 0 || !version.stdout.includes(await checkVersion()))
  throw new Error('CLI 版本不一致');
await mkdir(path.join(ROOT, 'src-tauri/binaries'), { recursive: true });
await copyFile(executable, path.join(ROOT, 'src-tauri/binaries/pinpage-cli.exe'));
