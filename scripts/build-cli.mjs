// 功能：为 Windows 安装包构建并校验 CLI，其他平台打包时跳过 NSIS 专用资源。
import { copyFile, mkdir } from 'node:fs/promises';
import { spawnSync } from 'node:child_process';
import path from 'node:path';
import { ROOT } from './release/shared.mjs';
import { checkVersion } from './version.mjs';

/// 只为 Windows 目标生成 NSIS 所需 CLI；macOS 的通用与实验应用沿用各自打包流程。
async function buildCli() {
  const expectedVersion = await checkVersion();
  const target = process.env.TAURI_ENV_TARGET_TRIPLE;
  const windowsTarget = target
    ? target.split('-').includes('windows')
    : process.platform === 'win32';
  if (!windowsTarget) {
    console.log('当前目标不分发 Windows CLI，跳过 NSIS 资源构建');
    return;
  }
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
  if (build.status !== 0) throw build.error ?? new Error('CLI 构建失败');
  const directory = path.resolve(ROOT, process.env.CARGO_TARGET_DIR || 'src-tauri/target');
  const executable = path.join(
    directory,
    ...(target ? [target] : []),
    'release',
    'pinpage-cli.exe',
  );
  const version = spawnSync(executable, ['--version'], { encoding: 'utf8', windowsHide: true });
  if (version.status !== 0 || !version.stdout.includes(expectedVersion))
    throw version.error ?? new Error('CLI 版本不一致');
  await mkdir(path.join(ROOT, 'src-tauri/binaries'), { recursive: true });
  await copyFile(executable, path.join(ROOT, 'src-tauri/binaries/pinpage-cli.exe'));
}

await buildCli();
