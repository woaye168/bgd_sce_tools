# 02 — bgd_sce_framework Lua 框架源码级深度审查报告

> 审查对象：`d:\sce_online\Res\maps\bgd_sce_framework`（`template/.bgd/` 全部模板代码）
> 审查方式：逐文件 Read 全量阅读（libs 全部 Lua/配置文件、src 骨架全部文件、types 与 doc/api 抽样）
> 审查日期：2026-08-20

---

## 1. 框架结构清单

### 1.1 libs/（框架所有，工具下发）

| 文件 | 行数 | 职责 |
| --- | --- | --- |
| `init.lua` | 11 | init 模板：声明 `_G.bgd_const/bgd_config/bgd_api`，`require('{{target}}.common')` + `require('{{target}}.{{module}}')`（构建时渲染占位符） |
| `bgd_default.json` | 21 | 默认配置下发通道：project_root、双端入口、libs/game 目录与产物目标、`libs_excludes`（types 等） |
| `common/init.lua` | 15 | 公共端装配：config → api 聚合 → `require_folder(const)` → 初始化日志 |
| `common/config.lua` | 7 | `bgd_config.common.libs_info`（name/key/version） |
| `common/api/init.lua` | 12 | AUTO-GENERATED 聚合：挂载 9 个模块到 `bgd_api.common.*` |
| `common/api/class.lua` | 128 | 轻量类系统（Lua/C++ 双模式继承、`instance_of`、全局类名注册表） |
| `common/api/ToSceClass.lua` | 26 | 触编类补丁：注入 `____constructor` 桥接触发编辑器构造 |
| `common/api/co.lua` | 125 | 协程工具：wrap（回调→协程）/async/async_next/sleep/sleep_one_frame |
| `common/api/deque.lua` | 117 | 双端队列 + 单端队列（create_deque/create_queue） |
| `common/api/event_deque.lua` | 183 | 可等待事件队列（回调/协程两种消费、超时、关闭唤醒） |
| `common/api/exception.lua` | 167 | Exception 类（traceback/前置异常链/to_string 缓存）+ to_exception/throw |
| `common/api/json.lua` | 153 | json.encode/decode 透传 + encode_x（限深/防循环的调试序列化器） |
| `common/api/promise.lua` | 223 | promise/multi_promise（三种聚合策略）/as_promise |
| `common/api/read_db.lua` | 700 | 只读数据集 + 链式查询（索引/where/range/sort/limit/select/group_by） |
| `common/const/keyboard.lua` | 98 | 键名映射常量，自注册 `bgd_const.keyboard` |
| `server/init.lua` | 68 | 服务端装配 + 引擎生命周期事件注册（绝大多数回调体被注释） |
| `server/config.lua` | 7 | `bgd_config.server.libs_info` |
| `server/api/init.lua` | 3 | 空聚合（`bgd_api.server = {}`，框架侧暂无服务端 API 模块） |
| `server/const/server_event.lua` | 13 | 服务端事件名映射 |
| `client/init.lua` | 42 | 客户端装配 + 生命周期事件注册 |
| `client/config.lua` | 7 | `bgd_config.client.libs_info` |
| `client/api/init.lua` | 4 | 聚合 `bgd_api.client.io` |
| `client/api/io.lua` | 123 | 客户端文件 IO 封装（copy/create_dir/exist_*/read/write/remove/rename） |
| `client/const/client_event.lua` | 12 | 客户端事件名映射 |
| `entrance/client.lua` / `entrance/server.lua` | 3+3 | 入口片段：`require('libs')` + `require('src')`，内容完全相同 |
| `types/`（40 个 .d.lua） | 数千行 | 引擎 + 框架 EmmyLua 声明，不参与构建（在 `libs_excludes` 中） |
| `doc/api/`（7 个 md） | — | 基础库使用文档（**全部使用过时路径，见问题清单**） |

### 1.2 src/（游戏代码骨架）

