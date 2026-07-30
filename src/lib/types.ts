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

export type PageKey = "project" | "build" | "watch" | "plugins" | "settings" | "about";

/** 应用级设置（与项目无关，存于应用配置目录 settings.json） */
export interface AppSettings {
  /** HTTP 代理地址，如 http://127.0.0.1:7897；留空表示直连 */
  proxy: string;
  /** 监听开关状态（持久化，启动时自动恢复） */
  watch_enabled: boolean;
  /** 保存日志文件开关（构建/监听日志写入 .bgd/log/build-YYYY-MM-DD.log） */
  save_log: boolean;
  /** 插件仓库地址列表（registry.json URL，官方仓库固定第一行） */
  plugin_registries: string[];
  /** 插件启用状态（id -> enabled，缺省视为启用） */
  plugin_enabled: Record<string, boolean>;
}

/** 已安装插件（.bgd/plugins/{id}.dll + 同名 .json 元数据） */
export interface PluginInfo {
  id: string;
  name: string;
  version: string;
  description: string;
  author: string;
  path: string;
  enabled: boolean;
  /** 是否导出 UI 钩子（决定是否显示「打开」按钮） */
  has_ui: boolean;
}

/** 仓库 registry.json 中的插件条目 */
export interface RegistryEntry {
  id: string;
  name: string;
  version: string;
  description: string;
  author: string;
  download_url: string;
  checksum: string;
}

/** 插件下载进度事件（plugin-install-progress） */
export interface PluginInstallProgress {
  id: string;
  downloaded: number;
  total: number;
}

/** API 模块条目（scan_api_modules 返回） */
export interface ApiModuleEntry {
  /** 模块标识：<set>/<side>/<name> */
  id: string;
  /** 所属集合：libs / src */
  set: string;
  /** 端：common / server / client */
  side: string;
  /** 模块名（文件名去 .lua） */
  name: string;
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
