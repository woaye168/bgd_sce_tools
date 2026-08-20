# BGD SCE 技术生态全面评估 Spec

## Why

以 bgd_sce_tools（宿主工具）为主视角、四个关联仓库为辅助视角的整个技术体系已初具规模，但从未做过系统性复盘。需要对五仓库的架构设计、代码质量、跨仓库协作机制进行全面审查，找出架构问题、代码问题与流程问题，形成多份研究文档，并将可落地的改进点沉淀为需求文档，为后续迭代提供决策依据。

## What Changes

- 新增多份研究报告，全部放入 `d:\sce_online\Res\maps\bgd_sce_tools\doc\research\`：
  - `00-总览与架构评估.md`：五仓库定位、协作关系、数据流、技术选型优缺点总评
  - `01-宿主工具评估.md`：bgd_sce_tools 本体（Rust 后端 builder/project/config/apps/cli/net/updater、Tauri 前端、CLI 契约）
  - `02-Lua框架评估.md`：bgd_sce_framework（静态 require 改写、白名单构建、配置 overlay、资源系统、三路哈希更新、libs/src 分层）
  - `03-应用生态评估.md`：bgd_sce_appsdk（公共基建）+ sce_app_visual-injector + sce_app_editor-patch（插槽/补丁/MCP 桥/截图等）
  - `04-跨仓库一致性与流程评估.md`：发布流程、版本号机制、私有仓库认证、网络代理、文档同步约定、AGENTS.md 维护
  - `05-问题清单与改进建议.md`：汇总所有发现的问题，按严重级别分类（架构/代码/流程/安全/可维护性），给出改进建议
- 新增需求文档，放入 `d:\sce_online\Res\maps\bgd_sce_tools\doc\requirements\`：
  - `audit-follow-ups.md`：从研究中提炼的可执行需求条目（每条含背景、问题、建议方案、优先级、影响范围）
- 本次评估**只产出文档，不修改任何功能代码**（非 BREAKING）。

## Impact

- Affected specs: 无既有 spec（首次建立）
- Affected code: 仅新增 `doc/research/*.md` 与 `doc/requirements/*.md`，不触碰任何源码
- 主视角仓库：`d:\sce_online\Res\maps\bgd_sce_tools`
- 辅助视角仓库：`bgd_sce_framework`、`bgd_sce_appsdk`、`sce_app_visual-injector`、`sce_app_editor-patch`

## ADDED Requirements

### Requirement: 分仓库源码深度审查
系统 SHALL 对五个仓库的核心源码逐一阅读分析（而非仅依赖 AGENTS.md 描述），覆盖：宿主 Rust 后端各模块与前端页面、框架 Lua 模板核心文件、appsdk 各模块、两个应用的 Rust 与补丁 Lua/C# 关键文件。

#### Scenario: 审查完整性
- **WHEN** 研究报告完成
- **THEN** 每个仓库的每个核心模块均有被实际阅读的证据（报告中能引用具体文件与行级事实）

### Requirement: 架构优缺点评估
系统 SHALL 输出整体架构评估：仓库边界划分是否合理、职责是否重叠或缺失、关键机制（require 改写/白名单/三路哈希/插槽注入/MCP 桥/应用市场链路）的稳健性与风险。

#### Scenario: 架构结论可追溯
- **WHEN** 阅读 00 与 04 号报告
- **THEN** 每条架构结论均有具体代码/机制引用支撑，优缺点成对呈现

### Requirement: 代码问题清单
系统 SHALL 找出具体代码问题，包括但不限于：错误处理缺失、重复实现、超长文件、硬编码、潜在 panic/竞态、安全问题（token 处理、注入、路径处理）、前后端契约不一致。

#### Scenario: 问题分级
- **WHEN** 阅读 05 号报告
- **THEN** 每个问题含：位置（文件+行）、现象、影响、严重级别（高/中/低）、建议修复方向

### Requirement: 需求文档沉淀
系统 SHALL 将可落地的改进点转化为需求文档，放入 `doc/requirements/`，每条需求含背景/问题/建议方案/优先级/涉及仓库。

#### Scenario: 需求可执行
- **WHEN** 阅读 requirements 文档
- **THEN** 任意一条需求可独立拆分为后续开发任务，不依赖口头上下文

## REMOVED Requirements

无（纯文档产出，不删除任何既有能力）。
