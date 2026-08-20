# Tasks

- [x] Task 1: 审查宿主工具 bgd_sce_tools 本体
  - [x] SubTask 1.1: 阅读 Rust 后端全部模块（main.rs / lib.rs / cli.rs / builder.rs / project.rs / config.rs / apps.rs / net.rs / updater.rs），记录每个模块的职责、实现质量、具体问题（错误处理/panic/竞态/硬编码/超长文件）
  - [x] SubTask 1.2: 阅读前端（src/ 全部页面与组件、lib/api.ts、types.ts），检查前后端命令契约一致性、状态管理、UI 代码质量
  - [x] SubTask 1.3: 检查 Cargo.toml / package.json / tauri.conf.json / CI 工作流，评估依赖选型与构建配置
  - [x] SubTask 1.4: 产出 `doc/research/01-宿主工具评估.md`

- [x] Task 2: 审查 Lua 框架 bgd_sce_framework
  - [x] SubTask 2.1: 阅读 template/.bgd/libs 核心文件（init.lua、common/server/client 各 api 模块、entrance、bgd_default.json、types/）
  - [x] SubTask 2.2: 阅读 template/.bgd/src 骨架（GameServer/GameClient/CombatSystem 等），评估模板代码质量与约定落地情况
  - [x] SubTask 2.3: 评估框架机制设计：静态 require 改写、白名单构建、API 聚合、配置 overlay、资源系统、三路哈希更新、AGENTS.md 同步
  - [x] SubTask 2.4: 产出 `doc/research/02-Lua框架评估.md`

- [x] Task 3: 审查应用生态（appsdk + 两个应用）
  - [x] SubTask 3.1: 阅读 bgd_sce_plugins（bgd_sce_appsdk）src/ 全部模块（app/single_instance/watcher/ui/log/config）+ registry.json，评估 SDK 抽象质量
  - [x] SubTask 3.2: 阅读 sce_app_visual-injector（main.rs / core.rs / 注入规范文档），评估触编注入实现
  - [x] SubTask 3.3: 阅读 sce_app_editor-patch 核心（src/ 各模块、patches/、slots/、csharp/bgd_mcp_bridge 关键文件），评估补丁机制、MCP 桥、截图、安全红线落实
  - [x] SubTask 3.4: 产出 `doc/research/03-应用生态评估.md`

- [x] Task 4: 跨仓库一致性与流程评估
  - [x] SubTask 4.1: 核对各仓库 AGENTS.md 描述与实际代码的一致性（如机制描述是否过期、命令是否仍存在）
  - [x] SubTask 4.2: 评估发布/版本流程：tag 发版、release notes 生成、registry/app-release.json 链路、框架版本号机制、私有仓库 token 认证
  - [x] SubTask 4.3: 评估流程性问题：文档同步约定执行、代理依赖、测试验证闭环、单点维护风险
  - [x] SubTask 4.4: 产出 `doc/research/00-总览与架构评估.md` 与 `doc/research/04-跨仓库一致性与流程评估.md`

- [x] Task 5: 汇总问题清单与需求沉淀
  - [x] SubTask 5.1: 汇总 Task 1-4 所有发现，按严重级别分类，产出 `doc/research/05-问题清单与改进建议.md`
  - [x] SubTask 5.2: 将可落地改进点转化为需求条目，产出 `doc/requirements/audit-follow-ups.md`

# Task Dependencies
- Task 4 的 SubTask 4.1/4.4 依赖 Task 1/2/3 的源码事实（但 4.2/4.3 可与 Task 1-3 并行）
- Task 5 依赖 Task 1-4 全部完成
- Task 1 / Task 2 / Task 3 之间无依赖，可并行
