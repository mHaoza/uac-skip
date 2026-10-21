import { useState } from "react";
import type { AppView } from "../types";
import { TrashIcon, WarningIcon } from "./Icons";

interface Props {
  app: AppView;
  onCancel: () => void;
  onConfirm: (deleteTask: boolean) => Promise<void>;
}

export function ConfirmDialog({ app, onCancel, onConfirm }: Props) {
  const [deleteTask, setDeleteTask] = useState(true);
  const [busy, setBusy] = useState(false);
  const hasTask = Boolean(app.taskName);

  const confirm = async () => {
    if (busy) return;
    setBusy(true);
    try {
      await onConfirm(deleteTask);
    } finally {
      setBusy(false);
    }
  };

  return (
    <div
      className="overlay"
      onMouseDown={(event) => {
        if (event.target === event.currentTarget && !busy) onCancel();
      }}
    >
      <div className="dialog" role="dialog" aria-modal="true" aria-label="移除启动项">
        <div className="dialog-head">
          <div>
            <h3>移除「{app.name}」</h3>
            <p>该程序会从 UACSkip 列表中移除，不会删除程序文件本身。</p>
          </div>
        </div>

        <div className="dialog-body">
          <div className="callout">
            <WarningIcon size={14} />
            <span>
              免 UAC 启动依赖系统任务计划中的条目
              {hasTask ? `（${app.taskName}）` : ""}。如果保留该任务，它会继续存在于系统中。
            </span>
          </div>

          {hasTask ? (
            <label
              style={{
                display: "flex",
                alignItems: "center",
                gap: 9,
                fontSize: 12.3,
                color: "var(--text-dim)",
                cursor: "pointer",
              }}
            >
              <input
                type="checkbox"
                checked={deleteTask}
                onChange={(event) => setDeleteTask(event.target.checked)}
                disabled={busy}
                style={{ width: 15, height: 15, accentColor: "#4f7ffb" }}
              />
              同时删除系统中的计划任务（推荐，需要一次管理员授权）
            </label>
          ) : null}
        </div>

        <div className="dialog-foot">
          <button className="btn" onClick={onCancel} disabled={busy}>
            取消
          </button>
          <button className="btn danger" onClick={() => void confirm()} disabled={busy}>
            <TrashIcon size={14} />
            {busy ? "处理中…" : "确认移除"}
          </button>
        </div>
      </div>
    </div>
  );
}
