// 功能：装配 Vue I18n、合并 ui 与 native 命名空间，并把有效语言同步到文档。
import { createI18n } from 'vue-i18n';
import enNative from '../../locales/en-US/native.json';
import enUi from '../../locales/en-US/ui.json';
import zhNative from '../../locales/zh-CN/native.json';
import zhUi from '../../locales/zh-CN/ui.json';

/// 有效语言，即当前实际使用的翻译资源；与用户偏好 `system` 不同。
export type EffectiveLocale = 'zh-CN' | 'en-US';

export const DEFAULT_LOCALE: EffectiveLocale = 'zh-CN';

/// 资源在构建期由 @intlify/unplugin-vue-i18n 预编译，运行时不内嵌消息编译器，
/// 因此管理界面 CSP 无需 `unsafe-eval`。缺键统一回退中文。
export const i18n = createI18n({
  legacy: false,
  locale: DEFAULT_LOCALE,
  fallbackLocale: DEFAULT_LOCALE,
  messages: {
    'zh-CN': { ui: zhUi, native: zhNative },
    'en-US': { ui: enUi, native: enNative },
  },
});

/// 当前语言标签，供 `Intl` 等按语言格式化的接口使用。
export function localeTag(): EffectiveLocale {
  return i18n.global.locale.value as EffectiveLocale;
}

/// 应用有效语言：翻译资源、HTML `lang` 属性和文档标题一起更新。
///
/// 只影响应用界面，不刷新、销毁或重建内容 WebView。
export function applyLocale(locale: EffectiveLocale): void {
  i18n.global.locale.value = locale;
  document.documentElement.lang = locale;
  document.title = String(i18n.global.t('native.window.managerTitle'));
}
