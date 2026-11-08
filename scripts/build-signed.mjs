// 功能：从用户目录或环境变量读取 updater 密钥，构建签名包且不回显私钥和密码。
import { readFile } from 'node:fs/promises';
import { homedir } from 'node:os';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import { ROOT } from './release/shared.mjs';
import { checkVersion } from './version.mjs';

try {
  await checkVersion();
  const directory = path.join(homedir(), '.tauri/pinpage');
  const key =
    process.env.TAURI_SIGNING_PRIVATE_KEY ||
    (await readFile(path.join(directory, 'updater.key'), 'utf8'));
  const password =
    process.env.TAURI_SIGNING_PRIVATE_KEY_PASSWORD ||
    (await readFile(path.join(directory, 'updater.password'), 'utf8'));
  const result = spawnSync(
    process.execPath,
    [path.join(ROOT, 'node_modules/@tauri-apps/cli/tauri.js'), 'build', ...process.argv.slice(2)],
    {
      cwd: ROOT,
      stdio: 'inherit',
      windowsHide: true,
      env: {
        ...process.env,
        TAURI_SIGNING_PRIVATE_KEY: key.trim(),
        TAURI_SIGNING_PRIVATE_KEY_PASSWORD: password.trim(),
      },
    },
  );
  process.exitCode = result.status ?? 1;
} catch (error) {
  console.error(`签名构建失败：${error.message}`);
  process.exitCode = 1;
}
