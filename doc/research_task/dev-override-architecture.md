# 生态级架构治理报告 v2 —— Dev Channel、知识资产与工作流

> 报告性质：仅架构论证，不含代码实现。结论基于 7 个目录（bgd_sce_tools / bgd_sce_framework / bgd_sce_appsdk / sce_app_editor-patch / sce_app_mini-runtime / test_res002 / bgd_sce_veri）磁盘实读 + 业界实践调研。
>
> v2 修订说明：本版先重建「生态拓扑与仓库存在价值」的整体理解（第 1 章），再引入业界成熟做法对照（第 2 章），把 Dev Override 升级为统一的「Dev Channel」概念（第 3 章），新增知识/需求/Skills/MCP 四类生态资产的治理模型（第 4 章）与 Skills 专题（第 6 章），并回应 v1 的三处担忧：需求收件箱过重、业务侧绕法、工作区划分依据不足。
>
> 日期：2026-09-04

---

## 0. 生态起源与存在价值重建（一切论证的前提）

按你口述的发展史，生态的真实拓扑不是「6 个平级仓库」，而是一棵**从游戏开发需求长出来的树**：

```
                     星火编辑器/引擎（外部黑盒，加密包、无 MCP）
                          │
        ┌─────────────────┼──────────────────┐
        │ 被它限制          │ 需要它配合          │ 需要绕过/看穿它
        ▼                 ▼                  ▼
  bgd_sce_framework   sce_app_visual-      sce_app_editor-patch
  「游戏源码集中管理      injector           「给编辑器解锁：
   + 框架能力」          「把 Lua API 回写    解密库/打补丁/
        │                成触编可视化元件」   建 MCP 桥」
        │ 源码如何进                              │
        │ 真实运行时目录                           │ 还看穿了引擎二进制
        ▼                                         ▼
  bgd_sce_tools                          sce_app_mini-runtime
  「构建/下发/分发宿主」                    「脱机运行时/抓包/自研 host」
        │                                         │
        └──────── 两者都要做桌面应用 ──────────────┘
                          ▼
                 bgd_sce_appsdk
                 「应用底盘：单实例/驻留/窗口壳/协议」

  test_res002 = 一切的起因与最终消费者（既是客户，又是孵化器）
  bgd_sce_veri = 交付链的洁净验证场
```

由此得出每条链的**存在价值一句话**：

| 仓库 | 存在价值 | 没了它会怎样 |
|------|----------|--------------|
| test_res002 | 目的本身：做游戏 | 生态失去存在意义 |
| bgd_sce_framework | 让游戏源码可集中管理、可复用、可演进 | 每个游戏项目重复造框架 |
| bgd_sce_tools | 框架源码 → 星火真实运行时目录的**交付链** + 生态**分发宿主** | 框架无法落地，应用无处分发 |
| sce_app_editor-patch | 把封闭的星火编辑器变成**可被 AI 操作的平台**（MCP 唯一入口） | AI 闭环调试不存在 |
| sce_app_visual-injector | 打通「代码 → 触编可视化」的回写链 | 策划/可视化协作断链 |
| sce_app_mini-runtime | 把运行时从黑盒变成**可脱机、可观测、可自研** | 深度调试与逆向止步于编辑器 |
| bgd_sce_appsdk | 让上面三个应用不写重复底盘代码 | 三个 app 各自漂移 |

**关键洞察**：这个生态的本质是「**一个人同时扮演平台方与用户**」。所有痛点都来自这对矛盾——平台方需要分发纪律（tag/release/registry/token），用户（你自己）需要零成本迭代。业界对这对矛盾有标准答案，见第 2 章。

### 0.1 游戏开发中真实的跨仓场景（工作区划分的依据）

从 test_res002 的日常倒推，触发其他仓库的场景只有这 7 类：

