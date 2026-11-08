// 功能：按用户操作顺序串行执行桌面任务，失败不阻塞后续操作，也不丢弃保存请求。

/// 返回一个串行入口；每个调用者都能获得自己任务的结果或异常。
export function createOperationQueue() {
  let tail: Promise<unknown> = Promise.resolve();
  return <T>(operation: () => Promise<T>): Promise<T> => {
    const next = tail.then(operation);
    tail = next.catch(() => undefined);
    return next;
  };
}
