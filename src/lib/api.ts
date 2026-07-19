/** Tauri 命令封装与日志事件订阅 */
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  BgdConfig,
  FrameworkUpdateInfo,
  LogEvent,
  ProjectInfo,
} from "./types";

export const api = {
  selectProject: (path: string) =>
    invoke<string[]>("select_project", { path }),
  getCurrentProject: () => invoke<string | null>("get_current_project"),
  getRecentProjects: () => invoke<string[]>("get_recent_projects"),
  getProjectInfo: () => invoke<ProjectInfo>("get_project_info"),

  getConfig: () => invoke<BgdConfig>("get_config"),
  saveConfig: (config: BgdConfig) => invoke<void>("save_config", { config }),

  fullBuild: () => invoke<void>("full_build"),
  cleanBuild: () => invoke<void>("clean_build"),
  cleanLogs: () => invoke<number>("clean_logs"),

  startWatch: () => invoke<void>("start_watch"),
  stopWatch: () => invoke<void>("stop_watch"),
  isWatching: () => invoke<boolean>("is_watching"),

  initProject: (path: string, repo: string) =>
    invoke<string>("init_project", { path, repo }),
  checkFrameworkUpdate: () =>
    invoke<FrameworkUpdateInfo>("check_framework_update"),
  updateFramework: () => invoke<string>("update_framework"),
};

/** 订阅后端日志事件（build / watch 共用通道） */
export function onLog(
  callback: (event: LogEvent) => void
): Promise<UnlistenFn> {
  return listen<LogEvent>("bgd-log", (e) => callback(e.payload));
}
