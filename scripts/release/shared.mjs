// 功能：为版本管理与 Release 构建提供统一版本解析、仓库常量及命令执行。
import { readFile } from 'node:fs/promises';
import { spawnSync } from 'node:child_process';
import path from 'node:path';
import semver from 'semver';

export const ROOT = path.resolve(import.meta.dirname, '../..');
export const REPOSITORY = 'LLMxPM/pinpage';

/// 发布版本必须是规范 SemVer，不允许前缀、构建元数据或 Windows 无法区分的额外版本。
export function parseVersion(input) {
  if (semver.valid(input) !== input || semver.parse(input).build.length)
    throw new Error(`版本格式无效：${input}`);
  return input;
}

/// 原样读取三处版本与 Cargo 锁文件中的应用版本，不修改依赖版本。
export async function readVersions(root = ROOT) {
  const [packageText, configText, cargo, lock] = await Promise.all([
    readFile(path.join(root, 'package.json'), 'utf8'),
    readFile(path.join(root, 'src-tauri/tauri.conf.json'), 'utf8'),
    readFile(path.join(root, 'src-tauri/Cargo.toml'), 'utf8'),
    readFile(path.join(root, 'src-tauri/Cargo.lock'), 'utf8'),
  ]);
  const packageJson = JSON.parse(packageText);
  const config = JSON.parse(configText);
  return {
    packageJson,
    config,
    cargo,
    lock,
    versions: [
      packageJson.version,
      config.version,
      cargo.match(/\[package\][\s\S]*?\nversion = "([^"]+)"/)?.[1],
      lock.match(/\[\[package\]\]\r?\nname = "pinpage"\r?\nversion = "([^"]+)"/)?.[1],
    ],
  };
}

/// 无 shell 地执行构建辅助命令；调用者不得把密钥作为参数传入。
export function command(file, args, options = {}) {
  const result = spawnSync(file, args, { cwd: ROOT, encoding: 'utf8', ...options });
  if (result.status !== 0)
    throw new Error(result.stderr?.trim() || result.error?.message || `${file} 执行失败`);
  return result.stdout.trim();
}
