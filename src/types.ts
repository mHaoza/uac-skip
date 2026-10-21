export type LaunchMode = "highest" | "normal";

/** 与 Rust 端 AppView 对应 */
export interface AppView {
  id: string;
  name: string;
  path: string;
  args: string;
  workingDir: string | null;
  mode: LaunchMode;
  taskName: string | null;
  dirty: boolean;
  needsAdmin: boolean;
  createdAt: number;
  lastLaunchedAt: number | null;
  launchCount: number;
  pathExists: boolean;
  taskExists: boolean | null;
  enabled: boolean;
}

export interface AppPatch {
  name?: string;
  path?: string;
  args?: string;
  workingDir?: string;
  mode?: LaunchMode;
}

export interface AddOutcome {
  ok: boolean;
  path: string;
  id: string | null;
  message: string | null;
}

export interface TaskOutcome {
  id: string;
  ok: boolean;
  message: string | null;
  cancelled: boolean;
}

export interface LaunchOutcome {
  ok: boolean;
  message: string | null;
  elevated: boolean;
  suggestElevate: boolean;
}

export interface RemoveOutcome {
  ok: boolean;
  message: string | null;
}

/** 配置文件位置 */
export interface StoreInfo {
  path: string;
  portable: boolean;
  dir: string;
}

export type ToastKind = "success" | "error" | "info";

export interface ToastAction {
  label: string;
  run: () => void | Promise<void>;
}

export interface ToastItem {
  id: number;
  kind: ToastKind;
  message: string;
  action?: ToastAction;
}
