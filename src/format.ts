/** 相对时间：刚刚 / 5 分钟前 / 昨天 ... */
export function formatRelative(ts: number | null): string {
  if (!ts) return "从未启动";
  const diff = Date.now() - ts;
  if (diff < 0) return "刚刚";
  const sec = Math.floor(diff / 1000);
  if (sec < 45) return "刚刚";
  const min = Math.floor(sec / 60);
  if (min < 60) return `${min} 分钟前`;
  const hour = Math.floor(min / 60);
  if (hour < 24) return `${hour} 小时前`;
  const day = Math.floor(hour / 24);
  if (day === 1) return "昨天";
  if (day < 30) return `${day} 天前`;
  const month = Math.floor(day / 30);
  if (month < 12) return `${month} 个月前`;
  return `${Math.floor(month / 12)} 年前`;
}

export function formatDateTime(ts: number | null): string {
  if (!ts) return "尚未启动过";
  const d = new Date(ts);
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(
    d.getHours(),
  )}:${pad(d.getMinutes())}`;
}

/** 路径过长时中间省略 */
export function ellipsisPath(path: string, max = 68): string {
  if (path.length <= max) return path;
  const parts = path.split("\\");
  if (parts.length <= 3) return `${path.slice(0, max - 12)}…${path.slice(-11)}`;
  const head = parts.slice(0, 2).join("\\");
  const tail = parts.slice(-2).join("\\");
  return `${head}\\…\\${tail}`;
}

export function baseName(path: string): string {
  const idx = Math.max(path.lastIndexOf("\\"), path.lastIndexOf("/"));
  return idx >= 0 ? path.slice(idx + 1) : path;
}

export function dirName(path: string): string {
  const idx = Math.max(path.lastIndexOf("\\"), path.lastIndexOf("/"));
  return idx > 0 ? path.slice(0, idx) : path;
}
