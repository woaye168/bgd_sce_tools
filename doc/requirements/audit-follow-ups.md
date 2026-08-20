# BGD SCE 技术生态评估 — 后续改进需求文档

> 创建日期：2026-08-20
> **与研究报告的对应关系**：本需求文档从 [05-问题清单与改进建议](../research/05-问题清单与改进建议.md)（编号 P-001…P-092）提炼可落地的改进条目；05 又逐条汇总自 [00](../research/00-总览与架构评估.md)/[01](../research/01-宿主工具评估.md)/[02](../research/02-Lua框架评估.md)/[03](../research/03-应用生态评估.md)/[04](../research/04-跨仓库一致性与流程评估.md) 五份研究报告。每条需求的「背景」字段注明对应问题编号，可沿 P 编号回溯至原始报告的完整位置/现象/影响记录。纯观察性结论（如框架 §3 缺点清单中无修复指向的机制取舍）不收录。
>
> 优先级定义：**P0** = 确定性故障 + 凭证泄露（立即）；**P1** = 近期（下一版发布前）；**P2** = 中期；**P3** = 远期。

---

## P0 — 立即（6 条）

### RQ-01 吊销并清除框架模板中的明文 API key
- **背景**：P-004（02-P1；04 F-H2）
- **问题描述**：`template/.bgd/src/client/config.lua` L6/L9 明文提交真实 kimi/glm API key，随模板分发到每个新项目并进入各自 git 历史。
- **建议方案**：① 立即到对应平台吊销两个 key 并轮换；② 模板改为占位符 `''` 并加注释指引；③ 排查 framework git 历史与其他仓库是否残留。
- **优先级**：P0　**涉及仓库**：bgd_sce_framework　**影响范围**：全部新项目模板 + 已分发项目
- **验收标准**：模板中无任何真实 key；平台侧旧 key 已失效（调用返回 401）；Grep 五仓库无该 key 字符串残留。

### RQ-02 修复自我更新下载 URL 缺 owner 导致的必然 404
- **背景**：P-001（01-H1；04 §2.2 亲读确认）
- **问题描述**：`updater.rs:71` 用 `env!("CARGO_PKG_NAME")` 拼出 `repos/bgd_sce_tools/releases/latest`（缺 owner），与同文件 `SELF_REPO` 常量不一致，「下载安装包」步骤必然 404。
- **建议方案**：改用 `SELF_REPO` 常量拼 URL。
- **优先级**：P0　**涉及仓库**：bgd_sce_tools　**影响范围**：所有 GUI 用户自我更新路径
- **验收标准**：CLI/GUI 触发自我更新时，存在新版本场景下安装包可成功下载（可在测试仓库或本地 mock 验证 URL 形态为 `repos/woaye168/bgd_sce_tools/...`）。

### RQ-03 清理 CLI 死命令消除 unreachable panic
- **背景**：P-002（01-H2 / 01-D3）
- **问题描述**：`cli.rs:211-226` CMDS 白名单含 editor/logs/mcp 三个从 editor-patch 复制来的死入口，match 无分支落 `unreachable!()`，触发进程 panic。
- **建议方案**：从 CMDS 移除三项（推荐）；如需保留则改为返回错误码 2 的明确报错。
- **优先级**：P0　**涉及仓库**：bgd_sce_tools　**影响范围**：CLI 使用者
- **验收标准**：`bgd_sce_tools editor` 不再 panic，提示未知命令并以非 0 码干净退出；USAGE 与 CMDS 一致。

### RQ-04 修复 exception.lua throw 必然崩溃
- **背景**：P-005（02-P2）
- **问题描述**：`exception.lua:136` 调用未定义的 `instance_of`（class.lua 已注释 `_G.instance_of`，本文件只提取 `.class`），`throw(异常实例)` 路径必然「attempt to call a nil value」。
- **建议方案**：L3 改为 `local class_api = require('libs.common.api.class'); local class = class_api.class; local instance_of = class_api.instance_of`。
- **优先级**：P0　**涉及仓库**：bgd_sce_framework　**影响范围**：所有使用 throw(异常实例) 的游戏代码
- **验收标准**：真实项目构建后，服务端/客户端执行 `throw(Exception 实例)` 不崩溃，previous 链正常。

