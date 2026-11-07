// 功能：维护工作台示例库的稳定标识与名称键；标识与 Rust 枚举一致。
export type { ExampleId } from '../../types';

export const examples = [
  { id: 'timer', title: 'ui.examples.timer' },
  { id: 'notes', title: 'ui.examples.notes' },
  { id: 'checklist', title: 'ui.examples.checklist' },
  { id: 'clock', title: 'ui.examples.clock' },
] as const;
