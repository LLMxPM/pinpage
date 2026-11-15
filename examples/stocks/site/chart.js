// 功能：用透明 SVG 绘制真实分钟走势、昨收基线和可键盘访问的数据游标，不拉伸未发生时段。
const NS = 'http://www.w3.org/2000/svg';

/** 创建 SVG 节点，所有坐标来自已校验的数值，文本仍通过 textContent 写入。 */
function svgNode(tag, attributes = {}) {
  const element = document.createElementNS(NS, tag);
  for (const [key, value] of Object.entries(attributes)) element.setAttribute(key, String(value));
  return element;
}

/** 计算围绕昨收的对称价格范围；最小跨度避免微小变动被无限放大。 */
export function chartGeometry(trend, width = 600, height = 180) {
  const values = trend.points.map((point) => point.price);
  if (!values.length) return null;
  const center = trend.previousClose ?? (Math.max(...values) + Math.min(...values)) / 2;
  const range = Math.max(center * 0.002, ...values.map((value) => Math.abs(value - center)));
  const endMinute = Math.max(1, ...trend.points.map((point) => point.minute));
  const y = (price) => 10 + ((center + range - price) / (2 * range)) * (height - 20);
  return {
    top: center + range,
    bottom: center - range,
    baseline: trend.previousClose ? y(trend.previousClose) : null,
    endMinute,
    points: trend.points.map((point) => ({
      ...point,
      x: 3 + (point.minute / endMinute) * (width - 6),
      y: y(point.price),
    })),
  };
}

/** 生成已发生时段的刻度，午休断点标明两个时刻，避免将未来时段画成已有走势。 */
export function sessionLabel(minute) {
  if (minute === 120) return '11:30 / 13:00';
  const total = minute <= 120 ? 570 + minute : 780 + minute - 120;
  return `${String(Math.floor(total / 60)).padStart(2, '0')}:${String(total % 60).padStart(2, '0')}`;
}