### RQ-05 修复 deque.lua log.erro 拼写崩溃
- **背景**：P-006（02-P3）
- **问题描述**：`deque.lua` L18/L26 `log.erro(...)` 拼写错误，关闭后 push 必然崩溃；且报错后未 return 仍写入，与自身文档矛盾。
- **建议方案**：修正拼写为 `log.error`，报错后 `return`（或改 `error()` fail-fast，二选一并同步文档）。
- **优先级**：P0　**涉及仓库**：bgd_sce_framework　**影响范围**：所有使用 deque/queue 关闭语义的代码
- **验收标准**：真实项目构建后，对已 close 的队列 push 不崩溃且元素不写入。

### RQ-06 统一宿主/ appsdk / editor-patch 的单实例信号前缀
- **背景**：P-037（04 F-M1，由 03-L12 升级为实际缺陷）
- **问题描述**：宿主安装重命名 exe 为 `<id>.exe` → appsdk `default_si_prefix` 按 exe 名推导为 `editor-patch` → editor-patch `cli.rs:77` 硬编码 `"sce_app_editor-patch"`，导致宿主切项目 notify 后运行中的 GUI 收不到 refresh 事件。
- **建议方案**：appsdk 把 `default_si_prefix` 提为 pub，editor-patch cli.rs 改用同一推导（或宿主保留 asset 原 exe 名）；同时把该命名契约写入 appsdk 文档。
- **优先级**：P0　**涉及仓库**：bgd_sce_appsdk + sce_app_editor-patch（+ bgd_sce_tools 联动验证）　**影响范围**：宿主切项目联动链
- **验收标准**：宿主安装后启动 editor-patch，执行 `<exe> notify project_path=<新路径>`，GUI 界面项目栏实时刷新；appsdk 发新版后 editor-patch 依赖升级。

---

## P1 — 近期（12 条）

### RQ-07 github_token 迁入 Windows 凭据管理器
- **背景**：P-003（01-H3；04 F-H2）
- **问题描述**：token 明文存 `%APPDATA%/com.bgd.sce-tools/settings.json`，同用户任意进程可读，且是四个私有仓库全链路唯一凭证。
- **建议方案**（2026-08-20 修订，替代原 DPAPI 方案——DPAPI 密文仍留文件中且与凭据管理器防护面相同，无额外收益）：用 `keyring` crate（Windows 走 wincred 凭据管理器）存储 token；`AppSettings.github_token` 加 `#[serde(skip)]` 永不落盘；`load_settings` 做旧明文自动迁移 scrub；消费方零改动。残余风险（同用户针对性进程 CredRead）用 PAT 最小权限 + 泄露即轮换兜底并写入文档。**详细实施规格见 `0.8.0.md` W9-3。**
- **优先级**：P1　**涉及仓库**：bgd_sce_tools　**影响范围**：设置读写两处
- **验收标准**：settings.json 中无 token；凭据管理器可见 `bgd_sce_tools/github_token` 条目；GUI/CLI 各网络功能正常；旧明文配置自动迁移。

### RQ-08 统一框架版本号与内容来源
- **背景**：P-008（04 F-H1；00 A1）
- **问题描述**：版本号取 releases/latest tag、内容取 main 快照、基准重建取 tag 快照，三来源双轨；基准重建时本地未改文件被误判修改 → 大面积假冲突。
- **建议方案**：统一为「发版即打 tag、下载即取 tag 快照（codeload refs/tags/<tag>）」，main 快照仅保留给 `init --force` 等显式场景；或在 state 文件同时记录 main commit sha 作辅助判定。
- **优先级**：P1　**涉及仓库**：bgd_sce_tools（+ framework 发版约定）　**影响范围**：init/update-framework 全链路
- **验收标准**：删除 `.framework_state.json` 后执行 update-framework，未本地修改的文件不产生冲突/kept_local；记录的 framework_version 与实际内容同源。**必须在下次框架发版前完成**。

