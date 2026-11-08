// 功能：管理即时窗口字段的编辑、顺序提交及失败草稿，不以实时回传触发重复提交。
import { computed, reactive, ref, watch } from 'vue';
import { MAX_HEIGHT, MAX_WIDTH, MIN_HEIGHT, MIN_WIDTH } from './geometryLimits';
import { windowValues } from './usePinSettingsForm';
import type { PinConfig, WindowPatch, WindowValues } from '../../types';

/// 只有显式用户事件触发提交；原生快照更新只回填未编辑字段。
export function useLiveWindow(
  getPin: () => PinConfig,
  apply: (patch: WindowPatch) => Promise<PinConfig | null>,
) {
  let baseline = windowValues(getPin());
  const input = reactive<WindowValues>({ ...baseline });
  const pending = ref(0);
  const failures = reactive(new Set<keyof WindowValues>());
  const failed = computed(() => failures.size > 0);
  const pendingFields = new Map<keyof WindowValues, number>();
  const versions = new Map<keyof WindowValues, number>();
  const inFlight = new Set<Promise<unknown>>();
  const keys = Object.keys(windowValues(getPin())) as (keyof WindowValues)[];
  const dirty = computed(() => {
    const actual = windowValues(getPin());
    return keys.some((key) => input[key] !== actual[key]);
  });

  // 回传只同步未编辑、未提交中的字段，避免“开→关”的第二次输入被第一次回传覆盖。
  watch(
    () => windowValues(getPin()),
    (next) => {
      for (const key of keys) {
        if (!pendingFields.get(key) && Object.is(input[key], baseline[key])) {
          Object.assign(input, { [key]: next[key] });
        }
      }
      baseline = next;
    },
  );

  /// 单字段提交不会夹带其他正在编辑的字段或过期坐标。
  function commit(key: keyof WindowValues) {
    const operation = submit(key);
    inFlight.add(operation);
    void operation.finally(() => inFlight.delete(operation));
    return operation;
  }

  /// 即时提交前校验数值类型与范围，临时空字符串不能进入 IPC。
  function valid(key: keyof WindowValues) {
    const value = input[key];
    if (key === 'width')
      return (
        typeof value === 'number' &&
        Number.isInteger(value) &&
        value >= MIN_WIDTH &&
        value <= MAX_WIDTH
      );
    if (key === 'height')
      return (
        typeof value === 'number' &&
        Number.isInteger(value) &&
        value >= MIN_HEIGHT &&
        value <= MAX_HEIGHT
      );
    if (key === 'x' || key === 'y')
      return (
        typeof value === 'number' &&
        Number.isInteger(value) &&
        value >= -2147483648 &&
        value <= 2147483647
      );
    return true;
  }

  /// 记录本次输入快照，完成后只校正未被用户继续编辑的字段。
  async function submit(key: keyof WindowValues) {
    const value = input[key];
    if (value === windowValues(getPin())[key] && !pendingFields.get(key)) return;
    if (!valid(key)) return;
    const version = (versions.get(key) ?? 0) + 1;
    versions.set(key, version);
    pendingFields.set(key, (pendingFields.get(key) ?? 0) + 1);
    pending.value++;
    failures.delete(key);
    try {
      const result = await apply({ [key]: value });
      if (!result) {
        failures.add(key);
        return;
      }
      // 操作完成前用户可能继续输入，只有仍等于本次提交值时才回填系统校正结果。
      failures.delete(key);
      if (versions.get(key) === version && Object.is(input[key], value))
        Object.assign(input, { [key]: windowValues(result)[key] });
    } catch {
      failures.add(key);
    } finally {
      pendingFields.set(key, (pendingFields.get(key) ?? 1) - 1);
      pending.value--;
    }
  }

  /// 保存或切换预设前等待已有提交，并应用尚未失焦的合法输入；失败时阻止后续动作。
  async function flush() {
    await Promise.all(inFlight);
    if (failed.value || keys.some((key) => !valid(key))) return false;
    for (const key of keys) {
      if (input[key] !== windowValues(getPin())[key]) await commit(key);
      if (failed.value) return false;
    }
    return !dirty.value;
  }

  /// 放弃未生效的输入只恢复表单，不撤销已生效的窗口操作。
  function reset() {
    Object.assign(input, windowValues(getPin()));
    failures.clear();
  }

  return reactive({ input, pending, failed, dirty, commit, reset, flush });
}
