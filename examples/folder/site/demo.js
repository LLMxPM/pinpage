// 功能：提供明确标记的浏览器演示目录，不读取真实文件。
export const DEMO_TREE = {
  '': [
    { name: '项目', kind: 'dir', sizeBytes: 0, modifiedMs: Date.now() - 86400000, hidden: false },
    { name: '图片', kind: 'dir', sizeBytes: 0, modifiedMs: Date.now() - 3600000, hidden: false },
    {
      name: '说明.md',
      kind: 'file',
      sizeBytes: 2048,
      modifiedMs: Date.now() - 7200000,
      hidden: false,
    },
    {
      name: '预算.xlsx',
      kind: 'file',
      sizeBytes: 48211,
      modifiedMs: Date.now() - 172800000,
      hidden: false,
    },
  ],
  项目: [
    { name: 'src', kind: 'dir', sizeBytes: 0, modifiedMs: Date.now() - 600000, hidden: false },
    {
      name: 'index.html',
      kind: 'file',
      sizeBytes: 3120,
      modifiedMs: Date.now() - 600000,
      hidden: false,
    },
    {
      name: 'README.md',
      kind: 'file',
      sizeBytes: 1204,
      modifiedMs: Date.now() - 90000000,
      hidden: false,
    },
  ],
  '项目/src': [
    {
      name: 'main.js',
      kind: 'file',
      sizeBytes: 8231,
      modifiedMs: Date.now() - 300000,
      hidden: false,
    },
    {
      name: 'style.css',
      kind: 'file',
      sizeBytes: 2210,
      modifiedMs: Date.now() - 300000,
      hidden: false,
    },
  ],
  图片: [
    {
      name: '壁纸.png',
      kind: 'file',
      sizeBytes: 1048576,
      modifiedMs: Date.now() - 259200000,
      hidden: false,
    },
    {
      name: '截图.jpg',
      kind: 'file',
      sizeBytes: 324511,
      modifiedMs: Date.now() - 5400000,
      hidden: false,
    },
  ],
};