### RQ-09 建立产物 sha256 校验链
- **背景**：P-011（01-M3；04 F-M3）
- **问题描述**：自我更新 NSIS 与应用 exe 下载即执行/写盘，零完整性校验。
- **建议方案**：三个发版仓库（tools + 两应用）release.yml 同步产出 `sha256.txt` asset；宿主 updater/apps 安装前下载校验；editor-patch 桥 dll 随 exe 同链校验。
- **优先级**：P1　**涉及仓库**：bgd_sce_tools + sce_app_visual-injector + sce_app_editor-patch　**影响范围**：安装/升级/自我更新
- **验收标准**：篡改下载内容后宿主拒绝执行并明确报错；正常路径安装不受影响。

### RQ-10 registry app.id 字符校验
- **背景**：P-010（01-M2）
- **问题描述**：远程 registry 的 `app.id` 未校验直接拼 `apps_root().join(id)`，恶意条目可路径穿越。
- **建议方案**：安装/卸载/启动前校验 `^[a-zA-Z0-9_-]+$`，不匹配即拒绝并报错。
- **优先级**：P1　**涉及仓库**：bgd_sce_tools　**影响范围**：应用市场安装/卸载/启动
- **验收标准**：构造 `id: "../x"` 的 registry 条目，安装被拒绝且 apps/ 目录外无文件写入。

### RQ-11 CLI watch 补齐 watch 状态文件契约
- **背景**：P-009（01-M1 / 01-D4；04 F-M5）
- **问题描述**：CLI watch 不写 `.watch_state.json`，check-watch 完全依赖该文件，CLI 监听期间必误报「未监听」。
- **建议方案**：CLI watch 分支启动后写状态文件（含 PID + started_at）、正常退出时删除。
- **优先级**：P1　**涉及仓库**：bgd_sce_tools　**影响范围**：CLI watch / check-watch
- **验收标准**：CLI watch 运行期间另开终端 `check-watch` 返回「监听中」；Ctrl+C 停止后返回「未监听」。

### RQ-12 配置 JSON 解析失败显式报错
- **背景**：P-013（01-M5）
- **问题描述**：bgd_default.json / bgd.json 解析失败被 `if let Ok` 静默吞掉按空对象继续。
- **建议方案**：解析失败返回带文件路径与行号的错误，GUI/CLI 均透传展示。
- **优先级**：P1　**涉及仓库**：bgd_sce_tools　**影响范围**：config 读写
- **验收标准**：人为制造 bgd.json 语法错误后执行 build，报错明确指出文件与原因，而非静默按默认值构建。

### RQ-13 write_atomic 实现真原子替换
- **背景**：P-031（03-M1）
- **问题描述**：`crypto.rs:54-61` 的 write_atomic 实为「写临时→删原→rename」，删与改名间存在原文件缺失窗口，与安全红线表述有差距。
- **建议方案**：Windows 上用 MoveFileExW(MOVEFILE_REPLACE_EXISTING)（或确认平台语义的等价 API）实现真原子替换；同步修正注释。
- **优先级**：P1　**涉及仓库**：sce_app_editor-patch　**影响范围**：全部库文件写盘路径
- **验收标准**：`cargo test` 通过；写盘过程任意时刻目标路径要么旧内容要么新内容（可用并发读写测试佐证）；注释与实现一致。

### RQ-14 修复 strip_json_comments 的 UTF-8 缺陷
- **背景**：P-032（03-M2）
- **问题描述**：`locate.rs:178-224` 逐字节 `as char` push，非 ASCII 字节被重编码，中文路径下 tsconfig typeRoots 解析后路径损坏 → 定位失败且报错指向错误方向。
- **建议方案**：按 `char_indices` 遍历处理注释，或先在字节层剥离注释再整体 `String::from_utf8`。
- **优先级**：P1　**涉及仓库**：sce_app_editor-patch　**影响范围**：编辑器定位链
- **验收标准**：含中文路径的项目可完成定位链四步解析；现有集成测试通过。

