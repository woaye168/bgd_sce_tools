# AGENTS.md — bgd_sce_tools

> 本文件面向在本仓库工作的 AI 编程代理。遵循 [agents.md](https://agents.md/) 规范。

## 项目简介

星火编辑器（SCE）Lua 框架构建工具。Tauri 2（Rust 后端 + WebView2）+ Vite/React/TS/Tailwind 前端的 Windows 桌面应用，同时具备 **CLI 子命令**能力。配套框架模板仓库：[bgd_sce_framework](https://github.com/woaye168/bgd_sce_framework)。

核心职责：项目初始化、全量/增量构建（静态 require 改写）、监听更新、框架三路哈希增量更新、自我更新、应用市场（宿主，不依赖任何应用的功能——编辑器控制/MCP 自持于 sce_app_editor-patch）。

## 环境要求

| 依赖 | 版本 |
| --- | --- |
| Node.js | 22 |
| pnpm | 9+ |
| Rust | stable 1.77+ |
| OS | Windows 10+（需 WebView2） |

## 常用命令

```bash
pnpm install            # 安装前端依赖
pnpm tauri dev          # 开发模式（前端热更新 + Rust 自动重编译）
pnpm tauri build        # 打包（NSIS 安装包）
npx tsc                 # 前端类型检查
cargo check             # Rust 检查（在 src-tauri/ 下）
cargo build             # 构建 debug exe（在 src-tauri/ 下）
```

## CLI 子命令（功能验证的唯一可信方式）

exe 命中子命令即以控制台模式执行，否则启动 GUI。**测 CLI 就是测最终产物的真实代码路径**，不要另写 Python/脚本对照实现。

```bash
bgd_sce_tools build --project <项目路径> [--log .bgd/log/build.log]   # 全量构建
bgd_sce_tools watch --project <项目路径> [--log .bgd/log/watch.log]   # 监听更新（前台阻塞，Ctrl+C 停止）
bgd_sce_tools clean --project <项目路径>             # 清除构建（还原入口原文）
bgd_sce_tools clean-logs --project <项目路径>        # 清理 .bgd/log
bgd_sce_tools init --project <路径> [--force] [--repo owner/repo] [--proxy http://...]
bgd_sce_tools update-framework --project <路径> [--proxy http://...]
bgd_sce_tools check-framework --project <路径> [--proxy http://...]
bgd_sce_tools check-watch --project <项目路径>       # 判断是否监听中
bgd_sce_tools config get <键> --project <项目路径>   # 读取 bgd 配置（合并后生效值）
bgd_sce_tools config set <键> <值> --project <路径>  # 写入 bgd.json 覆盖项
bgd_sce_tools setting get <键>                       # 读取应用设置（proxy / watch_enabled / github_token / auto_start_apps）
bgd_sce_tools setting set <键> <值>                  # 写入应用设置（同上四键；save_log 为 GUI 独有，CLI 不支持）
bgd_sce_tools app <应用id> [--project <路径>]        # 启动已安装应用 EXE（透传 --project-path）
```

`--log <路径>` 会把过程日志同时写入文件（无 GUI 环境查看结果）；`check-watch` 通过项目 `.bgd/.watch_state.json` + PID 与映像名校验判断监听状态（该文件由 GUI 监听开关与 CLI watch 共同维护，已加入框架 .gitignore）；`config`/`setting` 子命令分别读写项目 bgd.json 覆盖项与应用设置，供 CLI 和自动化脚本使用。

开发期等价命令：`cargo run -- build --project ...`（在 src-tauri/ 下）。

### 硬性约定（重要）

**新增或修改任何构建/项目功能时，必须同步完成四件事，缺一不可：**
1. 修改核心逻辑（`builder.rs` / `project.rs` / `config.rs`）
2. 同步修改 `cli.rs`（暴露/调整对应子命令）
3. 用 CLI 在真实项目上验证通过后，才允许提交；标准验证项目为 `D:/sce_online/Res/maps/bgd_sce_veri`（专用空白项目，有 git 存档，只要不提交即可随时还原）
4. 同步检查并更新 `README.md` 与 `AGENTS.md`（如适用）；功能改动与对应文档更新必须在同一次提交中完成，不允许“先改功能后补文档”

## 代码结构

```
src/                        # 前端（React）
  lib/{api.ts, types.ts}    # Tauri 命令封装与类型（与后端命令一一对应）
  pages/                    # 项目/构建/监听/设置/关于/应用市场
  components/               # Sidebar/LogPanel/Card
src-tauri/src/
  main.rs                   # 二进制入口：CLI 分发 + GUI 启动
  cli.rs                    # CLI 子命令（本文件上方有同步约定）
  lib.rs                    # Tauri 命令注册、AppState、启动恢复
  builder.rs                # 构建核心：白名单构建/增量/清理/API聚合/init渲染/入口合并/配置合并/监听去重
  project.rs                # 初始化(含锁)/框架下载/三路哈希增量更新/最近项目/应用设置
  config.rs                 # bgd.json overlay 读写（bgd_default.json 基底 + bgd.json 覆盖）
  apps.rs                   # 应用市场：registry 拉取/安装/卸载/静默自启/停止应用
  secret.rs                 # 敏感凭证存储（github_token 存 Windows 凭据管理器）
```

## 关键机制（改代码前必读）

- **私有仓库认证（GitHub Token）**：四个仓库均为私有，所有 GitHub 请求（api.github.com / codeload / raw.githubusercontent / release asset API）必须走 `net.rs` 的 `http_client(proxy, token)`，token 来自应用设置 `github_token`（fine-grained PAT，Contents 只读）。token 为空时不加头（兼容公开仓库）。**禁止**绕过 net.rs 自建 reqwest 客户端。**token 存储**：持久化在 Windows 凭据管理器（`secret.rs`，keyring/wincred，条目 `bgd_sce_tools/github_token`），不落盘 settings.json；旧版明文配置在 `load_settings` 时自动迁移并 scrub 文件。残余风险：不防同用户针对性进程读取凭据管理器（CredRead）——PAT 必须保持 fine-grained Contents 只读，泄露立即吊销轮换。
- **release asset 下载**：私有仓库的 asset 直链（releases/download/...）带 token 也 404，必须走 API：`repos/<repo>/releases/tags/<tag>` 定位 asset → `asset.url` + `Accept: application/octet-stream` 下载（插件安装与自我更新均如此）。
- **静态 require 改写**：源码写真实路径（`require('src.xxx')`，含裸 `require('src')` / `require('libs')`），构建时引号内前缀改写为运行时根名（`bgd_game_server.` 等）。**禁止**恢复字符串拼接构造 require 路径。
- **白名单构建**：code set 根下仅 `server/client/common/res/entrance` 进产物；根级文件（init.lua、bgd_default.json、.emmyrc.json、.gitignore、AGENTS.md、doc/）各有专门流程。
- **AGENTS.md 同步**：项目根 `AGENTS.md` 由 `.bgd/src/AGENTS.md` 在初始化/构建/监听时同步生成（`sync_agents_md`，内容一致跳过），与 .gitignore 同属构建产物。
- **资源系统**：`res/` 目录五类资源（image/particle/sound/spine/sprites）同步到引擎目录；`.lua` 中字符串字面量 `'libs/res/<类型>/...'` / `'src/res/<类型>/...'` 在构建时替换为运行时路径（sound 去 `.ogg` 扩展名，sprites 前缀 `@<ProjectName>` 从 map_settings.json 注入）。
- **入口合并分界标记**：`src/main.lua` 标记之前为编辑器原文永久保留；之后为合并产物。原文若仍含标记（脏数据）则丢弃重建（自愈）。
- **配置 overlay**：`libs/bgd_default.json`（框架下发）逐 key 被 `.bgd/bgd.json`（项目覆盖）覆盖；保存只写差异。
- **三路哈希增量更新**：基准存 `.bgd/.framework_state.json`；冲突时本地保留 + 新版另存 `.framework-new`。文本文件统一 LF 后哈希（防 CRLF 误报）。
- **监听去重**：同一文件 300ms 窗口聚合一次处理（防编辑器原子保存产生重复日志）。
- **应用市场**：安装即覆盖写入 `apps/<id>/`（应用 id 仅允许字母/数字/下划线/连字符，安装/卸载/启动前校验，杜绝路径穿越）；升级按钮由前端对比 app-release.json 补全后的 registry version 与本地 app.json 版本号驱动，走 `install_app` 覆盖（升级前自动停止运行中实例：先 `--quit` 优雅退出、兜底 taskkill；装完按自启配置重启）。启动应用透传 `--project-path <当前项目>`。**清单读取链（R5）**：registry（[bgd_sce_appsdk](https://github.com/woaye168/bgd_sce_appsdk)，极简条目 id/name/repo）→ 各应用仓库 `releases/latest` 的 `app-release.json` asset 补全版本/描述/作者/asset名/版本说明（CI 合成；升级按钮展示版本说明；发版不再改 registry）。**静默自启**：勾选写入设置 `auto_start_apps`；默认值由 `default_auto_start` 下发（app-release.json 提供；用户本机勾选/取消优先，取消记入 `auto_start_disabled` 不再播种）；宿主启动时后台线程异步拉起（不阻塞首屏；进程列表一次查询内存匹配；子进程一律 CREATE_NO_WINDOW；透传 `--background` 由应用决定是否无窗口驻留）。**宿主联动**：退出时对所有运行中的已安装应用广播 `--quit`（兜底 taskkill）；切换项目时对静默自启应用执行 `<exe> notify project_path=<路径>` 解耦通知（应用自治处理）。

## 测试与验证流程（本地闭环）

**原则：日常功能开发不打 tag、不触发 GitHub Actions，全部本地验证。**

1. `cargo check` + `npx tsc` 通过
2. `cargo build` 出 debug exe
3. 用 CLI 在真实项目上验证本次改动覆盖的功能（见上方 CLI 命令）
4. 涉及前端时 `pnpm build` 通过
5. 全部通过后提交推送（不触发打包）

**只有以下两种情况才打 tag 触发 Actions 出包：**
- 需要交付给用户的正式版本
- 修改了 CI/CD 配置本身

## 发布

版本号**唯一来源是 git tag**：CI（`.github/workflows/release.yml`）构建时把 tag 版本自动注入 `tauri.conf.json` 与 `Cargo.toml`；源码中固定为 `0.0.0-dev` 占位，无需手工同步（AboutPage 版本号取自运行时 `getVersion()`）。

```bash
git tag -a vX.Y.Z          # 无需写注解，Release notes 自动生成
git push origin vX.Y.Z
```

Release notes 由 workflow 用 git log 自动归纳版本间提交（"版本说明 + What's Changed + Full Changelog 链接"），无需手写。打 tag 零备注即可生成专业版本说明。

## 注意事项

- 网络：国内环境拉依赖/调 GitHub API 需代理，本机 `http://127.0.0.1:7897`。cargo/npm 命令前记得设 `$env:HTTP_PROXY` / `$env:HTTPS_PROXY`。
- `pnpm` 在本机沙箱环境不稳定，本地验证前端改用 `npm install` + `node node_modules/typescript/bin/tsc` + `node node_modules/vite/bin/vite.js build`。
- 终端输出中文会 GBK 乱码，属显示问题，不影响实际写入文件/仓库的内容。
- 编译/写文件可直接在仓库内进行；个别沙箱环境如遇写权限问题，回退到 `D:/sce_online/Res/maps/bgd_glzy` 下的临时副本执行。
