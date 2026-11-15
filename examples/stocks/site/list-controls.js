// 功能：绑定行情本地筛选、排序和涨跌配色，恢复展示偏好且不改变上游数据。
/** 只保存排序与配色，临时搜索词不会随页面包携带。 */
export function initializeListControls(state, render, persist) {
  const sort = document.querySelector('#list-sort');
  const colors = document.querySelector('#colors');
  sort.value = state.sort;
  colors.value = state.colors;
  document.documentElement.dataset.colors = state.colors;
  document.querySelector('#quote-filter').addEventListener('input', (event) => {
    state.filter = event.target.value.slice(0, 80);
    render();
  });
  sort.addEventListener('change', () => {
    state.sort = sort.value;
    render();
    persist();
  });
  colors.addEventListener('change', () => {
    state.colors = colors.value;
    document.documentElement.dataset.colors = state.colors;
    persist();
  });
}