### RQ-15 panic hook 落盘路径去硬编码
- **背景**：P-033（03-M3）
- **问题描述**：`main.rs:31-36` panic 落盘硬编码 `C:/Users/woaye/AppData/Local/Temp/ep_panic.txt`，他机目录不存在 → panic 无留痕。
- **建议方案**：改 `std::env::temp_dir()` 或 exe 旁目录。
- **优先级**：P1　**涉及仓库**：sce_app_editor-patch　**影响范围**：崩溃诊断
- **验收标准**：在非 woaye 用户目录下人为触发 panic，panic 文件成功落盘且含回溯信息。

### RQ-16 pie_capture 非 xdeditor-169 版本兜底
- **背景**：P-035（03-M5；00 A4）
- **问题描述**：pie_capture 的行为主体 slot 仅 xdeditor 169 存在，第三级运行时注入只处理入口+isolation，非 169 版本拍照修复静默不生效。
- **建议方案**：运行时注入第三级补 pie_capture 锚点（make_pie_slot 已有模板）；兜底方案：模块启用时检测 slot 缺失并在 UI 明确提示「当前编辑器版本不支持」。
- **优先级**：P1　**涉及仓库**：sce_app_editor-patch　**影响范围**：非 169 版本用户
- **验收标准**：在非 169 版本 xdeditor 上启用 pie_capture，要么拍照修复生效，要么 UI 明确提示不支持——不允许静默失效。

### RQ-17 MCP 桥本机访问控制与威胁模型明示
- **背景**：P-036（03-M6；00 A8）
- **问题描述**：HttpListener 仅绑 127.0.0.1 但零鉴权，write 级能力本机任意进程可直接调用。
- **建议方案**：port 文件旁落随机 token 文件，请求头校验（低成本）；同时在 AGENTS/文档明示本机威胁模型。
- **优先级**：P1　**涉及仓库**：sce_app_editor-patch　**影响范围**：C# 桥 + Rust 桥客户端
- **验收标准**：无 token 的请求被 401 拒绝；合法链路（mcp stdio 透传）不受影响；文档含威胁模型段落。

### RQ-18 游戏骨架三模块重写为 local M + return M
- **背景**：P-007（02-P4 / 02-D1）
- **问题描述**：PlayerManager/MonsterManager 全局写法且无 return（require 返回 true），DataManager 全局写法，带头违反框架自身铁律。
- **建议方案**：三文件重写为 `local M = {} ... return M`，GameServer 等消费方改显式 require；同步检查 src_template 一致性。
- **优先级**：P1　**涉及仓库**：bgd_sce_framework　**影响范围**：全部新项目的第一眼代码
- **验收标准**：骨架三模块均 `return M`；真实项目 init + build 后示例玩法（F 键攻击木桩）功能不变；EmmyLua 跨文件跳转恢复。

---

## P2 — 中期（13 条）

### RQ-19 CI 门禁补齐
- **背景**：P-039（04 F-M4；00 A9）
- **问题描述**：仅 tools 有 CI；appsdk publish 连 `cargo test` 都不跑；editor-patch 高质量集成测试无 CI 承载；framework 无任何校验。
- **建议方案**：① appsdk publish.yml 加 `cargo test` 门禁；② editor-patch 加 ci.yml 跑 `cargo test`（测试不碰真实编辑器，可直接上 CI）；③ framework 加 Lua 语法/结构静态检查 workflow。
- **优先级**：P2　**涉及仓库**：appsdk / editor-patch / framework　**影响范围**：发版质量保障
- **验收标准**：三仓库 main push 触发对应检查，失败阻塞 publish/release。

### RQ-20 版本号注入机制统一约定
- **背景**：P-038（04 F-M2）
- **问题描述**：四仓库注入机制各自为政；editor-patch 要求 dotnet `-p:Version` 与 Rust 版本强一致仅靠注释维系。
- **建议方案**：收敛为统一约定（全部 0.0.0-dev + tag 注入）并固化进 appsdk 脚手架模板；editor-patch 增加构建期断言两版本一致。**依赖 RQ-08 的来源统一结论后实施**。
- **优先级**：P2　**涉及仓库**：四 Rust 仓库　**影响范围**：全部发版链
- **验收标准**：任一仓库发版产物版本号、元数据、deps 登记键三者一致；脚手架新应用开箱即符合约定。

