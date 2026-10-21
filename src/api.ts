import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import type {
  AddOutcome,
  AppPatch,
  AppView,
  LaunchOutcome,
  RemoveOutcome,
  StoreInfo,
  TaskOutcome,
} from "./types";

const EXECUTABLE_FILTER = [
  { name: "可执行程序", extensions: ["exe", "bat", "cmd"] },
  { name: "所有文件", extensions: ["*"] },
];

export const api = {
  isElevated: () => invoke<boolean>("is_elevated"),

  listApps: () => invoke<AppView[]>("list_apps"),

  addApps: (paths: string[]) => invoke<AddOutcome[]>("add_apps", { paths }),

  updateApp: (id: string, patch: AppPatch) =>
    invoke<AppView>("update_app", { id, patch }),

  removeApp: (id: string, deleteTask: boolean) =>
    invoke<RemoveOutcome>("remove_app", { id, deleteTask }),

  launchApp: (id: string) => invoke<LaunchOutcome>("launch_app", { id }),

  ensureTasks: (ids?: string[]) =>
    invoke<TaskOutcome[]>("ensure_tasks", { ids: ids ?? null }),

  appIcon: (path: string) => invoke<string | null>("app_icon", { path }),

  revealInExplorer: (path: string) =>
    invoke<void>("reveal_in_explorer", { path }),

  relaunchElevated: () => invoke<void>("relaunch_elevated"),

  createDesktopShortcut: (id: string) =>
    invoke<string>("create_desktop_shortcut", { id }),

  storeInfo: () => invoke<StoreInfo>("store_info"),
};

/** 打开系统文件选择框，选择可执行程序 */
export async function pickExecutables(): Promise<string[]> {
  const picked = await open({
    multiple: true,
    directory: false,
    title: "选择要免 UAC 启动的程序",
    filters: EXECUTABLE_FILTER,
  });
  if (!picked) return [];
  return Array.isArray(picked) ? picked : [picked];
}

/** 打开目录选择框 */
export async function pickDirectory(): Promise<string | null> {
  const picked = await open({
    multiple: false,
    directory: true,
    title: "选择工作目录",
  });
  if (!picked || Array.isArray(picked)) return null;
  return picked;
}

/** 打开单个文件选择框（用于更换路径） */
export async function pickExecutable(): Promise<string | null> {
  const picked = await open({
    multiple: false,
    directory: false,
    title: "选择可执行程序",
    filters: EXECUTABLE_FILTER,
  });
  if (!picked || Array.isArray(picked)) return null;
  return picked;
}
