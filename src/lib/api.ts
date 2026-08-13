/** Tauri 命令封装与日志事件订阅 */
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  AppInfo,
  AppRegistry,
  AppSettings,
  BgdConfig,
  FrameworkUpdateInfo,
  InstalledApp,
  LogEvent,
  ProjectInfo,
  SelfUpdateInfo,
  UpdateReport,
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

  initProject: (path: string, repo: string, force: boolean) =>
    invoke<string>("init_project", { path, repo, force }),
  checkFrameworkUpdate: () =>
    invoke<FrameworkUpdateInfo>("check_framework_update"),
  updateFramework: () => invoke<UpdateReport>("update_framework"),

  getAppSettings: () => invoke<AppSettings>("get_app_settings"),
  saveAppSettings: (settings: AppSettings) =>
    invoke<void>("save_app_settings", { settings }),

  // 自我更新（自建逻辑：私有仓库下 tauri updater 插件无法携带 token）
  checkSelfUpdate: (current: string) =>
    invoke<SelfUpdateInfo>("check_self_update", { current }),
  startSelfUpdate: () => invoke<void>("start_self_update"),

  // 应用（WeGame 模式）
  fetchAppRegistry: (url: string) =>
    invoke<AppRegistry>("fetch_app_registry", { url }),
  installApp: (appInfo: AppInfo) =>
    invoke<void>("install_app", { appInfo }),
  uninstallApp: (appId: string) => invoke<void>("uninstall_app", { appId }),
  getInstalledApps: () => invoke<InstalledApp[]>("get_installed_apps"),
  startApp: (appId: string) => invoke<void>("start_app", { appId }),
};

/** 订阅后端日志事件（build / watch 共用通道） */
export function onLog(
  callback: (event: LogEvent) => void
): Promise<UnlistenFn> {
  return listen<LogEvent>("bgd-log", (e) => callback(e.payload));
}
