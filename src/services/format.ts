// 功能：按当前语言格式化展示数字，并保持字节换算口径稳定。
import { localeTag } from '../i18n';

const UNITS = ['KiB', 'MiB', 'GiB'];

/// 按当前语言渲染数字，只影响展示；小数位上限沿用原有口径。
function number(value: number, digits: number): string {
  return new Intl.NumberFormat(localeTag(), { maximumFractionDigits: digits }).format(value);
}

/// 把字节数按 1024 换算为体积文字，单位固定为 B / KiB / MiB / GiB。
export function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${number(bytes, 0)} B`;
  let value = bytes / 1024;
  let index = 0;
  while (value >= 1024 && index < UNITS.length - 1) {
    value /= 1024;
    index += 1;
  }
  return `${value < 10 ? number(value, 1) : number(Math.round(value), 0)} ${UNITS[index]}`;
}
