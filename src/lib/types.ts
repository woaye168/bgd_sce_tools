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
  templates_dir: string;
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

export type PageKey = "project" | "build" | "watch" | "settings" | "about";