/** 绘制当前标的分时；单点仅绘制圆点，缺失时保留说明，不画虚构历史。 */
export function renderChart(trend, quote, status) {
  const chart = document.querySelector('#trend-chart');
  const empty = document.querySelector('#chart-empty');
  const caption = document.querySelector('#trend-status');
  const meta = document.querySelector('#trend-meta');
  const tooltip = document.querySelector('#chart-tooltip');
  chart.replaceChildren();
  chart.onpointermove = chart.onpointerleave = chart.onkeydown = null;
  tooltip.textContent = '';
  const points = trend?.points ?? [];
  const old = trend && (Date.now() - trend.fetchedAt > 120000 || status.loading || status.error);
  caption.textContent = status.loading
    ? `正在读取分时${points.length ? ' · 缓存' : ''}`
    : status.error
      ? `${status.error}${points.length ? ' · 保留缓存' : ''}`
      : points.length === 1
        ? `仅 1 个分时点，等待后续数据${old ? ' · 缓存' : ''}`
        : points.length
          ? `${points.length} 个分时点${old ? ' · 缓存' : ''}`
          : '暂无分时数据';
  meta.textContent = trend
    ? `${trend.date.slice(0, 4)}-${trend.date.slice(4, 6)}-${trend.date.slice(6)} · 腾讯财经`
    : '腾讯财经 · 当日分钟价格';
  empty.hidden = points.length > 0;
  empty.textContent = status.loading
    ? '正在读取走势…'
    : status.error
      ? '暂时无法取得分时数据'
      : '有分时数据后在此显示';
  chart.setAttribute(
    'aria-label',
    `${quote?.name ?? '选中标的'}，${meta.textContent}，${caption.textContent}。左右方向键查看分时点。`,
  );
  document.querySelector('#chart-high').textContent = '';
  document.querySelector('#chart-low').textContent = '';
  document.querySelector('.baseline-key').hidden = !trend?.previousClose;
  const ticks = document.querySelectorAll('.chart-axis span');
  ticks.forEach((tick) => {
    tick.textContent = '';
  });
  const geometry = trend && chartGeometry(trend);
  if (!geometry) return;
  const minutes =
    points.length === 1
      ? [points[0].minute]
      : [0, Math.round(geometry.endMinute / 2), geometry.endMinute];
  ticks.forEach((tick, index) => {
    tick.textContent = minutes[index] === undefined ? '' : sessionLabel(minutes[index]);
  });
  const decimals = quote?.decimals ?? 2;
  const last = geometry.points.at(-1);
  chart.dataset.direction = trend.previousClose
    ? last.price > trend.previousClose
      ? 'up'
      : last.price < trend.previousClose
        ? 'down'
        : 'flat'
    : 'flat';
  document.querySelector('#chart-high').textContent = geometry.top.toFixed(decimals);
  document.querySelector('#chart-low').textContent = geometry.bottom.toFixed(decimals);
  const defs = svgNode('defs');
  const gradient = svgNode('linearGradient', {
    id: 'trend-fill',
    x1: '0',
    y1: '0',
    x2: '0',
    y2: '1',
  });
  gradient.append(
    svgNode('stop', { offset: '0%', 'stop-color': 'currentColor', 'stop-opacity': '.18' }),
    svgNode('stop', { offset: '100%', 'stop-color': 'currentColor', 'stop-opacity': '0' }),
  );
  defs.append(gradient);
  chart.append(defs);
  for (const y of [10, 90, 170])
    chart.append(svgNode('line', { x1: 0, y1: y, x2: 600, y2: y, class: 'chart-grid' }));
  if (geometry.baseline !== null)
    chart.append(
      svgNode('line', {
        x1: 0,
        y1: geometry.baseline,
        x2: 600,
        y2: geometry.baseline,
        class: 'chart-baseline',
      }),
    );
  const path = geometry.points
    .map((point, index) => `${index ? 'L' : 'M'}${point.x.toFixed(2)},${point.y.toFixed(2)}`)
    .join(' ');
  if (points.length > 1) {
    chart.append(
      svgNode('path', {
        d: `${path} L${last.x},180 L${geometry.points[0].x},180 Z`,
        fill: 'url(#trend-fill)',
        stroke: 'none',
      }),
    );
    chart.append(svgNode('path', { d: path, class: 'chart-line' }));
  }
  chart.append(svgNode('circle', { cx: last.x, cy: last.y, r: 2.5, class: 'chart-last' }));
  const cursor = svgNode('g', { visibility: 'hidden' });
  const vertical = svgNode('line', { y1: 0, y2: 180, class: 'chart-cursor' });
  const dot = svgNode('circle', { r: 3.5, class: 'chart-last' });
  cursor.append(vertical, dot);
  chart.append(cursor);
  let active = geometry.points.length - 1;

  /** 指向已有分钟点，游标和提示不会插值生成不存在的价格。 */
  function showPoint(index) {
    active = Math.max(0, Math.min(geometry.points.length - 1, index));
    const point = geometry.points[active];
    cursor.setAttribute('visibility', 'visible');
    vertical.setAttribute('x1', point.x);
    vertical.setAttribute('x2', point.x);
    dot.setAttribute('cx', point.x);
    dot.setAttribute('cy', point.y);
    tooltip.textContent = `${point.time.slice(0, 2)}:${point.time.slice(2)}　${point.price.toFixed(decimals)}`;
  }
  chart.onpointermove = (event) => {
    const rect = chart.getBoundingClientRect();
    const x = ((event.clientX - rect.left) / rect.width) * 600;
    const nearest = geometry.points.reduce(
      (best, point, index, all) =>
        Math.abs(point.x - x) < Math.abs(all[best].x - x) ? index : best,
      0,
    );
    showPoint(nearest);
  };
  chart.onpointerleave = () => {
    cursor.setAttribute('visibility', 'hidden');
    tooltip.textContent = '';
  };
  chart.onkeydown = (event) => {
    if (!['ArrowLeft', 'ArrowRight', 'Home', 'End'].includes(event.key)) return;
    event.preventDefault();
    showPoint(
      event.key === 'Home'
        ? 0
        : event.key === 'End'
          ? geometry.points.length - 1
          : active + (event.key === 'ArrowRight' ? 1 : -1),
    );
  };
}
