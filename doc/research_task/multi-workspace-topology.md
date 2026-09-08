# 多工作区拓扑设想（完善稿）—— 工作区即场景，脚本建环境

> 状态：设想稿，待评审（本人表示「还没完全思考清楚」，本文负责补全与对齐）
> 日期：2026-09-08
> 前置阅读：[dev-override-architecture.md](dev-override-architecture.md)（v2 报告；其第 5 章的 WS-Game/WS-Delivery/WS-Apps 是本文的修订对象）

## 1. 与 v2 报告的关系

v2 按场景矩阵（S1~S7）提出三工作区：WS-Game（单仓游戏）/ WS-Delivery（交付链）/ WS-Apps（应用链）。本稿按你的最新想法修订为「**1 全 + 2 专**」结构，差异在：

- v2 的 WS-Game 设想「改框架经 Dev Channel 无需打开 framework 仓」；但你的真实工作模式是**做游戏的过程中顺手更新框架和工具**（你是平台方本人）——所以游戏工作区应直接包含供给链仓库，而非依赖单仓 + Dev Channel 切换
- v2 的 WS-Apps 把 4 个应用仓打包；你的意图是按「开发新应用」的任务粒度组织（tools + appsdk + 目标 app）
- 新增「完全工作区」承接跨链复杂问题（v2 的 S3 跨界 lane + 生态治理类任务）

## 2. 工作区定义

| 工作区 | 成员仓库 | 覆盖场景 | 定位 |
|--------|----------|----------|------|
| **ws-full（完全工作区）** | 全部：tools / framework / appsdk / editor-patch / visual-injector / mini-runtime / knowledge / 游戏项目 / veri | 生态治理、跨链联调（S3 类 MCP 能力改造）、逆向研究、知识库维护 | 解决复杂问题，平时不开 |
| **ws-game（游戏×框架工作区）** | 游戏项目（test_res002 等）+ framework + tools + editor-patch | 日常做游戏（S1/S2）+ 顺手改框架/工具（你是框架与工具作者）+ MCP 调试闭环消费（S3 消费侧） | 日常主战场 |
| **ws-app（应用开发工作区）** | tools + appsdk + 目标 sce_app_*（开发哪个挂哪个） | 开发/改造某个 bgd 应用（S4~S6 供给侧）、appsdk 底盘演进 | 应用迭代时开 |

补充说明：

- **knowledge（bgd_sce_knowledge）**：固定在 ws-full；其他工作区需要查官方包源码时可临时加 folder（各仓 AGENTS.md 的 `<workspace>/bgd_sce_knowledge` 指针已支持按名发现）。
- **veri（验证项目）**：固定在 ws-full 与 ws-game（tools 的硬性约定要求 CLI 改动必须在 veri 上验证）。
- **editor-patch 进 ws-game 的理由**：游戏开发中 AI 闭环调试（MCP）是唯一稳定横跨「游戏 × 应用链」的场景；能力不足时多数情况是 framework 侧 dbg_bus 加端点（ws-game 内解决），少数才需动桥本体——动桥时切 ws-full。
- **mini-runtime / visual-injector** 不进 ws-game：S4（触编注入）、S5（脱机调试）是低频事件驱动，用到时进 ws-full 或单开。

## 3. 工作区文件的管理（.code-workspace 放哪）

现状问题：`bgd_sce_tools/bgd_sce_tools.code-workspace` 被 gitignore，且内含机器相关绝对路径（test_res002 在 `C:/Users/...`），换机即丢。

建议方案：

1. **集中入库**：在 tools 仓建 `workspaces/` 目录，存放 `ws-full.code-workspace` / `ws-game.code-workspace` / `ws-app.code-workspace`，随 git 走（tools 是生态入口仓，工作区定义属生态资产）。
2. **路径写法**：生态仓库一律相对路径（`../bgd_sce_framework` 等，平级布局约定）；游戏项目路径机器相关（SCE Projects 目录），用**环境变量占位 + 脚本渲染**：`workspaces/` 内存放 `.template` 版，bootstrap 脚本（见第 4 节）按本机实际路径生成可用的 `.code-workspace`（生成物加 gitignore）。
3. 删除现有 `bgd_sce_tools.code-workspace` 或迁移为 `workspaces/ws-full` 的生成产物。

> 备选：新建独立「环境仓」存放工作区 + 脚本。不推荐——tools 已是生态宿主，再建仓徒增分发成本（与 v2 「不为 skills 建第七仓」同一逻辑）。

## 4. 工作区的第二职责：脚本建立本机环境

即 v2 §9.3 BOOTSTRAP 的落地。`workspaces/` 目录同时承载环境脚本（`bootstrap.ps1`）：

1. 检查/克隆全部生态仓库到平级目录（私有仓先配 token）
2. 工具链体检：Rust stable / Node 22 / dotnet 9 / 代理可达（远期并入 `bgd_sce_tools doctor` CLI）
3. 渲染本机的 `.code-workspace`（替换游戏项目路径占位）
4. 配置 Dev Channel 键（`framework_local_path` / `app_dev_paths`）
5. 验证闭环：veri 项目 `build` + 一个 app 经 `app_dev_paths` 拉起

换机流程收敛为：装工具链 → clone tools → 跑 bootstrap → 打开任一工作区。

## 5. 与既有机制的联动

- **Dev Channel 不变**：ws-game 里 framework 仓在场，可直接改 + `update-framework --local`；ws-full 同理。Dev Channel 的价值从「不开仓也能改」转变为「开了仓则改完当场生效」。
- **AGENTS.md 指针不变**：各仓的关联知识库段落与工作区划分正交（`<workspace>` 写法在任何工作区都成立）。
- **需求池不变**：游戏侧缺口仍记 `test_res002/.bgd/doc/生态需求池.md`，ws-game/ws-full 会话消费。

## 6. 开放问题（待定夺）

1. ws-game 是否含 editor-patch？（本文建议含，理由是 S3 消费侧高频；若觉得重可降为 ws-full 专属）
2. `.code-workspace` 集中放 tools 仓 `workspaces/` 是否接受？还是倾向独立环境仓？
3. 多个游戏项目并存时，ws-game 是「一游戏一工作区文件」（ws-game-test_res002 / ws-game-xxx）还是单文件多 folder？
4. bootstrap 脚本先做 PowerShell 单文件，还是直接做进 tools CLI（`doctor` / `env setup` 子命令）？
