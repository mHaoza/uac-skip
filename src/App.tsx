import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { api, pickExecutables } from "./api";
import type {
  AppPatch,
  AppView,
  LaunchMode,
  StoreInfo,
  ToastAction,
  ToastItem,
  ToastKind,
} from "./types";
import { AppCard } from "./components/AppCard";
import { EditDialog } from "./components/EditDialog";
import { ConfirmDialog } from "./components/ConfirmDialog";
import { Toasts } from "./components/Toasts";
import {
  BoltIcon,
  CloseIcon,
  FolderIcon,
  PlusIcon,
  RefreshIcon,
  SearchIcon,
  ShieldIcon,
  WarningIcon,
} from "./components/Icons";
import { baseName } from "./format";

type FilterKey = "all" | "enabled" | "pending" | "normal";

const FILTERS: { key: FilterKey; label: string }[] = [
  { key: "all", label: "全部" },
  { key: "enabled", label: "已启用" },
  { key: "pending", label: "待授权" },
  { key: "normal", label: "普通权限" },
];

const EXECUTABLE_EXT = [".exe", ".bat", ".cmd"];

function isExecutablePath(path: string): boolean {
  const lower = path.toLowerCase();
  return EXECUTABLE_EXT.some((ext) => lower.endsWith(ext));
}

async function copyText(text: string): Promise<boolean> {
  try {
    await navigator.clipboard.writeText(text);
    return true;
  } catch {
    try {
      const area = document.createElement("textarea");
      area.value = text;
      area.style.position = "fixed";
      area.style.opacity = "0";
      document.body.appendChild(area);
      area.select();
      const ok = document.execCommand("copy");
      document.body.removeChild(area);
      return ok;
    } catch {
      return false;
    }
  }
}

