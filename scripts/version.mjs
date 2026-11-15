// 功能：以 package.json 为版本来源，同步或校验 Tauri、Cargo 与锁文件中的应用版本。
import { writeFile } from 'node:fs/promises';
import path from 'node:path';
import { pathToFileURL } from 'node:url';
import { format, resolveConfig } from 'prettier';
import semver from 'semver';
import { ROOT, parseVersion, readVersions } from './release/shared.mjs';

/// 同步应用与工作区成员版本，先检查文件形状再写入；可传入临时项目目录进行独立测试。
export async function setVersion(version, root = ROOT) {
  parseVersion(version);
  const values = await readVersions(root);
  if (values.versions.some((value) => value === undefined))
    throw new Error('未能定位全部版本字段，未修改文件');
  let skillText = null;
  if (values.skill) {
    const skillVersion = semver.parse(parseVersion(values.skill.version));
    const appVersion = semver.parse(version);
    // 同一应用系列保留技能补丁号；跨系列时从新系列的 .0 开始。
    const next =
      skillVersion.major === appVersion.major && skillVersion.minor === appVersion.minor
        ? values.skill.version
        : `${appVersion.major}.${appVersion.minor}.0`;
    skillText = values.skill.text.replace(
      /(pub const VERSION: &str = ")[^"]+(";)/,
      (_, prefix, suffix) => prefix + next + suffix,
    );
  }
  values.packageJson.version = version;
  values.config.version = version;
  const cargo = values.cargo.replace(
    /(\[package\][\s\S]*?\nversion = ")[^"]+(")/,
    (_, prefix, suffix) => `${prefix}${version}${suffix}`,
  );
  const lock = values.lock.replace(
    /(\[\[package\]\]\r?\nname = "pinpage(?:-cli|-protocol|-skills)?"\r?\nversion = ")[^"]+(")/g,
    (_, prefix, suffix) => prefix + version + suffix,
  );
  for (const member of values.members) {
    await writeFile(
      path.join(root, member.relative),
      member.text.replace(
        /(\[package\][\s\S]*?\nversion = ")[^"]+(")/,
        (_, prefix, suffix) => prefix + version + suffix,
      ),
    );
  }
  for (const [relative, value] of [
    ['package.json', values.packageJson],
    ['src-tauri/tauri.conf.json', values.config],
  ]) {
    const filepath = path.join(root, relative);
    const options = await resolveConfig(filepath);
    await writeFile(
      filepath,
      await format(JSON.stringify(value), { ...options, filepath, parser: 'json' }),
    );
  }
  await writeFile(path.join(root, 'src-tauri/Cargo.toml'), cargo);
  await writeFile(path.join(root, 'Cargo.lock'), lock);
  if (values.skill) await writeFile(path.join(root, values.skill.relative), skillText);
}

/// 检查项目版本及可选 Git 标签；CI 与本地构建共用，避免校验逻辑漂移。
export async function checkVersion(tag, root = ROOT) {
  const { versions, skill } = await readVersions(root);
  const version = parseVersion(versions[0]);
  if (versions.some((value) => value !== version))
    throw new Error(`项目版本不一致：${versions.join(' / ')}`);
  if (skill) {
    const appVersion = semver.parse(version);
    const skillVersion = semver.parse(parseVersion(skill.version));
    if (skillVersion.major !== appVersion.major || skillVersion.minor !== appVersion.minor)
      throw new Error(`技能版本 ${skill.version} 与应用 ${version} 的主版本、次版本必须一致`);
  }
  if (tag && tag !== `v${version}`) throw new Error(`标签 ${tag} 与项目版本 v${version} 不一致`);
  return version;
}

if (process.argv[1] && import.meta.url === pathToFileURL(path.resolve(process.argv[1])).href) {
  try {
    if (process.argv[2] === '--check')
      console.log(`版本校验通过：${await checkVersion(process.argv[3])}`);
    else {
      await setVersion(process.argv[2]);
      console.log(`版本已同步：${await checkVersion()}`);
    }
  } catch (error) {
    console.error(error.message);
    process.exitCode = 1;
  }
}
