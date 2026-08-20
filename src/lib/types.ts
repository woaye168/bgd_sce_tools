/** 与后端交互的类型定义 */

export interface BgdConfig {
  project_root: string;
  enable_build_log: boolean;
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

/** 应用清单项（registry 极简条目 + 应用仓库 app-release.json 补全的元数据） */
export interface AppInfo {
  id: string;
  name: string;
  version: string;
  description?: string;
  author?: string;
  /** GitHub 仓库（owner/repo） */
  repo: string;
  /** Release tag（旧格式兼容字段，可空） */
  tag?: string;
  /** Release asset 文件名 */
  asset_name: string;
  /** 下发默认：静默自启（用户本机记忆优先） */
  default_auto_start?: boolean;
  /** 版本说明（升级按钮展示用） */
  release_notes?: string;
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
  /** GitHub Token（fine-grained PAT，Contents 只读；私有仓库的框架/插件/自我更新均需要） */
  github_token: string;
  /** 随主程序静默启动的应用 id 列表（单开：已在运行不重复拉起） */
  auto_start_apps: string[];
  /** 用户手动取消静默自启的应用 id（registry 下发的默认值不再对这些 id 播种） */
  auto_start_disabled: string[];
}

/** 自我更新检查结果 */
export interface SelfUpdateInfo {
  current: string;
  latest: string;
  has_update: boolean;
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