| # | 场景（在 test_res002 里做什么） | 实际动到的仓库 | 频率 |
|---|------|------|------|
| S1 | 写业务代码、构建、监听 | tools（仅消费 build/watch） | 极高 |
| S2 | 缺框架能力（UI/网络/工具类） | framework（或直接在 test_res002 的 libs 共建） | 高 |
| S3 | AI 闭环测试游戏，MCP 缺能力 | **editor-patch（桥）+ framework（dbg_bus）两侧** | 中高 |
| S4 | 把写好的 API 给触编用 | visual-injector | 中 |
| S5 | 脱机调试/抓包/引擎行为存疑 | mini-runtime（+ 逆向研究） | 中低 |
| S6 | 编辑器升级，补丁失效 | editor-patch（重打补丁、重生成 catalog） | 低（事件驱动） |
| S7 | 沉淀逆向/排障知识 | editor-patch 或 mini-runtime 的 doc/research（当前部分错误地落在 test_res002） | 中 |

**场景结论**：S1/S2 占日常 80%+，只需 test_res002 单仓；S3 是唯一稳定的「跨交付链 × 应用链」场景；S4-S7 各自只触达应用链的一个仓。**不存在需要同时打开 6 仓的真实场景**——这直接证明全量工作区是错误的，也给出了工作区划分的事实依据（第 5 章）。

---

## 1. 现状瓶颈再分析（在起源视角下）

v1 的四个卡点仍然成立（分发回路复用为开发回路、内容/版本双通道、crates.io 串联依赖、test_res002 双重身份），本版补充三条**生态级**瓶颈：

**卡点五：知识资产的位置错乱。**
- 引擎 UI（cgui）最深的知识沉淀在游戏项目 test_res002（`.bgd/src/doc/` 11 份 cgui 文档群），而 cgui 代码本体在下发型 libs 里——**知识跟着项目走，不跟代码走**；framework 模板的 libs/doc 反而停留在 7 份旧 API 文档，落后于实例的 13 份。
- 引擎逆向知识被切成两半互不引用：编辑器侧在 editor-patch（`doc/research/` 9 份 + 两个库知识库 skill），引擎二进制/协议侧在 mini-runtime（`doc/research/` 10+ 份 + 36 份编号研究笔记）。TNND/UPAK 解密主题两仓各自表述（editor-patch `pak-extract-guide.md` × mini-runtime `payload-packages.md`）。
- 实证漂移：`cgui引擎行为排障笔记.md` 同名双份并存；editor-patch AGENTS.md 声称的 `test/knowledge/` 平铺结构与磁盘实际（按版本归档）不符；framework AGENTS.md 声称的 `template/.bgd/src/doc/` 实际不存在；一份需求文档游离在 `test/requirements/`。

**卡点六：AI 资产（Skills）只有一个仓有，且没有分发通道。**
全生态仅 editor-patch 有 `.trae/skills/`（4 个：2 个流程技能 + 2 个库知识库）。test_res002（AI 最密集使用的游戏项目）反而是零 skills。Skills 目前是「长在哪就烂在哪」，没有 framework 那样的下发机制。

**卡点七：需求管理其实已有传统，但没用在生态层。**
各仓已有 `doc/requirements/x.x.x.md` 按版本规划需求的惯例（editor-patch 24 份、mini-runtime 6 份、tools 3 份）——**生态缺的不是需求管理方法，而是「游戏开发现场 → 基建版本规划」的输入通道**。v1 提出的模板化需求池文件组确实重了，修订见第 5 章。

---

## 2. 业界成熟做法对照

你的处境在业界有成熟对应物，按层对照如下。

### 2.1 「平台方兼用户」的本地覆盖：这是有标准答案的

| 生态 | 机制 | 与你处境的对应 |
|------|------|----------------|
| Rust | **`[patch.crates-io]`**：依赖声明不动，本地路径/git 覆盖源；官方文档定位即「在发布前用真实应用验证库修改」 | appsdk 联调的标准解法，无需等发 crates.io |
| Go | `go.work` + `replace`：多仓本地联机开发，提交物不含本地路径 | 同上 |
| Node | `workspace:*` / `overrides: link:` / npm link | 同上 |
| Python | `pip install -e`（editable install） | 同上 |
| .NET | 本地 NuGet feed | 同上 |
| vcpkg/Nix | overlay ports / overlays：上游不动，本地叠加层覆盖 | **与你需要的 framework override 最神似** |

