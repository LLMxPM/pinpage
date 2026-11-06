// 功能：将规范化的系统路径转换为便于阅读的界面文字。

/// 去掉 Windows 规范化路径的扩展前缀，保留原始授权路径供后端使用。
export function displayPath(path: string): string {
  const slash = String.fromCharCode(92);
  const prefix = slash + slash + '?' + slash;
  if (path.startsWith(prefix + 'UNC' + slash)) return slash + slash + path.slice(8);
  return path.startsWith(prefix) ? path.slice(4) : path;
}
