// 功能：装配国际化、挂载 PinPage 管理界面与全局样式。
import { createApp } from 'vue';
import App from './App.vue';
import { applyLocale, i18n } from './i18n';
import './styles/base.css';
import './styles/select.css';
import './styles/feedback.css';
import './styles/dialogs.css';
import './styles/examples.css';
import './styles/launcher.css';
import './styles/pins.css';
import './styles/pin-settings.css';
import './styles/presets.css';
import './styles/workspace.css';

// 浏览器预览没有 Rust 快照，先按默认中文呈现；连接桌面后由快照给出有效语言。
applyLocale('zh-CN');
createApp(App).use(i18n).mount('#app');