| 文件 | 行数 | 职责 |
| --- | --- | --- |
| `init.lua` | 6 | 游戏 init 模板（同 libs 结构，`{{target}}.common` + `{{target}}.{{module}}`） |
| `entrance/client.lua` / `entrance/server.lua` | 2+2 | 各一行 log.info 启动日志 |
| `common/init.lua` | 18 | 装配 config → api → SceInit → const → 日志（读 `bgd_config.common.game_info`） |
| `common/config.lua` | 5 | `bgd_config.common.game_info` |
| `common/DataManager.lua` | 17 | 示例配置表查询（**全局变量写法，无本地模块表**） |
| `common/SceInit.lua` | 2 | visual-injector 注入位（空 AUTO-GENERATED） |
| `common/api/init.lua` | 3 | 空聚合（`bgd_api.common = bgd_api.common or {}`，软合并） |
| `common/base/json.lua` | 152 | **与 libs json.lua 近乎完全重复的死代码**（类名 CommonJson2，无任何引用） |
| `common/const/game_common_const_tpl.lua` | 5 | 占位常量 `['a']='A'` |
| `server/init.lua` | 21 | 装配 + `require('src.server.GameServer')` |
| `server/GameServer.lua` | 57 | 示例服务端主入口：连入/断线事件 → PlayerManager，生成测试木桩 |
| `server/CombatSystem.lua` | 50 | 示例战斗：监听 `Req_BasicAttack` → 扣血 → `base.game:ui('Sync_CombatResult')` 广播 |
| `server/MonsterManager.lua` | 13 | 示例怪物管理（**全局写法、无 return**） |
| `server/PlayerManager.lua` | 35 | 示例玩家数据（**全局写法、无 return**） |
| `server/{api/init.lua, config.lua, SceInit.lua, const/...}` | 3/5/2/5 | 空聚合 / game_info / 空注入位 / 占位常量 |
| `client/init.lua` | 21 | 装配 + `require('src.client.GameClient')` |
| `client/GameClient.lua` | 37 | 示例客户端：F 键发 `Req_BasicAttack`，`base.proto.Sync_CombatResult` 收广播 |
| `client/config.lua` | 11 | **含明文真实 API key（高危，见问题 P1）** |
| `client/{api/init.lua, SceInit.lua, const/...}` | 3/2/5 | 空聚合 / 空注入位 / 占位常量 |
| `bgd.json` | 5 | 仅 `framework_version`（空）+ `framework_repo` |

---

## 2. 基础库 API 实现质量评估（逐个模块）

### class.lua（中等质量，有隐患）
- 双模式继承（C++ 对象 `__ctype=1` / Lua 对象 `__ctype=2`）思路完整，`__supper_map` 支持多代 `instance_of`。
- 问题：字符串父类名查不到时仅 `log.error` 后继续创建**无父类的孤儿类**（L17-26），静默降级难排查；`__supper_map` 拼写错误（supper→super）已进入公开 API 表面；全局 `class_name_map` 跨模块隐式耦合，字符串父类对**加载顺序敏感**。
- 大量注释掉的死代码（L89-93、L115-119），其中 `_G.instance_of` 的注销直接导致了 exception.lua 的运行时 Bug（P2）。

### ToSceClass.lua（简单可用）
- 26 行补丁函数，拷贝 `M.New(...)` 产出的字段到触编传入的 self。未拷贝 metatable，实例方法若依赖元表会丢失——当前示例均为普通表，风险低。

### co.lua（中等质量）
- wrap 的「立即回调不 yield」（L52-68）处理正确；`coroutine_resume_with_check` 吞掉 `error_pending_kill` 的设计与引擎配合合理。
- 问题：主线程误用 wrap 时仅 `log.error` 后**返回未包装的原函数**（L39-42），调用方语义静默错误；`sleep`/`sleep_one_frame` 每次调用重新 `wrap(base_wait)`，属无谓开销；模块加载即向引擎全局 `base` 写入 `error_pending_kill`（L19-20），是对引擎全局的侵入式补丁。