### RQ-21 框架文档体系全量刷新
- **背景**：P-018 / P-019 / P-020 / P-026 / P-029 / P-063（02-P5/P6/P7/P13/P16/P27；02-D2/D3/D4）
- **问题描述**：doc/api 七篇全部停留在前代 `#common.sce_base.*` 路径；下发的 libs/README 仍是旧 asset 机制；src/README 引用不存在目录；注解/签名文档多处与实现不符。
- **建议方案**：一次集中提交：doc/api 全量改写为 `bgd_api.common.<模块>` 双写法；libs/README 与 src/README 按 res 五类资源机制重写；清理 libs_excludes 残留；修正 exception/deque/event_deque 文档签名。
- **优先级**：P2　**涉及仓库**：bgd_sce_framework　**影响范围**：随框架下发到所有项目
- **验收标准**：按文档示例代码在真实项目可运行；Grep 无 `#common.sce_base`、`asset_target` 残留。**必须在下次框架下发前完成**。

### RQ-22 require 改写与软覆盖的防护检测
- **背景**：P-040（00 A2；02 §3 缺点 2/4）
- **问题描述**：文本级正则改写 + entrance 顺序保证软覆盖，均无检测，破坏即静默失效。
- **建议方案**：构建后对产物做残留检测（产物中不应存在未改写的 `require('src.` / `require('libs.`）并警告；入口合并时断言 libs 先于 src 的顺序，违例明确报错。
- **优先级**：P2　**涉及仓库**：bgd_sce_tools（+ framework 约定）　**影响范围**：构建产物正确性
- **验收标准**：人为颠倒 entrance 顺序或构造未改写样例，构建报明确错误而非静默产出坏产物。

### RQ-23 builder.rs 按职责拆分
- **背景**：P-014（01-M6；00 A7）
- **问题描述**：1030 行单文件承担 10+ 职责。
- **建议方案**：拆 watch.rs / res.rs / merge.rs 等子模块，builder.rs 仅留构建主流程编排。**建议在 RQ-22 之前完成**。
- **优先级**：P2　**涉及仓库**：bgd_sce_tools　**影响范围**：构建核心可维护性
- **验收标准**：`cargo check` 通过；CLI 在真实项目全量/增量构建产物与拆分前一致；单文件均 < 500 行。

### RQ-24 editor-patch 大文件拆分
- **背景**：P-034（03-M4 / 03-D1）
- **问题描述**：main.rs 804 行、kernel.rs 861 行，超本仓库「500 行必拆」约定。
- **建议方案**：main.rs 拆 ui_kernel/ui_patches/ui_settings；kernel.rs 的 `mod slots` 独立为 slots.rs。
- **优先级**：P2　**涉及仓库**：sce_app_editor-patch　**影响范围**：可维护性
- **验收标准**：`cargo test` 全绿；单文件 ≤ 500 行；功能不变。

### RQ-25 read_db 死代码清理与索引策略修正
- **背景**：P-023 / P-024（02-P10/P11）
- **问题描述**：智能缓存体系完整死代码；默认索引按首条记录建全字段索引。
- **建议方案**：删除缓存死代码并修正 get_stats（或补实现，二选一）；默认只建 id 唯一索引，其余字段改显式 add_index。
- **优先级**：P2　**涉及仓库**：bgd_sce_framework　**影响范围**：read_db 使用方
- **验收标准**：异构数据表查询结果正确；get_stats 无虚报；真实项目构建通过。

### RQ-26 框架基础库 fail-fast 改造与死代码补全
- **背景**：P-022 / P-027 / P-028（02-P9/P14/P15）
- **问题描述**：co.lua 主线程误用返回原函数；class.lua 父类缺失创建孤儿类；默认异常处理器未导出。
- **建议方案**：前两处改 `error()` fail-fast（加载期错误理应暴露）；异常处理器补导出或删除（补导出优先，机制已具雏形）。
- **优先级**：P2　**涉及仓库**：bgd_sce_framework　**影响范围**：基础库使用方
- **验收标准**：误用场景在加载期即报明确错误；导出表与文档一致。