**共同模式（业界共识）**：① 覆盖只改「源解析」，不改「消费逻辑」；② 覆盖声明收在**本地配置文件**（通常不提交或属个人配置），生产产物零感知；③ 环境变量只作临时开关，不作主机制；④ 文件系统链接（symlink/junction）被普遍视为脆弱手段。**v1 推荐的 bgd.json 键 + 宿主 setting 双入口方案与此完全同构**，方向无需推翻，本版将其上升为统一概念「Dev Channel」（第 3 章）。

Monorepo 是另一条路线（cargo workspace / Nx / Turborepo）：对单人强耦合生态有真实吸引力（原子跨仓提交、单一 AGENTS.md）。**但本生态不适合整体 monorepo**：星火游戏项目必须独立目录；各 app 需要独立 release 分发；私有仓 + token 分发链已按 polyrepo 建好。正确粒度是「**逻辑 monorepo，物理 polyrepo**」——物理保持现状，逻辑上通过生态地图、统一 Dev Channel、统一知识索引把它当一个仓库治理。

### 2.2 AI 资产分层：2026 年业界收敛的四层模型

| 层 | 载体 | 回答的问题 | 加载方式 |
|----|------|-----------|----------|
| 环境上下文 | **AGENTS.md**（开放标准，20+ 工具） | 「这是什么地方，规矩是什么」 | 每会话常驻，故必须短 |
| 可调用能力 | **Skills**（SKILL.md，Anthropic 发起已成跨平台标准） | 「这类任务怎么做」 | **按需加载**（description 匹配触发） |
| 实时数据/操作 | **MCP** | 「AI 能触达什么」 | 工具定义常驻 + 能力目录搜索 |
| 长效知识 | **docs/research/ADR** | 「为什么这样、踩过什么坑」 | AI 按需检索 |

要点：① AGENTS.md 与 Skills 是互补不是竞争——**AGENTS.md 管「此处」，Skills 管「此类任务」**；② Skills 天然支持「集中存放 + 按需发现」，description frontmatter 就是发现机制，不需要全量注入上下文；③ 凡「必须每次执行」的约束不应依赖文档，应进 hook/CI。你的生态已有 MCP 能力目录的 search→invoke 模式，Skills 的发现本质是同一件事。

### 2.3 决策与需求沉淀：ADR + 版本需求文档

轻量 ADR（Architecture Decision Record，一条决策一份短文档：背景/决定/后果）是跨仓决策的标准沉淀方式——本报告实质上就是一份 ADR。需求侧你已有按版本 `doc/requirements/x.x.x.md` 的成熟惯例，直接复用即可。

---

## 3. 核心架构概念：Dev Channel（本地通道）

把 v1 的两个孤立方案统一为一个概念：**生态存在两条供给通道——Release Channel（GitHub，服务第三方用户）与 Dev Channel（本地文件系统，服务你本人）。两者共享全部下游消费逻辑，仅「源解析」入口不同。**

```
                 ┌── Release Channel ──────────────┐
  源解析入口 ────┤  codeload zip / release asset    │──→ 三路哈希对比 / 安装启动（不变）
   （唯一分叉点） └── Dev Channel ─────────────────┘
                    本地目录 / 本地 exe（声明式覆盖）
```

| 产物类型 | Release Channel（现状不动） | Dev Channel（新增覆盖键） | 配置载体 |
|----------|---------------------------|--------------------------|----------|
| 框架 libs | codeload main 快照 | `framework_local_path` → 本地 framework 仓库 `template/` | 项目级 bgd.json |
| 应用 exe | release asset API | `app_dev_paths[id]` → 本地 build 产物 | 宿主级 setting |
| Skills/文档（远期） | 随框架 zip 下发 | 同一 `framework_local_path` 顺带覆盖 | 同上 |