export default function App() {
  const [apps, setApps] = useState<AppView[]>([]);
  const [icons, setIcons] = useState<Record<string, string | null>>({});
  const [elevated, setElevated] = useState(false);
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState<string | null>(null);
  const [query, setQuery] = useState("");
  const [filter, setFilter] = useState<FilterKey>("all");
  const [editing, setEditing] = useState<AppView | null>(null);
  const [removing, setRemoving] = useState<AppView | null>(null);
  const [dragging, setDragging] = useState(false);
  const [toasts, setToasts] = useState<ToastItem[]>([]);
  const [store, setStore] = useState<StoreInfo | null>(null);

  const toastSeq = useRef(0);
  const searchRef = useRef<HTMLInputElement | null>(null);

  /* ---------------- Toast ---------------- */
  const dismissToast = useCallback((id: number) => {
    setToasts((prev) => prev.filter((item) => item.id !== id));
  }, []);

  const pushToast = useCallback(
    (kind: ToastKind, message: string, action?: ToastAction) => {
      const id = (toastSeq.current += 1);
      setToasts((prev) => [...prev.slice(-3), { id, kind, message, action }]);
      const ttl = kind === "error" ? 7000 : 4200;
      window.setTimeout(() => {
        setToasts((prev) => prev.filter((item) => item.id !== id));
      }, ttl);
    },
    [],
  );

  /* ---------------- 数据加载 ---------------- */
  const sortApps = (list: AppView[]) =>
    [...list].sort((a, b) => b.createdAt - a.createdAt);

  const refresh = useCallback(async () => {
    const list = await api.listApps();
    setApps(sortApps(list));
  }, []);

  useEffect(() => {
    let alive = true;
    (async () => {
      try {
        const [isAdmin, list] = await Promise.all([api.isElevated(), api.listApps()]);
        if (!alive) return;
        setElevated(isAdmin);
        setApps(sortApps(list));
      } catch (error) {
        pushToast("error", `加载启动项失败：${String(error)}`);
      } finally {
        if (alive) setLoading(false);
      }
      api
        .storeInfo()
        .then((info) => {
          if (alive) setStore(info);
        })
        .catch(() => undefined);
    })();
    return () => {
      alive = false;
    };
  }, [pushToast]);

  /* ---------------- 图标 ---------------- */
  useEffect(() => {
    const missing = apps.filter((app) => !(app.path in icons));
    if (missing.length === 0) return;
    let alive = true;
    (async () => {
      for (const app of missing) {
        let data: string | null = null;
        try {
          data = await api.appIcon(app.path);
        } catch {
          data = null;
        }
        if (!alive) return;
        setIcons((prev) => ({ ...prev, [app.path]: data }));
      }
    })();
    return () => {
      alive = false;
    };
  }, [apps, icons]);

  /* ---------------- 免 UAC 注册 ---------------- */
  const ensureFor = useCallback(
    async (ids: string[], successText = "已启用免 UAC 启动") => {
      if (ids.length === 0) return;
      try {
        const results = await api.ensureTasks(ids);
        const okCount = results.filter((item) => item.ok).length;
        const cancelled = results.some((item) => item.cancelled);
        const failed = results.filter((item) => !item.ok && !item.cancelled);
        if (okCount > 0) {
          pushToast("success", `${successText}（${okCount} 项）`);
        }
        if (cancelled) {
          pushToast("info", "已取消管理员授权，该项仍保持「待授权」状态");
        }
        failed.forEach((item) =>
          pushToast("error", item.message ?? "注册免 UAC 启动项失败"),
        );
        await refresh();
      } catch (error) {
        pushToast("error", `操作失败：${String(error)}`);
      }
    },
    [pushToast, refresh],
  );

  /* ---------------- 添加 ---------------- */
  const addPaths = useCallback(
    async (paths: string[]) => {
      if (paths.length === 0) return;
      setBusy("add");
      try {
        const outcomes = await api.addApps(paths);
        outcomes
          .filter((item) => !item.ok)
          .forEach((item) =>
            pushToast("error", `${baseName(item.path)}：${item.message ?? "添加失败"}`),
          );
        const added = outcomes
          .filter((item) => item.ok && item.id)
          .map((item) => item.id as string);
        await refresh();
        if (added.length > 0) {
          pushToast("success", `已添加 ${added.length} 个程序`);
          await ensureFor(added);
        }
      } catch (error) {
        pushToast("error", `添加失败：${String(error)}`);
      } finally {
        setBusy(null);
      }
    },
    [ensureFor, pushToast, refresh],
  );

  const handleAdd = useCallback(async () => {
    if (busy === "add") return;
    try {
      const paths = await pickExecutables();
      if (paths.length > 0) await addPaths(paths);
    } catch (error) {
      pushToast("error", `打开文件选择框失败：${String(error)}`);
    }
  }, [addPaths, busy, pushToast]);

  /* ---------------- 拖放 ---------------- */
  useEffect(() => {
    let unlisten: (() => void) | undefined;
    let disposed = false;
    getCurrentWebview()
      .onDragDropEvent((event) => {
        const payload = event.payload;
        if (payload.type === "enter" || payload.type === "over") {
          setDragging(true);
        } else if (payload.type === "leave") {
          setDragging(false);
        } else if (payload.type === "drop") {
          setDragging(false);
          const files = (payload.paths ?? []).filter(isExecutablePath);
          if (files.length > 0) {
            void addPaths(files);
          } else {
            pushToast("error", "请拖入 .exe / .bat / .cmd 可执行文件");
          }
        }
      })
      .then((fn) => {
        if (disposed) fn();
        else unlisten = fn;
      })
      .catch(() => undefined);
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, [addPaths, pushToast]);

  /* ---------------- 快捷键 ---------------- */
  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "f") {
        event.preventDefault();
        searchRef.current?.focus();
      }
      if (event.key === "Escape") {
        setEditing(null);
        setRemoving(null);
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, []);

  /* ---------------- 启动 ---------------- */
  const handleSetMode = useCallback(
    async (app: AppView, mode: LaunchMode) => {
      if (mode === app.mode) return;
      setBusy(app.id);
      try {
        await api.updateApp(app.id, { mode });
        await refresh();
        await ensureFor(
          [app.id],
          mode === "highest" ? "已改为免 UAC 启动" : "已改为普通权限启动",
        );
      } catch (error) {
        pushToast("error", `切换启动方式失败：${String(error)}`);
      } finally {
        setBusy(null);
      }
    },
    [ensureFor, pushToast, refresh],
  );

  const handleLaunch = useCallback(
    async (app: AppView) => {
      setBusy(app.id);
      try {
        const result = await api.launchApp(app.id);
        if (result.ok) {
          pushToast("success", `已启动 ${app.name}`);
        } else {
          pushToast(
            "error",
            result.message ?? "启动失败",
            result.suggestElevate
              ? { label: "改为免 UAC 启动", run: () => handleSetMode(app, "highest") }
              : undefined,
          );
        }
        await refresh();
      } catch (error) {
        pushToast("error", `启动失败：${String(error)}`);
      } finally {
        setBusy(null);
      }
    },
    [handleSetMode, pushToast, refresh],
  );

  const handleSaveEdit = useCallback(
    async (patch: AppPatch) => {
      if (!editing) return;
      const { id, mode } = editing;
      setEditing(null);
      try {
        await api.updateApp(id, patch);
        await refresh();
        if (patch.mode && patch.mode !== mode) {
          await ensureFor([id], "已同步新的启动方式");
        } else if (patch.mode === "highest" || mode === "highest") {
          pushToast("info", "配置已更新，点击「重新授权」即可同步到系统");
        }
      } catch (error) {
        pushToast("error", `保存失败：${String(error)}`);
      }
    },
    [editing, ensureFor, pushToast, refresh],
  );

  const handleRemove = useCallback(
    async (deleteTask: boolean) => {
      if (!removing) return;
      const target = removing;
      try {
        await api.removeApp(target.id, deleteTask);
        setRemoving(null);
        await refresh();
        pushToast("success", `已移除 ${target.name}`);
      } catch (error) {
        pushToast("error", `移除失败：${String(error)}`);
      }
    },
    [pushToast, refresh, removing],
  );

  const handleRelaunch = useCallback(async () => {
    try {
      await api.relaunchElevated();
    } catch (error) {
      pushToast("error", `提权重启失败：${String(error)}`);
    }
  }, [pushToast]);

  const handleReveal = useCallback(
    async (app: AppView) => {
      try {
        await api.revealInExplorer(app.path);
      } catch (error) {
        pushToast("error", `无法打开文件位置：${String(error)}`);
      }
    },
    [pushToast],
  );

  const handleCopyPath = useCallback(
    async (app: AppView) => {
      const ok = await copyText(app.path);
      pushToast(ok ? "success" : "error", ok ? "路径已复制" : "复制失败");
    },
    [pushToast],
  );

  const handleCreateShortcut = useCallback(
    async (app: AppView) => {
      try {
        const linkPath = await api.createDesktopShortcut(app.id);
        pushToast("success", `已在桌面创建快捷方式：${baseName(linkPath)}`);
      } catch (error) {
        pushToast("error", `创建快捷方式失败：${String(error)}`);
      }
    },
    [pushToast],
  );

  /* ---------------- 派生数据 ---------------- */
  const counts = useMemo(
    () => ({
      all: apps.length,
      enabled: apps.filter((app) => app.enabled).length,
      pending: apps.filter((app) => app.mode === "highest" && !app.enabled).length,
      normal: apps.filter((app) => app.mode === "normal").length,
    }),
    [apps],
  );

  const visible = useMemo(() => {
    const keyword = query.trim().toLowerCase();
    return apps.filter((app) => {
      if (filter === "enabled" && !app.enabled) return false;
      if (filter === "pending" && !(app.mode === "highest" && !app.enabled)) return false;
      if (filter === "normal" && app.mode !== "normal") return false;
      if (!keyword) return true;
      return (
        app.name.toLowerCase().includes(keyword) ||
        app.path.toLowerCase().includes(keyword)
      );
    });
  }, [apps, filter, query]);

  const pendingIds = useMemo(
    () => apps.filter((app) => app.mode === "highest" && !app.enabled).map((app) => app.id),
    [apps],
  );

  const busyAdd = busy === "add";

  return (
    <div className="app">
      <header className="topbar">
        <div className="brand">
          <div className="brand-mark">
            <ShieldIcon size={22} />
          </div>
          <div className="brand-text">
            <h1>
              UACSkip
              <span className={`pill ${elevated ? "admin" : "user"}`}>
                {elevated ? "管理员模式" : "普通权限"}
              </span>
            </h1>
            <p>免 UAC 弹窗的应用启动管理器 · 共 {counts.all} 个启动项</p>
          </div>
        </div>
        <div className="spacer" />
        <button
          className="btn ghost icon"
          onClick={() => void refresh()}
          title="刷新状态"
          disabled={loading}
        >
          <RefreshIcon size={15} />
        </button>
        {!elevated ? (
          <button className="btn" onClick={() => void handleRelaunch()}>
            <ShieldIcon size={14} />
            以管理员身份重启
          </button>
        ) : null}
        <button className="btn primary" onClick={() => void handleAdd()} disabled={busyAdd}>
          <PlusIcon size={15} />
          {busyAdd ? "添加中…" : "添加应用"}
        </button>
      </header>

      {!elevated && apps.length > 0 ? (
        <div className="notice">
          <WarningIcon size={16} />
          <span>
            当前以普通权限运行：启用或修改免 UAC 启动项时，系统会弹出一次管理员授权窗口；
            授权完成后，这些程序的启动不再弹窗。
          </span>
        </div>
      ) : null}

      <div className="toolbar">
        <div className="search">
          <SearchIcon size={14} />
          <input
            ref={searchRef}
            value={query}
            onChange={(event) => setQuery(event.target.value)}
            placeholder="搜索名称或路径…  (Ctrl+F)"
            spellCheck={false}
          />
          {query ? (
            <button className="clear" onClick={() => setQuery("")} title="清空">
              <CloseIcon size={12} />
            </button>
          ) : null}
        </div>

        <div className="tabs">
          {FILTERS.map((item) => (
            <button
              key={item.key}
              className={`tab${filter === item.key ? " active" : ""}`}
              onClick={() => setFilter(item.key)}
            >
              {item.label}
              <span className="count">{counts[item.key]}</span>
            </button>
          ))}
        </div>

        <div className="spacer" />

        {pendingIds.length > 0 ? (
          <button
            className="btn"
            onClick={() => void ensureFor(pendingIds, "已启用免 UAC 启动")}
            title="一次性为全部待授权项创建免 UAC 启动项"
          >
            <BoltIcon size={14} />
            一键启用待授权（{pendingIds.length}）
          </button>
        ) : null}
      </div>

      <main className="content">
        {loading ? (
          <div className="skeleton-grid">
            <div className="skeleton" />
            <div className="skeleton" />
            <div className="skeleton" />
          </div>
        ) : visible.length > 0 ? (
          <div className="grid">
            {visible.map((app, index) => (
              <AppCard
                key={app.id}
                app={app}
                index={index}
                icon={icons[app.path] ?? null}
                busy={busy === app.id}
                onLaunch={() => void handleLaunch(app)}
                onEnable={() => void ensureFor([app.id])}
                onEdit={() => setEditing(app)}
                onRemove={() => setRemoving(app)}
                onReveal={() => void handleReveal(app)}
                onCopyPath={() => void handleCopyPath(app)}
                onSetMode={(mode) => void handleSetMode(app, mode)}
                onCreateShortcut={() => void handleCreateShortcut(app)}
              />
            ))}
          </div>
        ) : apps.length === 0 ? (
          <div className="empty">
            <div className="halo">
              <ShieldIcon size={42} />
            </div>
            <h2>还没有添加任何程序</h2>
            <p>
              选择需要以管理员权限运行、但不想每次都看到 UAC 弹窗的程序。
              UACSkip 会为它注册一个免授权的启动项，之后双击即可直接运行。
            </p>
            <div className="actions">
              <button className="btn primary" onClick={() => void handleAdd()} disabled={busyAdd}>
                <PlusIcon size={15} />
                打开资源管理器选择程序
              </button>
            </div>
            <div className="hints">
              <span className="hint">
                <BoltIcon size={13} />
                <b>拖入窗口</b>即可添加
              </span>
              <span className="hint">
                <ShieldIcon size={13} />
                支持 <b>exe / bat / cmd</b>
              </span>
              <span className="hint">
                <FolderIcon size={13} />
                {store?.portable ? (
                  <>
                    配置保存在<b>程序目录</b>
                  </>
                ) : (
                  <>
                    配置保存在<b>用户目录</b>
                  </>
                )}
              </span>
            </div>
            {store ? (
              <button
                className="store-path"
                title="点击复制配置文件的完整路径"
                onClick={() => {
                  void copyText(store.path).then((ok) =>
                    pushToast(ok ? "success" : "error", ok ? "路径已复制" : "复制失败"),
                  );
                }}
              >
                {store.path}
              </button>
            ) : null}
          </div>
        ) : (
          <div className="empty">
            <div className="halo">
              <SearchIcon size={36} />
            </div>
            <h2>没有匹配的启动项</h2>
            <p>换个关键词，或切换到其他筛选条件看看。</p>
            <div className="actions">
              <button
                className="btn"
                onClick={() => {
                  setQuery("");
                  setFilter("all");
                }}
              >
                清除筛选
              </button>
            </div>
          </div>
        )}
      </main>

      {dragging ? (
        <div className="drop-overlay">
          <div className="drop-inner">
            <PlusIcon size={30} />
            松开即可添加为免 UAC 启动项
          </div>
        </div>
      ) : null}

      {editing ? (
        <EditDialog
          app={editing}
          onCancel={() => setEditing(null)}
          onSave={handleSaveEdit}
        />
      ) : null}

      {removing ? (
        <ConfirmDialog
          app={removing}
          onCancel={() => setRemoving(null)}
          onConfirm={handleRemove}
        />
      ) : null}

      <Toasts items={toasts} onDismiss={dismissToast} />
    </div>
  );
}
