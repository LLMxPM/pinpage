// 功能：在本地自选中按名称或代码筛选、按涨跌排序，不新增网络搜索请求。
/** 排序仅影响展示，保留用户原有自选顺序；缺失报价始终置后。 */
export function visibleSymbols(state) {
  const quotes = new Map(state.quotes.map((quote) => [quote.symbol, quote]));
  const query = (state.filter || '').trim().toLocaleLowerCase();
  const symbols = state.symbols.filter((symbol) =>
    `${symbol} ${quotes.get(symbol)?.name || ''}`.toLocaleLowerCase().includes(query),
  );
  if (state.sort === 'name')
    symbols.sort((a, b) =>
      (quotes.get(a)?.name || a).localeCompare(quotes.get(b)?.name || b, 'zh-CN', {
        numeric: true,
      }),
    );
  if (state.sort === 'percent')
    symbols.sort((a, b) => {
      const left = quotes.get(a),
        right = quotes.get(b);
      const validLeft = left?.price != null && Number.isFinite(left?.percent);
      const validRight = right?.price != null && Number.isFinite(right?.percent);
      return validLeft && validRight
        ? right.percent - left.percent
        : Number(validRight) - Number(validLeft);
    });
  return symbols;
}
