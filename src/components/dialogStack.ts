// 功能：协调嵌套弹窗的可交互层级，管理预设时暂停后面的设置弹窗。
const dialogs: HTMLElement[] = [];

/// 新弹窗打开时屏蔽上一层，关闭时恢复上一层，不让键盘进入背景表单。
export function pushDialog(dialog: HTMLElement) {
  const previous = dialogs.at(-1);
  if (previous) previous.inert = true;
  dialogs.push(dialog);
  return () => {
    const index = dialogs.indexOf(dialog);
    if (index >= 0) dialogs.splice(index, 1);
    const active = dialogs.at(-1);
    if (active) active.inert = false;
  };
}

/// 只有最上层弹窗可以接管焦点，后台快照刷新不会抢走前层输入。
export function isActiveDialog(dialog: HTMLElement) {
  return dialogs.at(-1) === dialog;
}
