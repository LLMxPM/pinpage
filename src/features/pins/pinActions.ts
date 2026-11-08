// 功能：根据页面配置与运行态推导卡片操作，保证运行、可见与层级三类语义互不混用。
import type { PinConfig, PinRuntime } from '../../types';

/// 卡片主按钮只承载开启、显示或重新加载，不把“带到最前”伪装成主操作。
export interface PinPrimaryAction {
  operation: 'open' | 'focus' | 'reload';
  icon: 'power' | 'eye' | 'refresh';
  labelKey: 'ui.card.open' | 'ui.card.show' | 'ui.card.retry';
}

/// 页面正常可见时没有主操作；隐藏时优先恢复显示，加载失败才接管主按钮。
export function primaryAction(pin: PinConfig, runtime?: PinRuntime): PinPrimaryAction | null {
  if (!runtime?.open) return { operation: 'open', icon: 'power', labelKey: 'ui.card.open' };
  if (!runtime.visible) return { operation: 'focus', icon: 'eye', labelKey: 'ui.card.show' };
  if (runtime.error) {
    return { operation: 'reload', icon: 'refresh', labelKey: 'ui.card.retry' };
  }
  return null;
}

/// 桌面模式窗口嵌在桌面层，无法抬到普通窗口前面；异常页优先处理错误，也不提供置前。
export function canBringToFront(pin: PinConfig, runtime?: PinRuntime): boolean {
  return !!runtime?.open && runtime.visible && !runtime.error && pin.mode === 'floating';
}
