// 功能：在不受信任内容窗口中检查资源可用性与管理命令拒绝行为。
document.querySelector('#module').textContent = 'JS 模块：通过';

/// 从包含中文和空格的目录相对加载 JSON，并尝试调用应被拒绝的管理命令。
async function verify() {
  try {
    const response = await fetch('./data.json');
    const data = await response.json();
    document.querySelector('#json').textContent = data.message;
  } catch (error) {
    document.querySelector('#json').textContent = `JSON：失败 ${error}`;
  }
  const output = document.querySelector('#ipc');
  if (!window.__TAURI_INTERNALS?.invoke) {
    output.textContent = '管理 IPC：未暴露（通过）';
    return;
  }
  try {
    await window.__TAURI_INTERNALS.invoke('snapshot');
    output.textContent = '管理 IPC：错误放行（失败）';
  } catch {
    output.textContent = '管理 IPC：已拒绝（通过）';
  }
}
void verify();