### RQ-27 GameClient 示例反模式修正
- **背景**：P-025（02-P12）
- **问题描述**：按键回调内重复注册、每广播新建面板永不释放、读私有字段。
- **建议方案**：forward_event_register 移模块顶层一次注册；示例 UI 改单次创建复用（或注释说明仅调试）；uid 由服务端从事件回传。
- **优先级**：P2　**涉及仓库**：bgd_sce_framework　**影响范围**：新用户教学路径
- **验收标准**：连续按 F 键 100 次无面板堆积、无重复注册；示例功能不变。

### RQ-28 emmyrc 引擎库路径参数化
- **背景**：P-021（02-P8）
- **问题描述**：libs/.emmyrc.json 硬编码开发者本机绝对路径且版本钉死 195/48。
- **建议方案**：改相对/可发现路径，或由 tools 在 init 时按本机编辑器定位生成（复用 editor-patch locate 链思路），版本号参数化。
- **优先级**：P2　**涉及仓库**：bgd_sce_framework + bgd_sce_tools　**影响范围**：全部项目的 EmmyLua 索引
- **验收标准**：在非 D:/sce_open 机器上 init 后 EmmyLua 引擎 API 索引可用；编辑器升级后无需手改模板。

### RQ-29 WebView 最小 CSP
- **背景**：P-016（01-M8）
- **问题描述**：tauri.conf.json `csp: null`，远程下发文本进 React 渲染无防线。
- **建议方案**：设置最小 CSP（`default-src 'self'` 等），验证不影响现有资源加载。
- **优先级**：P2　**涉及仓库**：bgd_sce_tools　**影响范围**：前端安全面
- **验收标准**：GUI 全部页面功能正常；devtools 无 CSP 违例误报。

### RQ-30 前后端配置契约清理
- **背景**：P-017 / P-053（01 §2.2）
- **问题描述**：types.ts 残留 3 个幻影字段保存时被静默丢弃；3 个真实字段 UI 不可编辑。
- **建议方案**：删除幻影字段（asset_target 等）；表单补 enable_build_log/excludes 编辑或在 UI 注明 CLI 路径。
- **优先级**：P2　**涉及仓库**：bgd_sce_tools　**影响范围**：设置页
- **验收标准**：types.ts 与 config.rs 字段一一对应；保存/读取往返无字段丢失。

### RQ-31 AGENTS/注释偏差集中核销
- **背景**：P-069 / P-073 / P-074 / P-084~P-092（01-D1/D2/D5/D6；03-D2/D3/D4/D5/D6/D10；04 X-D1）
- **问题描述**：五仓库共 13 条纯文档偏差 + 3 条注释过期，呈「机制演进 → 文档留在上一代」统一模式。
- **建议方案**：一次跨仓库集中提交逐条核销；visual-injector 注入规范文档（P-088）单独先行（它是注入逻辑准绳）；删两仓库 clap 冗余依赖；framework 删不存在的 release.yml 条目或补分组文件。
- **优先级**：P2　**涉及仓库**：全部五仓库　**影响范围**：文档可信度
- **验收标准**：23 条偏差台账全部核销（含已并入问题条目的 14 条随对应代码修复自然消除）；后续可用 PR 模板提醒「文档同提交」。

---

## P3 — 远期（8 条）

### RQ-32 visual-injector core.rs 拆分与 JSON id 稳定化
- **背景**：P-076 / P-077（03-L14/L15）
- **建议方案**：拆 parser / json_gen / sce_init 三模块；id 改「模块+函数」名稳定哈希（先实测确认触编是否按 id 引用，若按 name 引用则文档注明即可）。
- **优先级**：P3　**涉及仓库**：sce_app_visual-injector
- **验收标准**：重复注册同一模块 id 不变；`cargo check` 通过，生成 JSON 与拆分前一致。

