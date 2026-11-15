// 功能：维护工作台示例库的稳定标识与名称键；标识与 Rust 枚举一致。
export type { ExampleId } from '../../types';

export const examples = [
  { id: 'timer', title: 'ui.examples.timer', badge: 'offline' },
  { id: 'notes', title: 'ui.examples.notes', badge: 'offline' },
  { id: 'calendar', title: 'ui.examples.calendar', badge: 'offline' },
  { id: 'kanban', title: 'ui.examples.kanban', badge: 'offline' },
  { id: 'stocks', title: 'ui.examples.stocks', badge: 'network' },
  { id: 'folder', title: 'ui.examples.folder', badge: 'permission' },
  { id: 'systemMonitor', title: 'ui.examples.systemMonitor', badge: 'host' },
] as const;
