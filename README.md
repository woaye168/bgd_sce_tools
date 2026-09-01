# bgd_sce_tools

BGD 工作室 · 星火编辑器（SCE）Lua 框架构建工具（Windows 桌面应用）。

本工具配合框架模板仓库 **[bgd_sce_framework](https://github.com/woaye168/bgd_sce_framework)** 一起使用：

| 仓库 | 作用 |
| --- | --- |
| **bgd_sce_tools**（本仓库） | Tauri 桌面构建工具：初始化项目、全量构建、监听更新、框架更新 |
| [bgd_sce_framework](https://github.com/woaye168/bgd_sce_framework) | 框架代码模板（`.bgd` 目录内容），定义代码约定与框架模块 |

> 两个仓库需要一起看：框架定义"代码怎么写"，工具负责"代码怎么构建"。

---

## 功能特性

- **项目初始化**：从框架仓库下载最新框架，一键生成 `.bgd` 结构（libs/src 分治）、合并生成 `.emmyrc.json` 与 `.gitignore`
- **初始化锁**：首次初始化后生成 `.bgd/init.lock`，重复初始化被拒绝（确认后可强制解锁，旧 `.bgd` 自动备份）
- **全量构建 / 清除构建**：白名单构建（`server`/`client`/`common`/`res`/`entrance`），静态 require 自动改写为运行时模块名；清除时入口文件**还原为编辑器原文**
- **监听更新**：文件级增量构建，修改秒级同步，删除自动清理产物，`api/` 目录变动自动重新生成注册聚合，emmyrc/gitignore 片段变动自动重新合并
- **自动类型提示**：`api/` 目录丢入模块文件即注册到 `bgd_api`，EmmyLua 补全即刻生效
- **框架增量更新**：三路哈希对比，你改过的文件不会被覆盖（冲突时本地保留 + 新版另存 `.framework-new` + 报告清单）
- **配置设定**：默认值内建于工具（exe 内嵌 `bgd_default.json`，随安装释放到安装目录仅供查看）+ `bgd.json`（项目覆盖）双层配置；网络代理与 GitHub Token 设置（对更新检查、框架下载、应用市场生效）
- **私有仓库访问**：仓库已转私有，所有 GitHub 链路（框架下载/更新、应用市场、自我更新）通过应用设置里的 fine-grained PAT 认证
- **自我更新**：启动后可在【关于】页检查更新，自动下载安装新版本（自建逻辑：认证查询最新 Release → 下载 NSIS 安装包 → 启动安装）
- **应用市场**：安装/升级/卸载独立应用（如模块To触编、编辑器补丁）；清单来自 [bgd_sce_appsdk](https://github.com/woaye168/bgd_sce_appsdk) 的极简 registry，版本/描述/版本说明由应用仓库 CI 合成的 app-release.json 提供；有新版显示「升级」按钮（可展开版本说明），一键覆盖更新（运行中的实例自动停止后装回）；支持应用「静默自启」（后台拉起，单开唤起；宿主退出联动关闭）
- **界面**：侧边栏布局，明/暗主题切换，构建日志实时输出

## 下载安装

到 [Releases](https://github.com/woaye168/bgd_sce_tools/releases) 下载最新的 `bgd_sce_tools_x.x.x_x64-setup.exe`。

> 未做代码签名，Windows SmartScreen 会提示"未知发布者"，选择"仍要运行"即可。运行时依赖 WebView2（Win10 及以上系统自带）。

## 使用指南

### 首次使用

1. 【项目】页 → "初始化新项目"：选择一个空文件夹（或已有 SCE 地图项目），工具会从 [bgd_sce_framework](https://github.com/woaye168/bgd_sce_framework) 下载框架并生成 `.bgd` 结构
2. 在 `.bgd/src/` 下按约定编写游戏代码（约定详见框架仓库 README）
3. 【构建】页 → "全量构建"，产物输出到 `script/`、`ui/script/`，引擎入口自动合并到 `src/main.lua`
4. 日常开发：【监听】页打开监听开关，改代码即自动增量构建

### 国内网络提示

GitHub 直连不稳定时：【设置】页 → "通用设置" → 填入本机代理（如 `http://127.0.0.1:7897`）→ 保存。对**检查更新、框架版本检查、框架下载**全部生效。

### GitHub Token（必需，仓库已转私有）

框架/插件/工具仓库均为**私有仓库**，首次使用必须配置 Token，否则框架下载/更新、应用市场、自我更新全部不可用：

1. GitHub → Settings → Developer settings → Personal access tokens → **Fine-grained tokens** → Generate new token
2. Repository access 选 **Only select repositories**，勾选 `bgd_sce_tools` / `bgd_sce_framework` / `bgd_sce_appsdk` / `sce_app_visual-injector` / `sce_app_editor-patch`（后续新增应用仓库同样要加）
3. Permissions 只需 **Contents: Read-only**
4. 生成后填入：【设置】页 → "通用设置" → GitHub Token → 保存（或 CLI：`bgd_sce_tools setting set github_token <PAT>`）

Token 仅存于本机 Windows 凭据管理器（条目 `bgd_sce_tools/github_token`），不会写入任何项目或仓库；settings.json 不落盘 token（旧版明文配置会在下次启动时自动迁移并清除）。

**多台电脑使用**：Token 按机器各自配置。同一个 PAT 可以复制到多台电脑填用；更推荐每台电脑单独建一个 PAT（哪台不用了单独吊销哪个）。

**给其他人使用（协作者）**：

1. 仓库所有者操作（**每个仓库**都要加一次）：仓库页 → Settings → 左侧 **Collaborators** → **Add people** → 输入对方 GitHub 用户名/邮箱 → Role 选 **Read**；对方在邀请邮件/通知中接受
2. 对方接受邀请后，用**自己的 GitHub 账号**按上面步骤建 fine-grained PAT（此时他能选到这些仓库），填入自己机器的【设置】页即可
3. 不要把自己的 PAT 直接发给别人——那是你账号的凭证

### 界面导览

| 页面 | 功能 |
| --- | --- |
| 项目 | 选择/切换项目、最近项目列表、初始化新项目、显示框架版本 |
| 构建 | 全量构建、清除构建、清理日志，实时构建输出 |
| 监听 | 监听开关，文件变更事件流实时显示 |
| 应用 | 应用市场（安装/升级/卸载，可展开版本说明）、已安装应用（打开/静默自启） |
| 设置 | 通用设置（代理 / GitHub Token）、框架设置（更新）、`bgd.json` 构建路径配置 |
| 关于 | 版本信息、检查更新（自动下载安装） |

## 技术栈与项目结构

- **桌面壳**：Tauri 2.x（Rust 后端 + 系统 WebView2）
- **前端**：Vite + React 18 + TypeScript + Tailwind CSS（`dark:` 类切换明暗主题）
- **后端**：Rust（构建引擎 / 文件监听 / GitHub 下载 / 配置管理）

```
├── src/                        # 前端（React）
│   ├── App.tsx                 # 布局与主题
│   ├── lib/{api.ts, types.ts}  # Tauri 命令封装与类型
│   ├── components/             # Sidebar / LogPanel / Card
│   └── pages/                  # 项目/构建/监听/应用/设置/关于 六个页面
├── src-tauri/                  # 后端（Rust）
│   ├── src/main.rs             # 二进制入口（CLI 分发 + GUI 启动）
│   ├── src/cli.rs              # CLI 子命令
│   ├── src/lib.rs              # Tauri 命令注册与应用状态
│   ├── src/builder/            # 构建核心（mod 主流程编排 + merge/rewrite/res/watch 子模块）
│   ├── src/project.rs          # 初始化/框架下载更新/最近项目/应用设置
│   ├── src/config.rs           # bgd.json 读写
│   ├── src/apps.rs             # 应用市场（清单/安装/卸载/静默自启/联动停止）
│   ├── src/secret.rs           # 敏感凭证存储（GitHub Token 存 Windows 凭据管理器）
│   ├── src/net.rs              # 统一 HTTP 客户端（代理 + GitHub Token 认证）
│   ├── src/updater.rs          # 自我更新（私有仓库：认证查 Release + 下载安装包）
│   ├── tauri.conf.json         # 应用配置
│   └── capabilities/           # 权限声明
└── .github/workflows/
    ├── ci.yml                  # push/PR：前端构建 + cargo check/test
    └── release.yml             # tag v*：tauri-action 出包 + 发布 Release
```

## 二次开发（Fork 后）

### 环境要求

| 依赖 | 版本 | 说明 |
| --- | --- | --- |
| Node.js | 22 | 前端构建 |
| pnpm | 9+ | 包管理（`pnpm-workspace.yaml` 已声明 `packages` 与 `onlyBuiltDependencies`） |
| Rust | stable (1.77+) | Tauri 后端 |
| 操作系统 | Windows 10+ | 开发/运行均需 WebView2 |

### 本地开发

```bash
pnpm install        # 安装前端依赖
pnpm tauri dev      # 开发模式（前端热更新 + Rust 自动重编译）
```

### 本地构建

```bash
pnpm tauri build    # 产出 NSIS 安装包（src-tauri/target/release/bundle/）
```

### CLI 子命令

exe 命中子命令即以控制台模式执行（否则启动 GUI），可用于脚本与无 GUI 环境：

```bash
bgd_sce_tools build --project <项目路径> [--log .bgd/log/build.log]   # 全量构建
bgd_sce_tools watch --project <项目路径> [--log .bgd/log/watch.log]   # 监听更新（前台阻塞，Ctrl+C 停止）
bgd_sce_tools clean --project <项目路径>             # 清除构建（还原入口原文）
bgd_sce_tools clean-logs --project <项目路径>        # 清理日志
bgd_sce_tools init --project <路径> [--force]        # 初始化项目
bgd_sce_tools update-framework --project <路径>      # 增量更新框架
bgd_sce_tools check-framework --project <路径>       # 检查框架更新
bgd_sce_tools check-watch --project <项目路径>       # 判断是否监听中
bgd_sce_tools config get <键> --project <项目路径>   # 读取 bgd 配置（合并后生效值）
bgd_sce_tools config set <键> <值> --project <路径>  # 写入 bgd.json 覆盖项
bgd_sce_tools config reset <键> --project <路径>     # 恢复指定配置项为工具内建默认
bgd_sce_tools setting set github_token <PAT>         # 写入 GitHub Token（私有仓库必需）
bgd_sce_tools app <应用id> --project <项目路径>      # 启动已安装应用（透传 --project-path）
# 可选参数：--repo owner/repo  --proxy http://127.0.0.1:7897  --log <日志路径>
```

安装时会自动把安装目录写入用户 PATH，装完即可直接敲 `bgd_sce_tools`。

开发期等价：`cargo run -- build --project ...`（在 `src-tauri/` 下）。

### Fork 后必改清单（重要）

fork 本仓库（及 [bgd_sce_framework](https://github.com/woaye168/bgd_sce_framework)）后，需要修改以下位置才能完整使用：

**1. 自我更新仓库（`src-tauri/src/updater.rs`）**

自我更新为自建逻辑（不依赖 tauri updater 插件，无签名密钥），fork 只需改 `SELF_REPO` 常量：

```rust
const SELF_REPO: &str = "你的用户名/bgd_sce_tools";
```

**2. 框架仓库地址（`src-tauri/src/project.rs`）**

```rust
const DEFAULT_FRAMEWORK_REPO: &str = "你的用户名/bgd_sce_framework";
```

**3. 应用标识（可选）**

`tauri.conf.json` 的 `identifier`（`com.bgd.sce-tools`）建议改成你自己的域名形式，避免与上游应用的配置目录冲突。

**4. 游戏项目侧**

已初始化的项目，其 `.bgd/bgd.json` 的 `framework_repo` 字段改为你的框架 fork。

### 发布新版本（CI/CD）

版本号**唯一来源是 git tag**，CI 构建时自动注入 `tauri.conf.json` 与 `Cargo.toml`；源码中固定为 `0.0.0-dev` 占位，无需手工同步。

```bash
# 提交并打 tag 推送（无需写注解，Release notes 自动生成）
git add -A && git commit -m "chore(release): v0.1.4"
git push
git tag -a v0.1.4
git push origin v0.1.4
```

`release.yml` 会自动：安装依赖 → 构建前端 → cargo 编译 → 打包 NSIS 安装包 → 创建 GitHub Release 并上传 `setup.exe`。

老版本用户打开工具点"检查更新"即可自动升级（自建更新逻辑：认证读取最新 Release 的 `*-setup.exe` asset，下载后启动安装器）。

`ci.yml` 在每次 push/PR 时做基本验证（前端构建 + cargo check/test）。

## 构建逻辑（与框架的约定）

工具的构建引擎（`src-tauri/src/builder/`）实现以下规则，与框架仓库的约定一一对应：

1. **白名单构建**：code set 根下仅 `server/`、`client/`、`common/`、`res/`、`entrance/` 进产物流程；根级文件（init.lua、.emmyrc.json、.gitignore、AGENTS.md、doc/）各有专门流程
2. **模块名改写**：复制 `.lua` 时，把引号内的 `src.` / `libs.` 前缀改写为运行时根名（`bgd_game_server.` / `bgd_game_client.` / `bgd_libs_server.` / `bgd_libs_client.`，取自配置的 target 目录名）；裸 `require('src')` / `require('libs')` 同样改写为对应运行时根
3. **端拆分**：`server/` → 仅服务端产物，`client/` → 仅客户端产物，`common/` → 双端各一份
4. **API 自动注册**：扫描 `{libs,src}/{common,server,client}/api/*.lua`，在源码树内重新生成聚合 `init.lua`（框架用 `bgd_api.<端> = {}` 新建，游戏用 `or {}` 合并）
5. **入口合并（分界标记）**：`src/main.lua` 标记之前的内容视为原文永久保留，标记之后为 `libs/entrance/` + `src/entrance/` 的合并产物；重复构建幂等，「清除构建」还原原文
6. **配置合并**：`libs/.emmyrc.json` + `src/.emmyrc.json` 深合并生成项目根配置（数组并集、标量游戏侧优先）；`.gitignore` 同理（文本拼接去重）；项目根 `AGENTS.md` 由 `src/AGENTS.md` 同步生成（构建/监听/初始化时，内容一致则跳过），要改就改 `.bgd/src/AGENTS.md`
7. **init 渲染**：code set 根 `init.lua` 渲染 `{{target}}` / `{{module}}` / `{{time}}` 后输出到产物根目录
8. **资源系统**：`res/` 目录五类资源（image/particle/sound/spine/sprites）同步到引擎目录；`.lua` 中 `'libs/res/<类型>/...'` / `'src/res/<类型>/...'` 构建时替换为运行时路径（sound 去 `.ogg`，sprites 前缀 `@<ProjectName>`）；资源类型/落位/运行时前缀均为「设置-构建路径配置」里的资源路径规则（`res_rules`），可按项目覆盖，也可新增/删除自定义类型
9. **配置 overlay**：生效配置 = 工具内建默认（exe 内嵌 `bgd_default.json`，随安装释放到安装目录仅供查看）逐 key 被 `.bgd/bgd.json`（项目覆盖）覆盖；保存只写差异。**所有路径配置统一相对项目根**（0.9.1 起）
10. **替换排除（rewrite_excludes）**：相对项目根的完整路径（不含扩展名），命中文件或目录则正常进产物但跳过模块名/res 替换；单条内 `|` 分隔多个；默认含 `.bgd/src/client/path_rules`（工具自产盖戳保护，设置中可见可删）
11. **行级注解跳过（rewrite_skip_annotation）**：某行含注解文本（默认 `-- @bgd:no-rewrite`）时其**下一行**跳过全部替换（require 改写 + res 替换，entrance 管线同样生效）；注解文本可配置，留空禁用
12. **配置热更新**：监听（watch）运行期间修改 bgd.json 即时生效（mtime 轮询热重读，不重启监听）；path_rules 盖戳随 res_rules 变更自动重生成；历史产物不追溯，全量构建后完全生效

## 许可证

本项目采用 [GNU Affero General Public License v3.0](LICENSE) 开源。
