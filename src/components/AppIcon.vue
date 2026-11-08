<!-- 功能：提供统一的轻量 SVG 图标，避免依赖远程图标资源；settings/refresh 路径取自 Lucide（ISC 许可）。 -->
<script setup lang="ts">
withDefaults(defineProps<{ name: string; size?: number }>(), { size: 20 });
const paths: Record<string, string> = {
  pin: 'M9 3h6l-1 6 4 4v2H6v-2l4-4-1-6ZM12 15v7',
  plus: 'M12 5v14M5 12h14',
  link: 'm10 13 4-4M8 16l-1 1a4 4 0 0 1-6-6l4-4a4 4 0 0 1 6 0m2 1 1-1a4 4 0 0 1 6 6l-4 4a4 4 0 0 1-6 0',
  folder: 'M3 7V5a1 1 0 0 1 1-1h5l2 3h9a1 1 0 0 1 1 1v11a1 1 0 0 1-1 1H4a1 1 0 0 1-1-1V7Z',
  window: 'M4 4h16v16H4zM4 9h16M7 6.5h.01M10 6.5h.01',
  settings:
    'M9.671 4.136a2.34 2.34 0 0 1 4.659 0 2.34 2.34 0 0 0 3.319 1.915 2.34 2.34 0 0 1 2.33 4.033 2.34 2.34 0 0 0 0 3.831 2.34 2.34 0 0 1-2.33 4.033 2.34 2.34 0 0 0-3.319 1.915 2.34 2.34 0 0 1-4.659 0 2.34 2.34 0 0 0-3.32-1.915 2.34 2.34 0 0 1-2.33-4.033 2.34 2.34 0 0 0 0-3.831A2.34 2.34 0 0 1 6.35 6.051a2.34 2.34 0 0 0 3.319-1.915M15 12a3 3 0 1 1-6 0 3 3 0 0 1 6 0Z',
  arrow: 'M5 12h14m-5-5 5 5-5 5',
  close: 'm6 6 12 12M6 18 18 6',
  refresh:
    'M3 12a9 9 0 0 1 9-9 9.75 9.75 0 0 1 6.74 2.74L21 8M21 3v5h-5M21 12a9 9 0 0 1-9 9 9.75 9.75 0 0 1-6.74-2.74L3 16M8 16H3v5',
  eye: 'M2 12s4-7 10-7 10 7 10 7-4 7-10 7S2 12 2 12ZM12 9a3 3 0 1 0 0 6 3 3 0 0 0 0-6Z',
  target: 'M12 2v3M12 19v3M2 12h3M19 12h3M12 7a5 5 0 1 0 0 10 5 5 0 0 0 0-10Z',
  expand: 'M4 9V4h5m6 0h5v5M4 15v5h5m6 0h5v-5',
  clock: 'M12 3a9 9 0 1 0 0 18 9 9 0 0 0 0-18Zm0 4v5l3 2',
  check: 'm5 12 4 4L19 6',
  info: 'M12 3a9 9 0 1 0 0 18 9 9 0 0 0 0-18Zm0 8v5m0-9h.01',
  power: 'M12 2v10M6 5a9 9 0 1 0 12 0',
  download: 'M12 3v12m0 0 4-4m-4 4-4-4M4 20h16',
  more: 'M5 12h.01M12 12h.01M19 12h.01',
  note: 'M5 3h14v13l-5 5H5V3Zm9 18v-5h5M8 7h8M8 11h6',
  checklist: 'm3 6 1 1 2-2m-3 8 1 1 2-2M10 6h11M10 13h11M3 20h3m4 0h11',
  trash: 'M3 6h18M9 6V3h6v3M5 6l1 15h12l1-15M10 10v7m4-7v7',
  chevron: 'm8 10 4 4 4-4',
  minimize: 'M5 12h14',
  maximize: 'M5 5h14v14H5z',
  restore: 'M9 8h11v11H9zM15 8V4H4v11h5',
};
</script>

<template>
  <svg
    :width="size"
    :height="size"
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    stroke-width="1.7"
    stroke-linecap="round"
    stroke-linejoin="round"
    aria-hidden="true"
  >
    <path :d="paths[name] || paths.window" />
  </svg>
</template>
