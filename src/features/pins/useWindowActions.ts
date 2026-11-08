// 功能：协调窗口即时修改、预设管理和仅作用于窗口参数的撤销。
import { ref, type Ref } from 'vue';
import * as desktop from '../../services/desktop';
import { message } from '../../i18n/messages';
import { presetLabel } from './presets';
import { windowValues } from './usePinSettingsForm';
import { isAppMessage } from '../../types/messages';
import type {
  PinConfig,
  PresetInput,
  Snapshot,
  UiMessage,
  WindowPatch,
  WindowValues,
} from '../../types';

export type RunOperation = (operation: () => Promise<void>, key?: string) => Promise<boolean>;

/// 桌面任务由工作台统一排队，预设与手动设置使用同一份实际状态。
export function useWindowActions(
  snapshot: Ref<Snapshot>,
  run: RunOperation,
  notify: (entry: UiMessage, undo?: (() => Promise<void>) | null) => void,
) {
  const undoState = ref<{ id: string; before: WindowPatch; after: WindowPatch } | null>(null);
  const find = (id: string) => snapshot.value.settings.pins.find((pin) => pin.id === id);

  /// 只记录此次修改涉及的字段，避免撤销一个开关时覆盖后续移动的位置。
  function remember(
    id: string,
    before: WindowValues,
    after: WindowValues,
    keys: (keyof WindowValues)[],
  ) {
    undoState.value = {
      id,
      before: Object.fromEntries(keys.map((key) => [key, before[key]])),
      after: Object.fromEntries(keys.map((key) => [key, after[key]])),
    };
  }

  /// 局部修改成功才替换撤销记录；失败保留上一次已成功操作。
  async function applyWindow(id: string, patch: WindowPatch): Promise<PinConfig | null> {
    let result: PinConfig | null = null;
    const saved = await run(async () => {
      const pin = find(id);
      if (!pin) throw { code: 'errors.pinNotFound', args: {} };
      const change = await desktop.updateWindow(id, patch);
      result = change.after;
      remember(
        id,
        windowValues(change.before),
        windowValues(result),
        Object.keys(patch) as (keyof WindowValues)[],
      );
    }, `${id}:window`);
    return saved ? result : null;
  }

  /// 已被外部操作改动的字段不执行过期撤销，防止覆盖窗口的新状态。
  async function undoWindow(id: string) {
    return run(async () => {
      const entry = undoState.value;
      const pin = find(id);
      if (!entry || entry.id !== id) return;
      if (!pin) throw { code: 'errors.pinNotFound', args: {} };
      try {
        await desktop.updateWindow(id, entry.before, entry.after);
      } catch (reason) {
        if (isAppMessage(reason) && reason.code === 'errors.windowUndoExpired')
          undoState.value = null;
        throw reason;
      }
      undoState.value = null;
    }, `${id}:window:undo`);
  }

  /// 预设切换一次提交整套窗口参数；它不进入单字段撤销，避免模式或锁定变化后旧快照失效。
  async function applyPreset(id: string, presetId: string) {
    const preset = snapshot.value.presets.find((entry) => entry.id === presetId);
    if (!preset) return false;
    const applied = await run(async () => {
      if (!find(id)) throw { code: 'errors.pinNotFound', args: {} };
      await desktop.pinAction(id, `preset:${presetId}`);
    }, `${id}:preset:${presetId}`);
    if (applied) {
      // 预设覆盖整组窗口字段，先前针对该页的单字段撤销不再适用。
      if (undoState.value?.id === id) undoState.value = null;
    }
    return applied;
  }

  /// 另存或覆盖已生效的窗口配置，等待成功后才反馈。
  async function savePreset(id: string, name: string, presetId?: string) {
    const saved = await run(async () => {
      await desktop.savePreset(id, name, presetId);
    }, `${id}:preset:save`);
    if (saved)
      notify(message(presetId ? 'ui.toast.presetUpdated' : 'ui.toast.presetSaved', { name }));
    return saved;
  }

  /// 管理器只编辑模板；已有页面不会自动跟随模板变化。
  async function writePreset(input: PresetInput, presetId?: string) {
    const saved = await run(async () => {
      await desktop.writePreset(input, presetId);
    }, 'preset:write');
    if (saved)
      notify(
        message(presetId ? 'ui.toast.presetUpdated' : 'ui.toast.presetSaved', { name: input.name }),
      );
    return saved;
  }

  /// 删除模板后的页面继续保留当前状态。
  async function deletePreset(presetId: string) {
    const preset = snapshot.value.presets.find((entry) => entry.id === presetId);
    const removed = await run(() => desktop.deletePreset(presetId), 'preset:delete');
    if (removed && preset) notify(message('ui.toast.presetDeleted', { name: presetLabel(preset) }));
    return removed;
  }

  return { applyWindow, applyPreset, savePreset, writePreset, deletePreset, undoWindow, undoState };
}
