// 功能：导出时采集页面 Web 存储，导入时通过应用内引导页回填，避开页面脚本竞争。
use tauri::AppHandle;

/// 引导页与随包数据的固定路径，只对所属页面的 WebView 开放。
pub const RESTORE_PAGE: &str = "/__pinpage__/restore.html";
pub const RESTORE_DATA: &str = "/__pinpage__/data.json";

/// 单个数据库最多采集的记录数，超出后截断并在导出结果中说明。
const MAX_RECORDS: usize = 3000;

/// 采集脚本模板，用占位符避免在 JS 中转义大量大括号。
const COLLECT_TEMPLATE: &str = r#"(function () {
  const id = __ID__;
  const MAX_RECORDS = __MAX_RECORDS__;

  const toBase64 = (buffer) => {
    const bytes = new Uint8Array(buffer);
    let text = '';
    const chunk = 0x8000;
    for (let index = 0; index < bytes.length; index += chunk) {
      text += String.fromCharCode.apply(null, bytes.subarray(index, index + chunk));
    }
    return btoa(text);
  };

  const encode = async (value, depth) => {
    if (depth > 12) return { $t: 'unsupported', v: 'nested' };
    if (value === null) return null;
    const type = typeof value;
    if (type === 'string' || type === 'boolean') return value;
    if (type === 'number') {
      return Number.isFinite(value) ? value : { $t: 'number', v: String(value) };
    }
    if (type === 'bigint') return { $t: 'bigint', v: value.toString() };
    if (type === 'undefined') return { $t: 'undefined' };
    if (type !== 'object') return { $t: 'unsupported', v: type };
    if (value instanceof Date) return { $t: 'date', v: value.getTime() };
    if (value instanceof RegExp) return { $t: 'regexp', v: value.source, f: value.flags };
    if (value instanceof ArrayBuffer) return { $t: 'buffer', v: toBase64(value) };
    if (ArrayBuffer.isView(value)) {
      const view = new Uint8Array(value.buffer, value.byteOffset, value.byteLength);
      return { $t: 'typed', c: value.constructor.name, v: toBase64(view) };
    }
    if (typeof Blob !== 'undefined' && value instanceof Blob) {
      return { $t: 'blob', m: value.type || '', v: toBase64(await value.arrayBuffer()) };
    }
    if (Array.isArray(value)) {
      const items = [];
      for (const item of value) items.push(await encode(item, depth + 1));
      return items;
    }
    // 只递归普通对象；Map、Set 等结构化类型标记为不支持，避免被静默拍平。
    const proto = Object.getPrototypeOf(value);
    if (proto !== Object.prototype && proto !== null) {
      return { $t: 'unsupported', v: 'object' };
    }
    // 普通对象统一包一层标记，避免用户数据里的 $t 字段被误判成特殊类型。
    const fields = {};
    for (const key of Object.keys(value)) fields[key] = await encode(value[key], depth + 1);
    return { $t: 'obj', v: fields };
  };

  const dump = (store) => {
    const result = {};
    try {
      for (let index = 0; index < store.length; index += 1) {
        const key = store.key(index);
        if (key !== null) result[key] = store.getItem(key);
      }
    } catch (error) {
      return { __error: String(error) };
    }
    return result;
  };

  const readStore = (db, storeName) =>
    new Promise((resolve, reject) => {
      let transaction;
      try {
        transaction = db.transaction(storeName, 'readonly');
      } catch (error) {
        return reject(error);
      }
      const store = transaction.objectStore(storeName);
      const entry = {
        name: storeName,
        keyPath: store.keyPath === undefined ? null : store.keyPath,
        autoIncrement: Boolean(store.autoIncrement),
        indexes: [],
        records: [],
        truncated: false
      };
      for (const indexName of Array.from(store.indexNames)) {
        const index = store.index(indexName);
        entry.indexes.push({
          name: index.name,
          keyPath: index.keyPath,
          unique: Boolean(index.unique),
          multiEntry: Boolean(index.multiEntry)
        });
      }
      const request = store.openCursor();
      request.onsuccess = () => {
        const cursor = request.result;
        if (!cursor) return;
        if (entry.records.length >= MAX_RECORDS) {
          entry.truncated = true;
          return;
        }
        entry.records.push({ key: cursor.primaryKey, value: cursor.value });
        cursor.continue();
      };
      request.onerror = () => reject(request.error);
      transaction.oncomplete = () => resolve(entry);
      transaction.onerror = () => reject(transaction.error);
      transaction.onabort = () => reject(transaction.error || new Error('aborted'));
    });

  const collectDatabase = async (name) => {
    const db = await new Promise((resolve, reject) => {
      const request = indexedDB.open(name);
      request.onsuccess = () => resolve(request.result);
      request.onerror = () => reject(request.error);
      // 已存在的库不应触发升级；一旦触发说明状态异常，放弃本次采集。
      request.onupgradeneeded = () => {
        try {
          request.transaction.abort();
        } catch (error) {}
        reject(new Error('missing'));
      };
    });
    const info = { name: db.name, version: db.version, stores: [] };
    for (const storeName of Array.from(db.objectStoreNames)) {
      const store = await readStore(db, storeName);
      const records = [];
      for (const record of store.records) {
        records.push({
          key: await encode(record.key, 0),
          value: await encode(record.value, 0)
        });
      }
      info.stores.push({
        name: store.name,
        keyPath: store.keyPath,
        autoIncrement: store.autoIncrement,
        indexes: store.indexes,
        records,
        truncated: store.truncated
      });
    }
    db.close();
    return info;
  };

  const collectDatabases = async () => {
    if (typeof indexedDB === 'undefined' || typeof indexedDB.databases !== 'function') {
      return [];
    }
    const databases = [];
    try {
      const list = await indexedDB.databases();
      for (const item of list) {
        if (!item || !item.name) continue;
        try {
          databases.push(await collectDatabase(item.name));
        } catch (error) {
          databases.push({ name: item.name, error: String(error), stores: [] });
        }
      }
    } catch (error) {
      return [];
    }
    return databases;
  };

  collectDatabases()
    .then((databases) => ({
      localStorage: dump(window.localStorage),
      sessionStorage: dump(window.sessionStorage),
      indexedDB: databases
    }))
    .then((payload) => {
      const bridge = window.__TAURI_INTERNALS__;
      if (!bridge || typeof bridge.invoke !== 'function') return;
      return bridge.invoke('submit_page_data', {
        id: id,
        payload: JSON.stringify(payload)
      });
    })
    .catch(() => {});
})();"#;