### deque.lua（有明确 Bug）
- 区间指针（_front/_back）实现正确，`__len`/close 清理逻辑可用。
- **Bug：L18、L26 `log.erro` 拼写错误**（应为 `log.error`），队列关闭后 push 会触发「调用 nil」崩溃；且报错后**没有 return，仍继续写入**，与自身文档「关闭后不再接受 push」矛盾。
- `pop_back`/`pop_front` 返回 `(ret, nil)` 双值，与 `---@field` 注解的单返回值不一致（轻微）。

### event_deque.lua（质量较好，一处注解错误）
- 回调队列 + 元素队列双队列设计正确；`_push_callback` 超时通过置 nil 槽位 + `called` 闭包防重（L112-130），pop 侧的 `if f then` 能跳过被超时清除的槽位，闭环正确。
- 问题：L137 `event_deque.pop = event_deque._pop_front`，但 L15 注解写 `alias pop_back`——**注解与实现相反**（实现对单端队列 FIFO 语义反而是对的，注解错）。

### exception.lua（有明确 Bug + 死代码）
- **Bug：L136 `throw` 调用了未定义的 `instance_of`**。class.lua 中 `_G.instance_of` 赋值已被注释（L118-119），exception.lua 又只提取了 `.class`（L3），因此 `throw(异常实例)` 路径必然「attempt to call a nil value」崩溃；`throw(非异常)` 走 else 分支不受影响。
- L151-157 `set_default_exception_handler`/`get_default_exception_handler` 定义了但**未加入模块导出表**（L161-165），默认处理器机制是未完成的死代码。
- `throw` 的 rethrow 分支把原异常包进 `msg='rethrow exception'` 的新异常再挂 previous 链，语义怪异（多一层无信息包装）。

