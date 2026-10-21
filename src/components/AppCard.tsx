import { useState } from "react";
import type { AppView, LaunchMode } from "../types";
import { formatDateTime, formatRelative } from "../format";
import {
  BoltIcon,
  CopyIcon,
  FolderIcon,
  MoreIcon,
  PencilIcon,
  PlayIcon,
  ShieldIcon,
  ShortcutIcon,
  SpinnerIcon,
  TrashIcon,
  WarningIcon,
} from "./Icons";

interface Props {
  app: AppView;
  icon?: string | null;
  busy: boolean;
  index: number;
  onLaunch: () => void;
  onEnable: () => void;
  onEdit: () => void;
  onRemove: () => void;
  onReveal: () => void;
  onCopyPath: () => void;
  onSetMode: (mode: LaunchMode) => void;
  onCreateShortcut: () => void;
}

type Status = { kind: "ok" | "warn" | "danger" | "muted"; label: string };

function statusOf(app: AppView): Status {
  if (!app.pathExists) return { kind: "danger", label: "文件不存在" };
  if (app.mode === "normal") return { kind: "muted", label: "普通权限启动" };
  if (app.enabled) return { kind: "ok", label: "免 UAC 已就绪" };
  if (app.dirty) return { kind: "warn", label: "配置已修改" };
  return { kind: "warn", label: "待授权" };
}

export function AppCard({
  app,
  icon,
  busy,
  index,
  onLaunch,
  onEnable,
  onEdit,
  onRemove,
  onReveal,
  onCopyPath,
  onSetMode,
  onCreateShortcut,
}: Props) {
  const [menuOpen, setMenuOpen] = useState(false);
  const status = statusOf(app);
  const pending = app.mode === "highest" && !app.enabled;

  return (
    <div
      className={`card${app.pathExists ? "" : " missing"}`}
      style={{ animationDelay: `${Math.min(index, 12) * 22}ms` }}
    >
      <div className="card-head">
        <div className="app-icon">
          {icon ? <img src={icon} alt="" draggable={false} /> : <ShieldIcon size={20} />}
        </div>
        <div className="card-title">
          <div className="name" title={app.name}>
            {app.name}
          </div>
          <div className="path" title={app.path}>
            {app.path}
          </div>
        </div>
      </div>

      <div className="badges">
        <span className={`badge ${status.kind}`}>
          <span className="dot" />
          {status.label}
        </span>
        {app.mode === "highest" ? (
          <span className="badge accent">
            <ShieldIcon size={11} />
            管理员权限
          </span>
        ) : null}
        {app.needsAdmin && app.mode === "normal" ? (
          <span className="badge warn">
            <WarningIcon size={11} />
            需要管理员
          </span>
        ) : null}
        {app.args.trim() ? <span className="badge muted">带参数</span> : null}
      </div>

      <div className="card-foot">
        <div className="meta" title={`上次启动：${formatDateTime(app.lastLaunchedAt)}`}>
          {app.launchCount > 0
            ? `${formatRelative(app.lastLaunchedAt)} · 共 ${app.launchCount} 次`
            : "从未启动"}
        </div>
        <div className="card-actions">
          {pending ? (
            <button
              className="btn sm"
              onClick={onEnable}
              disabled={busy}
              title="创建免 UAC 启动项（需要一次管理员授权）"
            >
              {busy ? <SpinnerIcon size={13} /> : <BoltIcon size={13} />}
              {app.dirty ? "重新授权" : "启用免 UAC"}
            </button>
          ) : null}
          <button
            className="btn run sm"
            onClick={onLaunch}
            disabled={busy || !app.pathExists}
            title={app.pathExists ? "立即启动" : "目标文件不存在"}
          >
            {busy ? <SpinnerIcon size={13} /> : <PlayIcon size={12} />}
            启动
          </button>
          <button className="btn ghost icon sm" onClick={onEdit} title="编辑">
            <PencilIcon size={14} />
          </button>
          <div className="menu-wrap">
            <button
              className="btn ghost icon sm"
              onClick={() => setMenuOpen((v) => !v)}
              title="更多操作"
            >
              <MoreIcon size={15} />
            </button>
            {menuOpen ? (
              <>
                <div className="menu-backdrop" onClick={() => setMenuOpen(false)} />
                <div className="menu">
                  <button
                    onClick={() => {
                      setMenuOpen(false);
                      onReveal();
                    }}
                  >
                    <FolderIcon size={14} />
                    打开文件位置
                  </button>
                  <button
                    onClick={() => {
                      setMenuOpen(false);
                      onCopyPath();
                    }}
                  >
                    <CopyIcon size={14} />
                    复制程序路径
                  </button>
                  {app.enabled ? (
                    <button
                      onClick={() => {
                        setMenuOpen(false);
                        onCreateShortcut();
                      }}
                    >
                      <ShortcutIcon size={14} />
                      创建桌面快捷方式
                    </button>
                  ) : null}
                  <div className="sep" />
                  {app.mode === "normal" ? (
                    <button
                      onClick={() => {
                        setMenuOpen(false);
                        onSetMode("highest");
                      }}
                    >
                      <ShieldIcon size={14} />
                      改为免 UAC 启动
                    </button>
                  ) : (
                    <button
                      onClick={() => {
                        setMenuOpen(false);
                        onSetMode("normal");
                      }}
                    >
                      <PlayIcon size={13} />
                      改为普通权限启动
                    </button>
                  )}
                  <div className="sep" />
                  <button
                    className="danger"
                    onClick={() => {
                      setMenuOpen(false);
                      onRemove();
                    }}
                  >
                    <TrashIcon size={14} />
                    从列表移除
                  </button>
                </div>
              </>
            ) : null}
          </div>
        </div>
      </div>
    </div>
  );
}
