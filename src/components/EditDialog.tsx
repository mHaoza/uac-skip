import { useState } from "react";
import type { AppPatch, AppView, LaunchMode } from "../types";
import { pickDirectory, pickExecutable } from "../api";
import { BoltIcon, FolderIcon, ShieldIcon, WarningIcon } from "./Icons";

interface Props {
  app: AppView;
  onCancel: () => void;
  onSave: (patch: AppPatch) => Promise<void>;
}

export function EditDialog({ app, onCancel, onSave }: Props) {
  const [name, setName] = useState(app.name);
  const [path, setPath] = useState(app.path);
  const [args, setArgs] = useState(app.args);
  const [workingDir, setWorkingDir] = useState(app.workingDir ?? "");
  const [mode, setMode] = useState<LaunchMode>(app.mode);
  const [saving, setSaving] = useState(false);

  const changed =
    name !== app.name ||
    path !== app.path ||
    args !== app.args ||
    workingDir !== (app.workingDir ?? "") ||
    mode !== app.mode;

  const submit = async () => {
    if (saving || !changed) return;
    setSaving(true);
    try {
      await onSave({ name, path, args, workingDir, mode });
    } finally {
      setSaving(false);
    }
  };

  const chooseFile = async () => {
    const picked = await pickExecutable();
    if (picked) setPath(picked);
  };

  const chooseDir = async () => {
    const picked = await pickDirectory();
    if (picked) setWorkingDir(picked);
  };

  return (
    <div
      className="overlay"
      onMouseDown={(event) => {
        if (event.target === event.currentTarget && !saving) onCancel();
      }}
    >
      <div className="dialog" role="dialog" aria-modal="true" aria-label="编辑启动项">
        <div className="dialog-head">
          <div>
            <h3>编辑启动项</h3>
            <p>修改启动参数后需要重新授权一次，免 UAC 启动项才会同步更新。</p>
          </div>
        </div>

        <div className="dialog-body">
          <div className="field">
            <label htmlFor="edit-name">显示名称</label>
            <input
              id="edit-name"
              className="input"
              value={name}
              onChange={(event) => setName(event.target.value)}
              placeholder="留空则使用文件名"
            />
          </div>

          <div className="field">
            <label htmlFor="edit-path">程序路径</label>
            <div className="input-row">
              <input id="edit-path" className="input mono" value={path} readOnly />
              <button className="btn" onClick={chooseFile} disabled={saving}>
                <FolderIcon size={14} />
                选择
              </button>
            </div>
          </div>

          <div className="field">
            <label htmlFor="edit-args">
              启动参数 <span className="tag">可选</span>
            </label>
            <input
              id="edit-args"
              className="input mono"
              value={args}
              onChange={(event) => setArgs(event.target.value)}
              placeholder='例如 --silent "C:\data"'
            />
          </div>

          <div className="field">
            <label htmlFor="edit-dir">
              工作目录 <span className="tag">默认使用程序所在目录</span>
            </label>
            <div className="input-row">
              <input
                id="edit-dir"
                className="input mono"
                value={workingDir}
                onChange={(event) => setWorkingDir(event.target.value)}
                placeholder="留空则自动使用程序目录"
              />
              <button className="btn" onClick={chooseDir} disabled={saving}>
                <FolderIcon size={14} />
                选择
              </button>
            </div>
          </div>

          <div className="field">
            <label>启动方式</label>
            <div className="segmented">
              <button
                className={`seg${mode === "highest" ? " active" : ""}`}
                onClick={() => setMode("highest")}
                disabled={saving}
              >
                <span className="seg-icon">
                  <ShieldIcon size={16} />
                </span>
                <span>
                  <span className="seg-title">免 UAC 提权启动</span>
                  <span className="seg-desc">以管理员权限运行，不再弹出 UAC 窗口</span>
                </span>
              </button>
              <button
                className={`seg${mode === "normal" ? " active" : ""}`}
                onClick={() => setMode("normal")}
                disabled={saving}
              >
                <span className="seg-icon">
                  <BoltIcon size={16} />
                </span>
                <span>
                  <span className="seg-title">普通权限启动</span>
                  <span className="seg-desc">与双击打开一致，需要授权时仍会弹窗</span>
                </span>
              </button>
            </div>
          </div>

          {app.needsAdmin ? (
            <div className="callout">
              <WarningIcon size={14} />
              <span>检测到该程序的清单要求管理员权限，建议保持「免 UAC 提权启动」。</span>
            </div>
          ) : null}

          {mode === "highest" && changed ? (
            <div className="callout info">
              <ShieldIcon size={14} />
              <span>保存后需要一次管理员授权，用于把新的启动配置写入系统任务计划。</span>
            </div>
          ) : null}
        </div>

        <div className="dialog-foot">
          <button className="btn" onClick={onCancel} disabled={saving}>
            取消
          </button>
          <button
            className="btn primary"
            onClick={() => void submit()}
            disabled={saving || !changed}
          >
            {saving ? "保存中…" : "保存"}
          </button>
        </div>
      </div>
    </div>
  );
}