### json.lua（可用，定位需明确）
- encode/decode 直接透传引擎 `json.*`；encode_x 是限深 + 防循环 + 逐元素 pcall 的调试序列化器，容错思路好。
- 问题：字符串用 `string.format('%q')` 转义（L92、L126），`%q` 是 **Lua 风格**转义（换行为 `\`+换行），产出不是合法 JSON；数字键 `'%d'` 遇浮点键会报错（靠外层 pcall 兜底）；`math.floor(math.huge)==math.huge` 的分支会对 inf 执行 `%d` 报错。作为调试工具可接受，但命名 encode_x 易被误认为可传输的 JSON。

### promise.lua（质量较好，小问题若干）
- 一次性 set（`try_set` 后 close 事件队列唤醒等待者）闭环正确；`get` 的 proxy_callback 用 `_ready` 区分「就绪唤醒」与「closed/timeout 唤醒」（L36-44）处理细致。
- 问题：`as_promise` 用 `local _, ret = xpcall(...)` **丢失多返回值**（L203-205）；`multi_promise` 的 `---@field` 注解全部误写为 `self: promise`（L134-136）；`co_result` 注解称「会抛出异常」，实现实际只 `log.error` 后返回 nil（文档/注解与行为不符）。

### read_db.lua（功能丰富，但有死代码与过度设计）
- 链式查询（where/with/range/sort_by/order_by/limit/select/group_by）以 `_execution_order` 按调用序执行，语义清晰。
- 问题：①「智能缓存」体系（`_sorted_views`/`_cache_hit_count`/`_cleanup_cache`/`generate_cache_key`）是**完整死代码**——缓存从未被写入、清理从未被调用（L91-93、L285-311）；②`_build_default_indexes` 仅依据**首条记录**的字段自动建全字段索引（L174-198），异构数据会得到错误索引，且内存开销大；③`query_chain.sort_fn`/`limit_count`/`filters` 字段被 pipeline 的执行顺序机制架空（冗余字段，误导阅读）；④`with()` 用 `index_result[1]` 区分唯一/非唯一索引结果（L345），若唯一索引的记录本身含 `[1]` 字段会被误判为数组。
- 错误处理风格不统一：部分 error() 抛出（L204、L230），部分返回 `false, '原因'`（L208、L215）。

### keyboard.lua / server_event.lua / client_event.lua（常量表，无问题）
- 自注册 `bgd_const.*` 模式符合约定。双端事件表覆盖不全（服务端缺「断线/重连」等在 init.lua 中用到的事件名），仅作示例性质。

### client/api/io.lua（可用）
- 问题：L75 `log.error(nil, '...')` 首参传 nil，与本文件其他 `log.error(msg)` 调用风格不一致（取决于引擎 log.error 签名，可能输出异常）；`exist_dir`/`exist_file` 把「不存在」记为 error 级日志（L39、L53），语义过重——「检查不存在」是正常结果。

---

## 3. 机制设计优缺点

### 优点
1. **静态 require + 构建期前缀改写**（`libs/README.md` L30-39）：源码即真实路径，EmmyLua 全程可见，是整套框架最有价值的决策。
2. **API 自动注册 + 聚合 init.lua 生成在源码树内**（`src/README.md` L85-96）：新模块丢进 api/ 即获得补全，无需手改注册代码；游戏侧聚合用 `bgd_api.common = bgd_api.common or {}` 软合并（`src/common/api/init.lua` L3），不破坏框架侧注册。
3. **三命名空间分治**（bgd_api/bgd_const/bgd_config）职责清晰，const 自注册、config 挂载、api 聚合三种模式各自一致。
4. **入口合并保序**：entrance 先 `require('libs')` 后 `require('src')`，天然保证软覆盖的「游戏侧后加载」前提。
5. **types/ 排除构建 + EmmyLua 双片段深合并**（`.emmyrc.json`）思路完整，`workspaceRoots: ["./.bgd"]` 避免产物重复索引。
6. **bgd_default.json overlay** 字段精简（目录/产物目标/排除表/入口），边界清楚。

### 缺点与脆弱点
1. **init 模板占位符无校验**（`libs/init.lua` L10-11、`src/init.lua` L5-6）：`{{target}}/{{module}}` 若渲染失败，只在运行期以 `module '{{target}}.common' not found` 的形式报错，无渲染期断言。
2. **软覆盖依赖隐式加载顺序**：「游戏侧后加载」这一关键语义只由 entrance 两行的书写顺序保证，没有任何防护或检测；顺序被改即静默失效。
3. **聚合 init.lua 是生成物却入库**：libs/src 两侧 `api/init.lua` 均标 AUTO-GENERATED 但提交在 git 中，与工具再生成之间可能产生无意义 diff/冲突（尤其在三路哈希更新里会被当普通文件对比）。
4. **静态 require 改写是文本级操作**：框架侧约定「只改写引号内」，但注释或普通字符串中形似 `require('src.xxx')` 的内容是否被误改完全取决于工具实现，框架侧无法防御；裸 `require('src')`/`require('libs')` 需工具特判。
5. **const 自注册依赖引擎全局 `require_folder`**（`libs/common/init.lua` L8）：非标准 Lua，框架与引擎深度耦合，单测/离线验证困难。
6. **异步基建全家桶依赖引擎 `base.next/base.wait/base.error_pending_kill`**（co.lua L14-20）：语义绑定引擎帧驱动，移植性差（对本项目定位可接受，但应有文档说明）。
7. **class 的全局类名注册表**使字符串父类解析对加载顺序敏感，且无重名检测（重定义 silently 覆盖，L95）。

---

## 4. 问题清单

### 高危（4）

| # | 位置 | 现象 | 影响 | 建议 |
| --- | --- | --- | --- | --- |
| P1 | `template/.bgd/src/client/config.lua` L6、L9 | **明文真实 API key 入库**：L9 `api_key = 'sk-kimi-XUDMRNbZXGIK6JdBzMxfCs6ur2eXzchT11Pf6BueMBxFDgvvQhaRyUGoppgOXf9R'`（kimi），L6 注释中还有 glm key | 密钥随模板分发到每个新项目并进入各项目 git 历史；泄露即被滥用计费 | **立即吊销两个 key**；模板改为占位符 `''`；排查 git 历史与其他仓库是否同样残留 |
| P2 | `template/.bgd/libs/common/api/exception.lua` L136 | `throw` 调用未定义的 `instance_of`（class.lua L118-119 已注释掉 `_G.instance_of`，本文件只提取了 `.class`） | `throw(异常实例)` 路径必然运行时崩溃（attempt to call a nil value） | L3 改为同时提取：`local class_api = require('libs.common.api.class'); local class = class_api.class; local instance_of = class_api.instance_of` |
| P3 | `template/.bgd/libs/common/api/deque.lua` L18、L26 | `log.erro(...)` 拼写错误（应为 `log.error`）；且报错后未 return，关闭的队列仍被写入 | 队列关闭后 push → 「调用 nil」崩溃；即便修复拼写，行为也与自身文档「关闭后不再接受 push」矛盾 | 修正拼写并在报错后 `return`（或直接 `error()`  fail-fast） |
| P4 | `template/.bgd/src/server/{PlayerManager,MonsterManager}.lua`、`src/common/DataManager.lua` | 骨架用 `PlayerManager = {}` 全局变量写法；PlayerManager/MonsterManager **连 `return M` 都没有**（require 返回 true，消费全靠全局） | 直接违反框架自身「local M + return M」铁律，模板第一天就教坏用户；全局污染、EmmyLua 跨文件推断弱化 | 三个文件重写为 `local M = {} ... return M`，消费方 `local PlayerManager = require('src.server.PlayerManager')` |

### 中危（13）

| # | 位置 | 现象 | 影响 | 建议 |
| --- | --- | --- | --- | --- |
| P5 | `libs/doc/api/*.md`（全部 7 篇） | 模块路径全部写作过时的 `#common.sce_base.*`（前代框架遗留），README 索引还引用不存在的 `src/common/example/` 示例目录 | 文档完全无法按路径引用，新用户按文档写代码必失败 | 全量改写为 `bgd_api.common.<模块>` 与 `require('libs.common.api.<模块>')` 双写法，删除失效示例引用 |
| P6 | `libs/doc/api/exception.md` L17-18 | 文档记载 `E.make`/`E._make`，实现中它们是 `Exception` 的类方法且**未在模块导出表**（exception.lua L161-165） | 按文档调用 `E.make(...)` 得到 nil | 文档改为 `E.Exception.make(...)` 或在模块表补充导出 |
| P7 | `libs/doc/api/deque.md` L19、L31-32 | 文档 `close(clear_items?, on_clear_item?)` 双参；实现 `close(clean, recursive_clean)` 单清理参数（recursive_clean 是选 pop 方向） | 按文档传 `close(true, fn)` 时 fn 被当成 recursive_clean，清理回调永远不执行 | 文档对齐实现签名，说明 clean 可为 boolean 或函数 |
| P8 | `template/.bgd/libs/.emmyrc.json` L13、L17 | library 硬编码开发者本机绝对路径 `D:/sce_open/res/_m/script/195/script`、`D:/sce_open/res/_m/gameui/48/gameui/ui`，版本钉死 195/48 | 其他机器路径不存在；编辑器升级包版本变化后引擎 API 索引静默失效 | 改为相对/可发现路径（或工具在 init 时按本机编辑器定位生成），版本号参数化 |
| P9 | `libs/common/api/co.lua` L39-42 | 主线程误用 wrap：log.error 后**返回未包装的原函数** | 调用方拿到语义错误的函数继续执行，故障被推迟到更难定位的位置 | 返回一个调用即 error 的占位函数，或直接 error() fail-fast |
| P10 | `libs/common/api/read_db.lua` L91-93、L146-152、L285-311 | 缓存体系（_sorted_views/_cache_hit_count/_cleanup_cache/generate_cache_key）是完整死代码：从未写入、从未调用 | 误导维护者以为有缓存层；get_stats 报告的 cache_size 恒为 0 | 删除死代码或补实现；同时修正 get_stats |
| P11 | `libs/common/api/read_db.lua` L174-198 | `_build_default_indexes` 仅按**首条记录**字段自动建全字段索引 | 异构数据索引错误；大表全字段索引内存浪费 | 默认只建 id 唯一索引，其余字段改为显式 add_index |
| P12 | `template/.bgd/src/client/GameClient.lua` L8-10、L28-34 | ①`base.forward_event_register('Req_BasicAttack')` 在**每次按键回调内**重复注册；②每收到一次战斗广播就 `base.ui.panel{...}` 新建一个 600×100 面板（永不释放）；③读私有字段 `base.local_player()._user_id` | 示例代码教学性误导：面板泄漏、重复注册、依赖私有字段 | forward_event_register 移到模块顶层一次注册；示例 UI 改为注释或单次创建复用；uid 改由服务端从事件回传 |
| P13 | `libs/common/api/event_deque.lua` L15 vs L137 | 注解 `pop ... alias pop_back`，实现 `event_deque.pop = event_deque._pop_front` | 注解与实现相反（实现对 FIFO 语义正确，注解错），EmmyLua 提示误导 | 修正注解为 alias pop_front |
| P14 | `libs/common/api/class.lua` L17-26 | 字符串父类名查不到时仅 log.error，继续创建无父类的孤儿类 | 继承关系静默丢失，实例调用父类方法时 nil 崩溃，根因难查 | error() fail-fast（加载期错误理应暴露） |
| P15 | `libs/common/api/exception.lua` L145-157 | `set/get_default_exception_handler` 定义但未导出，默认处理器机制未完成 | 死代码；外部无法定制默认处理 | 补导出或删除 |
| P16 | `template/.bgd/libs/bgd_default.json` L13-14 | `libs_excludes` 含 `client/eg`、`client/hook`——模板中不存在这两个目录 | 残留配置，误导后续维护者以为存在这些目录 | 清理为仅 `types` |
| P17 | `template/.bgd/src/common/base/json.lua` | 与 `libs/common/api/json.lua` 近乎逐行重复（仅类名 CommonJson2），且**全仓库无任何引用** | 死代码；双份维护漂移风险；`base/` 目录在白名单 common/ 下会进产物 | 删除，或改为薄封装 `return require('libs.common.api.json')` 并在文档说明 base/ 的用途 |

### 低危（10）

| # | 位置 | 现象 | 建议 |
| --- | --- | --- | --- |
| P18 | `src/server/GameServer.lua` L28 | 日志打印 `Exp: exp/exp`（无 max_exp 字段），复制粘贴错误 | 修正为 `exp` 单值或补 max_exp |
| P19 | `libs/client/init.lua` L26、L40 | `场景-加载完成` 用 `event_register` 和 `base.game:event` 两种写法重复注册 | 保留一种 |
| P20 | `libs/server/init.lua` L43-46 | 唯一未注释的生命周期日志（玩家-暂时离开），其余全部注释——调试残留，风格不一致 | 统一注释或统一保留 |
| P21 | `libs/common/api/json.lua` L92、L126 | encode_x 用 `%q`（Lua 风格转义）产出非合法 JSON；数字键 `%d` 不支持浮点键 | 文档注明「调试序列化器，非传输格式」，或换 JSON 标准转义 |
| P22 | `libs/common/api/promise.lua` L203-205、L134-136、L18 | as_promise 丢多返回值；multi_promise 注解 self 类型误写 promise；co_result 注解「抛异常」与实现（log.error）不符 | 逐一修正注解；as_promise 用 table.pack 保留多返回值 |
| P23 | `src/{common,server,client}/const/game_*_const_tpl.lua` | 占位垃圾数据 `['a']='A'` | 删除文件或换成有意义的示例常量 |
| P24 | `libs/types/bgd_api.d.lua` L2 | 注释引用过时路径 `templates/init_tp_libs.lua`（现行为 `libs/init.lua` 模板） | 修正注释 |
| P25 | `libs/common/api/class.lua` L80-87 | `__supper_map` 拼写（supper→super）已进入公开 API 表面 | 择机改名并保留兼容别名，或文档明示 |
| P26 | `libs/client/api/io.lua` L75、L39、L53 | `log.error(nil, ...)` 首参 nil 与其他调用不一致；exist_* 把「不存在」记为 error 级 | 统一签名；「不存在」降级为 info 或不记日志 |
| P27 | `src/README.md` L56、L62、L148 vs 实际 | README 结构图列 `src/doc/`（实际不存在）；L47/L62/L148 仍写 `asset/` 与 `asset_target`（现为 `res/` 五类资源，bgd_default.json 无 asset_target 字段） | 文档随资源系统改名同步更新 |

---

## 5. 与仓库 AGENTS.md 描述的偏差

1. **「模块写法统一 `local M = {} ... return M`」被自身骨架违反**（偏差最严重）：`src/server/PlayerManager.lua`、`src/server/MonsterManager.lua` 全局写法且无 return，`src/common/DataManager.lua` 全局写法（虽有 return）。
2. **仓库结构图写 `src/ doc/`，实际 src/ 下无 doc/ 目录**（仅 libs/ 有 doc/）。
3. **libs/README.md 描述的资源机制已过期**：写 `*/asset/` → res 输出与 `asset_target` 配置；实际目录为 `res/`（image/particle/sound/spine/sprites 五类），`bgd_default.json` 中无 `asset_target` 字段。仓库 AGENTS.md 本身的描述是新的（res 五类），但下发到项目的 libs/README.md 是旧的——**同仓库两份文档不一致**。
4. **doc/api 文档根路径 `#common.sce_base.*` 与 AGENTS.md 宣称的 `bgd_api.<端>.<文件名>` 机制不符**：文档未随 API 注册机制迁移更新（前代框架遗留）。
5. **`libs_excludes` 除 AGENTS.md 所述的 `types` 外，还残留不存在的 `client/eg`、`client/hook`**——AGENTS.md 的描述与实际配置不完全对应（残留项无任何目录支撑）。

