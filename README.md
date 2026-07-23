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
- **全量构建 / 清除构建**：白名单构建（`server`/`client`/`common`/`asset`/`entrance`），静态 require 自动改写为运行时模块名；清除时入口文件**还原为编辑器原文**
- **监听更新**：文件级增量构建，修改秒级同步，删除自动清理产物，`api/` 目录变动自动重新生成注册聚合，emmyrc/gitignore 片段变动自动重新合并
- **自动类型提示**：`api/` 目录丢入模块文件即注册到 `bgd_api`，EmmyLua 补全即刻生效
- **框架增量更新**：三路哈希对比，你改过的文件不会被覆盖（冲突时本地保留 + 新版另存 `.framework-new` + 报告清单）
- **配置设定**：`bgd_default.json`（框架下发）+ `bgd.json`（项目覆盖）双层配置；网络代理设置（对更新检查、框架下载生效）
- **自我更新**：启动后可在【关于】页检查更新，自动下载安装新版本
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

### 界面导览

| 页面 | 功能 |
| --- | --- |
| 项目 | 选择/切换项目、最近项目列表、初始化新项目、显示框架版本 |
| 构建 | 全量构建、清除构建、清理日志，实时构建输出 |
| 监听 | 监听开关，文件变更事件流实时显示 |
| 设置 | 通用设置（代理）、框架设置（更新）、`bgd.json` 构建路径配置 |
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
│   └── pages/                  # 项目/构建/监听/设置/关于 五个页面
├── src-tauri/                  # 后端（Rust）
│   ├── src/main.rs             # 二进制入口
│   ├── src/lib.rs              # Tauri 命令注册与应用状态
│   ├── src/builder.rs          # 构建核心（全量/增量/清理/监听/API聚合生成）
│   ├── src/project.rs          # 初始化/框架下载更新/最近项目/应用设置
│   ├── src/config.rs           # bgd.json 读写
│   ├── tauri.conf.json         # 应用配置（含 updater 端点与公钥）
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
bgd_sce_tools clean --project <项目路径>             # 清除构建（还原入口原文）
bgd_sce_tools clean-logs --project <项目路径>        # 清理日志
bgd_sce_tools init --project <路径> [--force]        # 初始化项目
bgd_sce_tools update-framework --project <路径>      # 增量更新框架
bgd_sce_tools check-framework --project <路径>       # 检查框架更新
bgd_sce_tools check-watch --project <项目路径>       # 判断是否监听中
# 可选参数：--repo owner/repo  --proxy http://127.0.0.1:7897  --log <日志路径>
```

安装时会自动把安装目录写入用户 PATH，装完即可直接敲 `bgd_sce_tools`。

开发期等价：`cargo run -- build --project ...`（在 `src-tauri/` 下）。

### Fork 后必改清单（重要）

fork 本仓库（及 [bgd_sce_framework](https://github.com/woaye168/bgd_sce_framework)）后，需要修改以下位置才能完整使用：

**1. 更新器端点（`src-tauri/tauri.conf.json`）**

```json
"plugins": {
  "updater": {
    "endpoints": ["https://github.com/你的用户名/bgd_sce_tools/releases/latest/download/latest.json"],
    "pubkey": "你的更新公钥（见第 2 步）"
  }
}
```

**2. 生成你自己的更新签名密钥对**

```bash
npx @tauri-apps/cli@2 signer generate -w updater.key -p 你的密码
```

- 公钥（`.pub` 文件内容）→ 填入 `tauri.conf.json` 的 `pubkey`
- 私钥 → 到你的 fork 仓库 **Settings → Secrets and variables → Actions** 添加：
  - `TAURI_SIGNING_PRIVATE_KEY` = 私钥文件内容
  - `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` = 私钥密码
- **私钥不要提交到仓库！** 丢失私钥或密码将无法再签发更新包。

**3. 框架仓库地址（`src-tauri/src/project.rs`）**

```rust
const DEFAULT_FRAMEWORK_REPO: &str = "你的用户名/bgd_sce_framework";
```

**4. 应用标识（可选）**

`tauri.conf.json` 的 `identifier`（`com.bgd.sce-tools`）建议改成你自己的域名形式，避免与上游应用的配置目录冲突。

**5. 游戏项目侧**

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

`release.yml` 会自动：安装依赖 → 构建前端 → cargo 编译 → 打包 NSIS 安装包 → 用 Secrets 里的私钥签名 → 创建 GitHub Release 并上传 `setup.exe`、`setup.exe.sig`、`latest.json`。

老版本用户打开工具点"检查更新"即可自动升级（updater 读取 `releases/latest/download/latest.json`）。

`ci.yml` 在每次 push/PR 时做基本验证（前端构建 + cargo check/test）。

## 构建逻辑（与框架的约定）

工具的构建引擎（`src-tauri/src/builder.rs`）实现以下规则，与框架仓库的约定一一对应：

1. **白名单构建**：code set 根下仅 `server/`、`client/`、`common/`、`asset/`、`entrance/` 进产物流程；根级文件（init.lua、bgd_default.json、.emmyrc.json、.gitignore、doc/）各有专门流程
2. **模块名改写**：复制 `.lua` 时，把引号内的 `src.` / `libs.` 前缀改写为运行时根名（`bgd_game_server.` / `bgd_game_client.` / `bgd_libs_server.` / `bgd_libs_client.`，取自配置的 target 目录名）
3. **端拆分**：`server/` → 仅服务端产物，`client/` → 仅客户端产物，`common/` → 双端各一份
4. **API 自动注册**：扫描 `{libs,src}/{common,server,client}/api/*.lua`，在源码树内重新生成聚合 `init.lua`（框架用 `bgd_api.<端> = {}` 新建，游戏用 `or {}` 合并）
5. **入口合并（分界标记）**：`src/main.lua` 标记之前的内容视为原文永久保留，标记之后为 `libs/entrance/` + `src/entrance/` 的合并产物；重复构建幂等，「清除构建」还原原文
6. **配置合并**：`libs/.emmyrc.json` + `src/.emmyrc.json` 深合并生成项目根配置（数组并集、标量游戏侧优先）；`.gitignore` 同理（文本拼接去重）
7. **init 渲染**：code set 根 `init.lua` 渲染 `{{target}}` / `{{module}}` / `{{time}}` 后输出到产物根目录
8. **资源与包装**：`asset/` 二进制复制到 `res/`；`.html/.css/.js` 包装为 Lua 字符串模块
9. **配置 overlay**：生效配置 = `libs/bgd_default.json`（框架下发）逐 key 被 `.bgd/bgd.json`（项目覆盖）覆盖

## 许可证

本项目采用 [GNU General Public License v3.0](LICENSE) 开源。
