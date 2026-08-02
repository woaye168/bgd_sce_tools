/** 与后端交互的类型定义 */

export interface BgdConfig {
  project_root: string;
  enable_build_log: boolean;
  asset_target: string;
  libs_asset_output_name: string;
  game_asset_output_name: string;
  server_entrance: string;
  client_entrance: string;
  libs_dir: string;
  libs_server_target: string;
  libs_client_target: string;
  libs_excludes: string[];
  game_dir: string;
  game_server_target: string;
  game_client_target: string;
  game_excludes: string[];
  framework_version: string;
  framework_repo: string;
}

export interface ProjectInfo {
  initialized: boolean;
  framework_version?: string;
  framework_repo?: string;
}

export interface LogEvent {
  source: "build" | "watch";
  line: string;
}

export interface FrameworkUpdateInfo {
  current: string;
  latest: string | null;
}

export type PageKey = "project" | "build" | "watch" | "apps" | "settings" | "about";

/** 应用清单项（registry.json 中一个应用） */
export interface AppInfo {
  id: string;
  name: string;
  version: string;
  description?: string;
  author?: string;
  download_url: string;
  checksum?: string;
}

/** 应用清单（registry.json 顶层） */
export interface AppRegistry {
  apps: AppInfo[];
}

/** 已安装应用（apps/{id}/app.json） */
export interface InstalledApp {
  id: string;
  name: string;
  version: string;
  description?: string;
  author?: string;
}

/** 应用级设置（与项目无关，存于应用配置目录 settings.json） */
export interface AppSettings {
  /** HTTP 代理地址，如 http://127.0.0.1:7897；留空表示直连 */
  proxy: string;
  /** 监听开关状态（持久化，启动时自动恢复） */
  watch_enabled: boolean;
  /** 保存日志文件开关（构建/监听日志写入 .bgd/log/build-YYYY-MM-DD.log） */
  save_log: boolean;
}

/** 框架增量更新报告 */
export interface UpdateReport {
  updated: number;
  added: number;
  removed: number;
  kept_local: number;
  conflicts: string[];
  notes: string[];
  version: string;
}