验证为**属实**的描述：白名单构建目录、types 排除构建、init 渲染 `{{target}}/{{module}}`、api 聚合 AUTO-GENERATED、bgd.json 仅存状态与覆盖项骨架、entrance 双端同内容。

---

## 6. 总体结论

框架的**骨架设计是优秀的**：静态 require + 构建期前缀改写保住了 EmmyLua 全程可见性，三命名空间 + api 自动注册 + 软覆盖的扩展模型清晰自洽，白名单构建与配置 overlay 边界干净。但**实现层打磨不足**：27 个问题中有 4 个高危——最刺眼的是模板内明文提交真实 API key（P1，需立即吊销），以及 exception.lua 的 `instance_of` 未定义（P2）、deque.lua 的 `log.erro` 拼写（P3）两个确定性运行时崩溃，说明基础库缺乏最基础的运行验证。游戏骨架（src/）是全框架最薄弱的一环：它自己带头违反「local M + return M」铁律，GameClient 示例含面板泄漏与重复注册等反模式，作为「用户第一眼看到的代码」教学效果是负向的。文档体系存在系统性过期（doc/api 全部停留在前代 `#common.sce_base` 路径、README 的 asset 机制未随 res 改名同步），与 AGENTS.md 声称的「文档与功能同提交」约定执行不力。建议优先级：吊销密钥 → 修 P2/P3 两个崩溃 → 重写 src 骨架三模块 → 全量刷新 doc/api 与 libs/README。