/// 生成采集脚本：读取 localStorage、sessionStorage 与 IndexedDB 后回传。
///
/// 脚本在页面自身来源下执行，因此只能访问该页面 ID 的存储；回传命令只在
/// 导出会话期间接受调用，页面平时无法主动上报数据。
pub fn collect_script(id: &str) -> String {
    COLLECT_TEMPLATE
        .replace("__ID__", &format!("{id:?}"))
        .replace("__MAX_RECORDS__", &MAX_RECORDS.to_string())
}

/// 读取并删除随包数据，保证同一次导入只恢复一次。
pub fn take_pending(app: &AppHandle, id: &str) -> Result<(String, Vec<u8>), String> {
    let path = crate::import::pages_root(app)?
        .join(id)
        .join(crate::import::PENDING_DATA);
    let bytes = std::fs::read(&path).map_err(|_| "没有待恢复的页面数据")?;
    let _ = std::fs::remove_file(&path);
    Ok(("application/json".into(), bytes))
}

/// 引导页：在没有页面脚本竞争的同源空白页里完成回填，再跳转到真正入口。
pub fn restore_page() -> &'static str {
    r#"<!doctype html>
<html lang="zh-CN">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>正在恢复页面数据</title>
    <style>
      html, body { height: 100%; margin: 0; }
      body {
        display: grid; place-items: center; background: #fbfbfc; color: #8b96a4;
        font: 13px/1.6 system-ui, "Microsoft YaHei", sans-serif;
      }
      .hint { text-align: center; }
      .bar { width: 96px; height: 2px; margin: 10px auto 0; background: #e6eaef; border-radius: 2px; overflow: hidden; }
      .bar::after { content: ""; display: block; width: 40%; height: 100%; background: #d89578; animation: slide 1s ease-in-out infinite; }
      @keyframes slide { from { transform: translateX(-100%); } to { transform: translateX(250%); } }
    </style>
  </head>
  <body>
    <div class="hint">正在恢复页面数据…<div class="bar"></div></div>
    <script>
      (async () => {
        const target = '/' + (new URLSearchParams(location.search).get('entry') || 'index.html');
        const toBuffer = (text) => {
          const binary = atob(text);
          const bytes = new Uint8Array(binary.length);
          for (let index = 0; index < binary.length; index += 1) bytes[index] = binary.charCodeAt(index);
          return bytes.buffer;
        };
        const decode = (value) => {
          if (value === null || typeof value !== 'object') return value;
          if (Array.isArray(value)) return value.map(decode);
          const tag = value.$t;
          if (tag === 'date') return new Date(value.v);
          if (tag === 'number') return Number(value.v);
          if (tag === 'bigint') return BigInt(value.v);
          if (tag === 'undefined') return undefined;
          if (tag === 'buffer') return toBuffer(value.v);
          if (tag === 'typed') return new (window[value.c] || Uint8Array)(toBuffer(value.v));
          if (tag === 'blob') return new Blob([toBuffer(value.v)], { type: value.m || '' });
          if (tag === 'regexp') return new RegExp(value.v, value.f);
          if (tag === 'unsupported') return undefined;
          if (tag === 'obj') {
            const result = {};
            for (const key of Object.keys(value.v || {})) result[key] = decode(value.v[key]);
            return result;
          }
          const result = {};
          for (const key of Object.keys(value)) result[key] = decode(value[key]);
          return result;
        };
        const restoreDatabase = async (info) => {
          if (!info || !info.name || !info.version) return;
          const db = await new Promise((resolve, reject) => {
            const request = indexedDB.open(info.name, info.version);
            request.onupgradeneeded = () => {
              const handle = request.result;
              for (const store of info.stores || []) {
                const target = handle.objectStoreNames.contains(store.name)
                  ? request.transaction.objectStore(store.name)
                  : handle.createObjectStore(store.name, {
                      keyPath: store.keyPath === undefined ? null : store.keyPath,
                      autoIncrement: Boolean(store.autoIncrement)
                    });
                for (const index of store.indexes || []) {
                  if (target.indexNames.contains(index.name)) continue;
                  try {
                    target.createIndex(index.name, index.keyPath, {
                      unique: Boolean(index.unique),
                      multiEntry: Boolean(index.multiEntry)
                    });
                  } catch (error) {}
                }
              }
            };
            request.onsuccess = () => resolve(request.result);
            request.onerror = () => reject(request.error);
            request.onblocked = () => reject(new Error('blocked'));
          });
          for (const store of info.stores || []) {
            if (!store.records || !store.records.length) continue;
            const transaction = db.transaction(store.name, 'readwrite');
            const handle = transaction.objectStore(store.name);
            for (const record of store.records) {
              if (store.keyPath === null || store.keyPath === undefined) {
                handle.put(decode(record.value), decode(record.key));
              } else {
                handle.put(decode(record.value));
              }
            }
            await new Promise((resolve, reject) => {
              transaction.oncomplete = () => resolve();
              transaction.onerror = () => reject(transaction.error);
              transaction.onabort = () => reject(transaction.error);
            });
          }
          db.close();
        };
        try {
          const response = await fetch('/__pinpage__/data.json', { cache: 'no-store' });
          if (response.ok) {
            const info = await response.json();
            const payload = info.payload || {};
            for (const [key, value] of Object.entries(payload.localStorage || {})) {
              if (typeof value === 'string') localStorage.setItem(key, value);
            }
            for (const [key, value] of Object.entries(payload.sessionStorage || {})) {
              if (typeof value === 'string') sessionStorage.setItem(key, value);
            }
            for (const database of payload.indexedDB || []) {
              try {
                await restoreDatabase(database);
              } catch (error) {}
            }
          }
        } catch (error) {
        } finally {
          location.replace(target);
        }
      })();
    </script>
  </body>
</html>"#
}

#[cfg(test)]
#[path = "data_tests.rs"]
mod tests;