### RQ-33 appsdk 体验改进批
- **背景**：P-064 / P-065 / P-066 / P-067（03-L1/L2/L3/L4）
- **建议方案**：删 signal_show_self 死代码并修正注释；窗口居中改读主屏尺寸；日志时间戳本地化（civil_from_days 上移 SDK）；配置写入加文件锁或临时文件替换。
- **优先级**：P3　**涉及仓库**：bgd_sce_appsdk（两应用随依赖升级受益）
- **验收标准**：`cargo test` 通过；日志含可读本地时间；非常见分辨率窗口居中。

### RQ-34 tools 低级清理批
- **背景**：P-041~P-052（01-L1~L12）
- **建议方案**：删同步版死代码；正则静态化（P-012 可并入本批）；日志句柄缓存；PATH 写入标记跳过；LogPanel key/滚动修正；toggleAutoStart 回滚；pid_alive 校进程名；去重循环兜底扫描；注释清理。
- **优先级**：P3　**涉及仓库**：bgd_sce_tools
- **验收标准**：`cargo check` + `npx tsc` 通过；CLI 真实项目构建/监听回归正常。

### RQ-35 editor-patch 低级清理批
- **背景**：P-070 / P-071 / P-072 / P-075 / P-078 / P-079 / P-080（03-L7/L8/L9/L13/L16/L17/L18）
- **建议方案**：解密实现复用 crypto.rs；find_editor_pid 与日期算法去重；engine_root 回退校验；manifest 改 serde_json；ReadBodyAsync 加 4MB 上限；snapshot_handler.lua 移位或加注。
- **优先级**：P3　**涉及仓库**：sce_app_editor-patch
- **验收标准**：`cargo test` 全绿；中文路径回退场景明确报错。

### RQ-36 发布恢复手册（降 bus factor）
- **背景**：P-083（04 F-L3；00 A9）
- **建议方案**：离线文档记录 token 清单（CARGO_REGISTRY_TOKEN / PAT 权限范围）、ci/refs 引用程序集来源、五仓库发版步骤、故障恢复路径；存放于仓库外安全位置。
- **优先级**：P3　**涉及仓库**：流程（不落公开仓库）
- **验收标准**：手册存在且按手册可在干净环境完成一次完整发版演练。

### RQ-37 网络代理错误提示与差异文档化
- **背景**：P-081（04 F-L1）
- **建议方案**：net.rs 网络错误信息附「当前代理=x」；三份 AGENTS 注明 CI 直连与本地代理差异。
- **优先级**：P3　**涉及仓库**：bgd_sce_tools（+ 文档）
- **验收标准**：代理挂掉时错误信息可直接定位到代理层。

### RQ-38 app-release.json schema 版本与合成加固
- **背景**：P-082（04 F-L2）
- **建议方案**：app-release.json 加 `schema: 1`；两应用 release.yml 合成改对象合并而非 Add-Member。
- **优先级**：P3　**涉及仓库**：sce_app_visual-injector + sce_app_editor-patch + bgd_sce_tools
- **验收标准**：app.json 自带同名字段时 CI 不再报错；宿主端按 schema 字段兼容解析。

### RQ-39 init 下载进度与异步化
- **背景**：P-052（01-L12）
- **建议方案**：init_project 复用 `download_framework_template_async` + 进度事件，与更新路径行为一致。
- **优先级**：P3　**涉及仓库**：bgd_sce_tools
- **验收标准**：GUI 初始化过程有实时进度展示；CLI init 行为不变。

---

## 统计

| 优先级 | 条数 | 编号 |
| --- | --- | --- |
| P0（立即） | 6 | RQ-01 ~ RQ-06 |
| P1（近期） | 12 | RQ-07 ~ RQ-18 |
| P2（中期） | 13 | RQ-19 ~ RQ-31 |
| P3（远期） | 8 | RQ-32 ~ RQ-39 |
| **合计** | **39** | — |

> 未转化为需求的条目说明：问题清单中的纯观察性结论（如 read_db `with()` 的 `[1]` 歧义、错误处理风格不统一、协程基建绑定引擎等）按收录规则不转需求；低级问题中可被同类批次覆盖者已并入 RQ-32~RQ-35 的清理批；P-008 以外的批次依赖关系见 05 报告 §4 路线图。
