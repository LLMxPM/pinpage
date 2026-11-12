// 功能：从统一宿主契约生成自包含的公开技能参考，校验随包接口说明不漂移。
import { readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { format, resolveConfig } from 'prettier';
import { ROOT } from './release/shared.mjs';

/// 保留原生能力与 CSP 契约，补充独立页面接入示例并移除仓库内部依赖。
function buildHostReference(source) {
  const contract = source
    .split('## 页面接入方式')[0]
    .replace('# 页面原生能力契约', '# 宿主能力契约')
    .replace(
      /本文是能力层与页面作者之间的契约；实现位于[^\n]+/,
      '本文约定内容页可调用的原生能力；智能体的 CLI 命令见 [CLI 手册](cli.md)。',
    )
    .replace(
      '`code` 是 `locales/*/native.json` 的资源键，页面可自行映射文案；',
      '`code` 是宿主消息码，页面可自行映射为提示；',
    )
    .replace('（行情示例即据此联网）', '');
  const integration = [
    '## 接入与能力探测',
    '',
    '只在需要宿主数据时接入，通过 `window.__TAURI_INTERNALS__.invoke(command, args)` 调用下文列出的命令。独立静态页无需引入 Tauri npm 包；普通浏览器没有宿主对象时显示明确的不可用状态，演示数据需标注。',
    '',
    '```javascript',
    '// 功能：提供独立页面的宿主环境探测入口。',
    '/// 返回宿主信息；普通浏览器返回 null，原生调用失败交给调用方显示结构化错误。',
    'export async function readHostInfo() {',
    '  const host = window.__TAURI_INTERNALS__;',
    '  if (!host || typeof host.invoke !== "function") return null;',
    '  return host.invoke("host_info");',
    '}',
    '```',
    '',
    '先读取 `host_info.capabilities`，按能力名和契约版本探测后再使用；不根据应用版本猜测能力，不尝试其他应用命令。异步调用失败时处理消息码、保留当前可用内容，不把失败当成零值或空文件夹。',
    '',
    'L1 提供文件列表和打开／定位操作，不读取文件正文。CLI 安装源码不会授予内容页访问源码目录的权限；来源更新后需要重新授权。用户主动选择数据文件、联网及缓存实现见 [页面制作指南](page-development.md#数据来源联网与持久状态)。',
  ].join('\n');
  return (
    contract.replace('## 分级模型', integration + '\n\n## 分级模型').trimEnd() +
    '\n\n## 联调与降级\n\n浏览器预览只检查不可用／演示状态及布局。原生选择器、取消授权、根目录失效、越界拒绝、真实采样和 CSP 响应头需要在 PinPage 中验证；使用 [CLI 诊断与截图](cli.md#诊断与真实桌面截图)记录结果。页面目录、透明、滚动和尺寸适配统一见 [页面制作指南](page-development.md)。\n'
  );
}

const source = await readFile(
  path.join(ROOT, '.agents/skills/pinpage-examples/references/native-capabilities.md'),
  'utf8',
);
const target = path.join(ROOT, 'skills/pinpage-pages/references/host-api.md');
const generated = await format(buildHostReference(source), {
  ...(await resolveConfig(target)),
  filepath: target,
});
if (process.argv.includes('--check')) {
  const actual = await readFile(target, 'utf8');
  if (actual.replaceAll('\r\n', '\n') !== generated.replaceAll('\r\n', '\n'))
    throw new Error('公开 skill 的宿主契约已过期，请运行 pnpm skill:sync');
  console.log('公开 skill 宿主契约一致');
} else await writeFile(target, generated);