设计性质（与业界共识逐条对齐）：
- **源解析单点分叉**：framework 侧注入点是 `update_framework`/`init_project` 的下载函数（[project.rs](file:///d:/sce_online/Res/maps/bgd_sce_tools/src-tauri/src/project.rs) L162/L168/L179，下游 `extract_zip` L139 已只认「template 目录」）；app 侧注入点是启动寻址 `app_exe_path`（[apps.rs](file:///d:/sce_online/Res/maps/bgd_sce_tools/src-tauri/src/apps.rs) L99，start_app/autostart/CLI 三处调用）。
- **缺省 = Release Channel**，删键即回退，普通用户路径零条件分支。
- **声明在配置文件而非环境变量**：bgd.json 键随项目提交（审计痕迹），`config.rs::abs`（L174）已支持绝对路径；注意个人路径泄露问题——`framework_local_path` 属个人开发态，建议同时加入项目 `.gitignore` 豁免讨论（或接受提交，团队只有你时不构成问题）。
- **不用 Junction**（v1 已论证：哈希基准语义崩坏 / 写穿污染）。
- **SDK 联调用业界标准** `[patch.crates-io]`：放各 app 仓库根的 `.cargo/config.toml`（不提交），Cargo.toml 依赖声明一字不改——比改 path 依赖安全，CI 不受影响。

override 态语义：`framework_version` 记 `local`，`check-framework` 返回「Dev Channel 中」，GUI 显示徽标——堵住内容/版本双通道裂缝在 override 态的放大。

---

## 4. 生态资产治理：知识 / 需求 / Skills / MCP

这是 v1 完全缺失的部分。先给出**生态资产归属矩阵**（每条资产有且仅有一个 owner）：

### 4.1 知识库治理

| 知识域 | 当前位置 | 应归属 | 处置 |
|--------|---------|--------|------|
| 编辑器库源码级知识（script v199 / xdeditor v160） | editor-patch `.trae/skills/sce-lib-*` | editor-patch ✅ | 不动（知识与补丁代码同仓，健康） |
| 编辑器运行时/调试/解密逆向 | editor-patch `doc/research/` | editor-patch ✅ | 不动 |
| 引擎二进制/协议/载荷/云变量逆向 | mini-runtime `doc/research/` + `research_task/` | mini-runtime ✅ | 不动（知识与 Frida 探针同仓，健康） |
| **cgui/UI 引擎知识** | **test_res002** `.bgd/src/doc/`（错位！） | **framework**（cgui 代码在 libs） | **回迁**：移入 framework 模板 `libs/doc/`，经下发链同步到所有项目；同时修复「模板 doc 落后实例」漂移 |
| 生态全局决策（如本报告） | 各仓散落 | **tools** `doc/`（宿主是生态入口） | 建立 `doc/adr/` 轻量决策记录 |
| 游戏业务知识 | test_res002 | test_res002 ✅ | 不动 |

配套三个动作：
1. **生态知识地图**：tools `doc/` 一份单文件索引（知识域 → 各仓 canonical 文档链接），是唯一的跨仓知识入口；其余仓库 AGENTS.md 各留一行指针。**跨仓只许链接、不许复述**（复述必漂移，实证已发生）。
2. **去重**：`cgui引擎行为排障笔记.md` 双份合一；TNND/UPAK 主题两仓互链并指定 canonical（建议 mini-runtime 持有二进制层、editor-patch 持有编辑器包层，互相链接）。
3. **修漂移**：更正 editor-patch 与 framework AGENTS.md 中两处与磁盘不符的结构描述。

### 4.2 需求治理（回应「收件箱过重」担忧）

废除 v1 的「模板化需求单文件组」，改为**单文件 + 既有惯例**：

- **生态需求池** = 一个文件：`test_res002/.bgd/doc/生态需求池.md`（游戏开发现场唯一入口）。业务 AI 卡住时只追加一个条目，格式轻到不能再轻：

```markdown
## [2026-09-04] dbg_bus 缺少批量读取能力
- 场景：AI 测试 HUD 时需要逐个点开 5 个面板取状态
- 期望：invoke_capability 支持一次调用返回多面板状态
- 归属猜测：framework（dbg_bus）+ editor-patch（桥）
- 状态：pending
```

- **消费侧复用既有惯例**：基建 AI 做版本规划时（本来就要写 `doc/requirements/x.x.x.md`），第一步把需求池里归属本仓的 pending 条目收入版本需求文档，池中条目状态改为 `已收入 vX.Y.Z`。**池只是暂存，规划仍在各仓版本需求文档**——没有任何新流程，只是把已有的版本规划多了一个输入源。
- 撤销 v1 的「远期 MCP 化需求池」设想：文件已足够，不过度工程。

### 4.3 Skills 治理（详见第 6 章专题）

归属原则：**「此类任务」的技能跟着最常发生此类任务的仓库走**。
- 仓库私有 skills（补丁开发、库知识库）→ 留在 editor-patch，不动。
- **生态游戏开发 skills**（用这套工具链做游戏时会反复用到的工序：构建验证回路、触编注入流程、MCP 调试闭环等）→ 集中到 **framework 模板新增 `skills/` 目录**，由 tools 在下发/更新框架时同步到游戏项目 `.trae/skills/bgd-*/`——**完全复用 `sync_agents_md` 的现成模式**（根 AGENTS.md 已是这样从 `.bgd/src/AGENTS.md` 同步生成的），Dev Channel 对其同样生效。

### 4.4 MCP 的定位

MCP 不属于「治理对象」，它是**上述一切的运行时触达层**（四层模型中的 live 层）。治理上只需一条：MCP 能力变更天然横跨 editor-patch（桥）× framework（dbg_bus），这类变更的需求单归属字段标「联合」，实施时在 WS-Apps 工作区进行、framework 侧改动走 Dev Channel 当场验证。

---

## 5. 工作区与 AGENTS.md 治理（修订版）

### 5.1 工作区划分——以第 0.1 节场景矩阵为依据

| 工作区 | 成员 | 覆盖场景 | 依据 |
|--------|------|----------|------|
| **WS-Game** | test_res002 单仓 | S1、S2（经 Dev Channel 改 libs 也无需打开 framework 仓） | 日常 80%+ |
| **WS-Delivery** | tools + framework + bgd_sce_veri | 交付链改造、Dev Channel 自身开发 | S1/S2 的供给侧 |
| **WS-Apps** | appsdk + editor-patch + visual-injector + mini-runtime | S3~S7 | 共享 appsdk 契约与引擎知识语境 |

跨界场景处理：S3（MCP 能力）是唯一稳定的跨界 lane，处理方式是**临时任务工作区**（恰好打开 editor-patch + framework 两仓，任务完即关）。原则不变：**任何时刻打开的工作区 ≤ 场景所需的最小集合，永不六仓同开**。工作区是消耗品，不是固定资产。

### 5.2 AGENTS.md 分层（对齐业界四层模型）

1. **每仓 AGENTS.md 只答「此处」**：本仓是什么/构建验证命令/改代码前必读的本仓机制/本仓红线。目标 ≤150 行，超出下沉本仓 `doc/`。
2. **「此类任务」全部移出 AGENTS.md，进 Skills**：editor-patch 已示范（补丁开发工序是 skill 而非 AGENTS.md 段落）——这是正确方向，推广到全生态。
3. **生态地图唯一**：仓库起源/依赖图/知识索引只在 tools 维护一份，其余仓一行链接。
4. **角色条款**：每仓开头三条——本仓 AI 可改什么 / 禁改什么 / 发现外部缺口去哪写（答：`test_res002/.bgd/doc/生态需求池.md`，游戏侧 AI 除外，基建侧 AI 把缺口直接写进自己仓的下一版需求文档草稿）。
5. 修复实证漂移两处（editor-patch 的 test/ 结构描述、framework 的 src/doc 描述）。

---

## 6. Skills 专题：集中化、自发现与自优化

### 6.1 要不要集中到一个仓库？—— 分层回答

**不要全部集中，按作用域分两层**：

| 层 | 内容 | 存放 | 消费方式 |
|----|------|------|----------|
| 仓库任务技能 | 补丁模块开发、slots 制作、库知识库 | 各仓 `.trae/skills/`（现状） | 该仓工作区内按需触发 |
| 生态游戏开发技能 | 构建验证回路、触编注入、MCP 调试闭环、cgui 开发工序 | **framework 模板 `skills/`**（单一来源） | tools 下发到游戏项目 `.trae/skills/bgd-*/`，Dev Channel 覆盖生效 |

理由：Skills 的价值 = 在任务发生现场被发现。游戏开发技能的任务现场是游戏项目，所以必须能下发到游戏项目；而它描述的是生态能力，单一来源应在框架（生态的游戏开发门面）。**不建议新建第七个仓库**——分发通道（tools 的框架下发 + 三路哈希）已经存在，skills 搭上现成的车即可，这正好也回答了「集中管理」的诉求：集中的是**来源**，不是**位置**。

### 6.2 自发现（不需要新基建）

- Skills 的发现机制已内置：description frontmatter 匹配任务描述后按需加载，不占用常驻上下文——这正是它优于 AGENTS.md 段落的原因。
- 需要做的只有两件小事：① 各仓 AGENTS.md 加一行「本仓可用 skills 见 .trae/skills/」指针（防 AI 不知道存在）；② 游戏项目侧由下发机制保证 `.trae/skills/bgd-*/` 在场。

### 6.3 自优化（回应达尔文.Skills / SkillOpt 想法）

技能进化式优化（自动生成-评估-变异 skill）在业界处于探索期，对你的生态**现阶段不值得建重基建**。轻量替代回路（成本接近于零，收益 80%）：

1. **每个 skill 末尾设「实战记录」区**：AI 用完 skill 若踩坑或偏离，被要求在 skill 文件末尾追加一行日期+教训（skill 自我累积实证，类似 editor-patch 的 test/knowledge 惯例，但更内联）。
2. **定期评审节奏化**：每次对应仓库发版前（你本来就要写版本需求文档），扫一遍该仓 skills 的实战记录区，把验证过的教训并入正文、清空记录区。
3. **质量信号**：skill 被触发但任务失败的会话，在需求池记一条（归属 = 该 skill）——失败驱动修订，而不是靠自动变异。

等你有了几十个 skill、感知到人工评审跟不上时，再引入自动评估（agent-eval 类工具已存在），届时数据（实战记录 + 失败单）也已经攒好了。

---

## 7. 双层 AI 工作流（修订版，回应「业务侧绕法」）

**原则更正：你本人不绕。** v1 的「业务侧临时绕法」只适用于第三方生态用户（他们改不了基建），对你——平台方本人——标准动作是：

```
业务 AI 在 test_res002 卡住
  ├─ 缺口在 framework？→ Dev Channel 已开 → 直接改 framework 本地仓 → update-framework 本地源 → 当场继续
  ├─ 缺口在 MCP/editor-patch？→ 超出一个会话的合理边界 → 需求池记一条（pending）→ 换 WS-Apps 会话处理 → 处理完本地 exe 经 app_dev_paths 当场生效
  └─ 缺口在引擎本体？→ 记入 mini-runtime/editor-patch 的研究队列（doc/research 惯例）
```

Dev Channel 的意义正在于此：**让「改底层」和「继续业务」之间不再有发布回路**。需求池只在「缺口大到值得单开一个会话」时使用，是会话间的接力棒，不是流程负担。

第三方用户的差异待遇：生态用户遇到基建缺口只能绕或等版本——因此凡你为生态做的修复，若修复前用户可能已有绕法，在 CHANGELOG/需求文档中记「兼容性注记」即可，这是发布纪律的一部分，不是你开发流程的一环。

---

## 8. Roadmap v2（低风险 → 高风险）

| 阶段 | 内容 | 涉及 | 风险 | 完成判据 |
|------|------|------|------|----------|
| **0. 零代码止血** | ① 拆 3 工作区、不再六仓同开；② 建立 `生态需求池.md` 单文件；③ dev app 手工复制 exe 覆盖安装目录过渡；④ appsdk 联调启用 `.cargo/config.toml` 的 `[patch.crates-io]`（不提交） | 无代码 | 极低 | 当天可用 |
| **1. Dev Channel：框架** | bgd.json `framework_local_path` + update/init 下载入口短路 + cli.rs 白名单 + veri 项目 CLI 验证 + 文档同提交 | tools | 低（缺省零分叉） | `config set` 后本地改框架 → update → 游戏项目生效，全程无网络 |
| **2. Dev Channel：应用** | 宿主 setting `app_dev_paths` + 三处启动入口查表 + 启动前先 `--quit` 运行实例 | tools | 低 | 本地 build 的 app 被宿主直接拉起，升级不覆盖 dev 产物 |
| **3. 知识归位与去漂移** | ① cgui 文档群回迁 framework `libs/doc/`；② 双份排障笔记合一；③ TNND/UPAK 互链；④ 修两处 AGENTS.md 漂移；⑤ tools 建生态知识地图 + `doc/adr/`；⑥ tools 建 `doc/BOOTSTRAP.md` 开荒清单（第 9.3 节） | framework/tools/editor-patch/mini-runtime | 低（纯文档，但触及 4 仓） | 每个知识域有唯一 canonical，生态地图可一键跳转 |
| **4. Skills 下发通道** | framework 模板新增 `skills/` + tools 构建/初始化时同步到项目 `.trae/skills/bgd-*/`（复用 sync_agents_md 模式）+ 迁移 test_res002 散落技能/工序 | tools/framework | 中（新增一条下发管线，有三路哈希先例可循） | 新项目 init 后即获得生态技能；本地改 skill 经 Dev Channel 生效 |
| **5. 可视化与自动化（可选）** | GUI 的 Dev Channel 徽标（framework「本地覆盖中」/app「开发中」）；需求池消费进版本规划的惯例写进各仓 AGENTS.md | tools | 低 | 一眼可辨当前通道状态 |

回退保证不变：阶段 1/2 删键即回 Release Channel；阶段 3/4 是纯增量。**阶段 1+2 落地即达成你的核心诉求：本地改完编译即生效，业务侧永不绕路。**

---

## 9. 换机/多机工作场景（Dev Channel 的可移植性）

设计原则：**生态资产尽量跟着 git 走，机器态收敛到最小集合，且机器路径绝不进 CI 可见文件。**

### 9.1 资产盘点：换机时什么自动跟着走、什么要重建

**跟着 git 走（换机零成本）**：全部仓库源码、知识库（doc/research）、skills、需求池（test_res002 git）、bgd.json、app.json、registry.json。框架三路基准 `.framework_state.json` 虽被 gitignore，但缺失时现有逻辑会按 `framework_version` 下载旧 tag 自动重建（[project.rs](file:///d:/sce_online/Res/maps/bgd_sce_tools/src-tauri/src/project.rs) L412）——自愈，无需处理。

**机器本地（需重建，每项一步）**：

| 资产 | 位置 | 重建动作 |
|------|------|---------|
| github_token | Windows 凭据管理器（per-machine，不落盘） | 宿主设置页重填一次 |
| 宿主 settings（proxy 等） | 宿主 exe 旁 settings.json | 手工重设或复制 |
| `framework_local_path` | 项目 bgd.json | 一条 `config set` |
| `app_dev_paths` | 宿主 setting | 一条 `setting set` |
| 已安装应用 `apps/<id>/` | 宿主 exe 旁 | 随宿主目录整体复制，或应用市场重装 |
| 编辑器补丁状态 | 编辑器安装目录 | 重跑「应用补丁」（设计即幂等可重放） |
| 工具链 | Rust stable / Node 22 / dotnet 9（C# 桥构建前置）/ 代理 | 按开荒清单（9.3） |

### 9.2 路径可移植性决策

1. **约定生态仓库固定平级布局**（回归最初 `D:/devproject/` 设想）。`framework_local_path` 尽量写相对路径——`config.rs::abs`（[config.rs](file:///d:/sce_online/Res/maps/bgd_sce_tools/src-tauri/src/config.rs) L174）已支持相对项目根解析。但游戏项目必须住 `SCE Projects` 目录，与生态仓库跨盘时相对路径不可行 → 接受绝对路径。
2. **`framework_local_path` 定性为机器态值**：随 bgd.json 提交无妨（团队只有你，泄露本机路径无伤），换机后一条 CLI 更新；第三方用户根本不会有这个键（缺省 = Release Channel）。
3. **禁令**：机器路径不进任何 CI 可见文件——`.cargo/config.toml` 的 `[patch.crates-io]` 不提交（阶段 0 既定约定）；bgd_default.json 内建默认永不包含该键。

### 9.3 开荒清单（BOOTSTRAP）

在 tools 仓建 `doc/BOOTSTRAP.md`（阶段 3 随知识归位一起做），内容固定为可核对清单：① 6 仓 clone 到平级目录（需先配 token）；② 工具链版本表；③ token 填入；④ 两条 Dev Channel 命令；⑤ 验证闭环（veri 项目 `build` + 一个 app 经 `app_dev_paths` 拉起）。远期可选 `bgd_sce_tools doctor` CLI 自动体检（repos 在场/工具链/token/Dev Channel 键有效性），列入阶段 5。

---

## 10. 结论速览

1. **你的处境有业界标准答案**：Rust `[patch]` / Go `go.work` / vcpkg overlay 的共同模式 = 覆盖只改源解析、声明收本地配置、缺省即生产。v1 方案方向正确，本版统一为「**Dev Channel**」：framework（bgd.json 键）、apps（宿主 setting）、skills（搭框架下发便车）三类产物同一概念。
2. **生态拓扑是树不是网**：test_res002 是根与目的，其余六仓是长出来的供给链。真实场景矩阵证明无需六仓同开，3 个工作区（WS-Game / WS-Delivery / WS-Apps）覆盖全部日常，S3 跨界 lane 用临时任务工作区。
3. **知识资产有实证错位**：cgui 知识困在游戏项目、逆向知识两仓割裂、双份笔记并存、两处 AGENTS.md 与磁盘漂移——第 4 章给出归属矩阵与处置，核心规则「跨仓只链接不复述」。
4. **需求治理做减法**：废除模板化需求单组，改单文件需求池 + 复用既有按版本需求文档惯例。
5. **Skills 分层集中**：仓库技能留仓内，生态游戏开发技能集中 framework 经 tools 下发（复用 sync_agents_md 模式）；自发现零新基建；自优化用「实战记录区 + 发版前评审」轻回路，暂缓达尔文式重基建。
6. **你本人不绕路**：Dev Channel 让改底层零回路；绕法与兼容性注记只是对生态用户的发布纪律。

---

## 附：参考来源（业界调研）

- [Cargo - Overriding Dependencies（[patch] 官方文档）](https://doc.rust-lang.org/cargo/reference/overriding-dependencies.html)
- [How to use Cargo patch for dependency overrides - rustfaq](https://www.rustfaq.org/en/how-to-use-cargo-patch-for-dependency-overrides/)
- [AGENTS.md vs .cursorrules vs Claude Skills: 2026 Comparison](https://blog.buildbetter.ai/agents-md-vs-cursorrules-vs-claude-skills-2026-comparison/)
- [AGENTS.md vs CLAUDE.md vs skills: what goes where - SkillProof](https://skillproof.dev/blog/agents-md-vs-claude-skills)
- [下一代智能代理架构：Agent Skills 与 AGENTS.md 的深度技术解析](https://blog.csdn.net/m0_63309778/article/details/156149439)
- [Hefrock/agent-skills（个人 skills 仓库与 wiki 知识治理实践样例）](https://github.com/Hefrock/agent-skills)
