import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { api } from "../lib/api";
import type { AppSettings, BgdConfig, FrameworkUpdateInfo, ResRule, UpdateReport } from "../lib/types";
import Card from "../components/Card";

/** 设置页：通用设置（代理）、项目配置（bgd.json 表单）、框架更新 */
export default function SettingsPage() {
  const [config, setConfig] = useState<BgdConfig | null>(null);
  const [defaults, setDefaults] = useState<BgdConfig | null>(null);
  const [appSettings, setAppSettings] = useState<AppSettings>({ proxy: "", watch_enabled: false, save_log: false, github_token: "", auto_start_apps: [], auto_start_disabled: [] });
  const [resRules, setResRules] = useState<ResRule[]>([]);
  const [updateInfo, setUpdateInfo] = useState<FrameworkUpdateInfo | null>(null);
  const [report, setReport] = useState<UpdateReport | null>(null);
  const [message, setMessage] = useState("");
  const [settingsMsg, setSettingsMsg] = useState("");
  const [busy, setBusy] = useState(false);

  // 进入页面即拉取一次配置（含 init.lock / bgd.json 最新状态）
  // 必须有依赖数组 []：无依赖时每次渲染都会重新拉取并用存档值覆盖输入框，导致无法正常输入
  useEffect(() => {
    api.getConfig().then(setConfig).catch(() => setConfig(null));
    api.getConfigDefaults().then(setDefaults).catch(() => {});
    api.getAppSettings().then(setAppSettings).catch(() => {});
    api.getEffectiveResRules().then(setResRules).catch(() => {});
  }, []);

  const saveAppSettings = async () => {
    setBusy(true);
    try {
      await api.saveAppSettings(appSettings);
      setSettingsMsg("✔ 已保存（对检查更新、框架下载生效）");
    } catch (e) {
      setSettingsMsg(`✘ 保存失败: ${String(e)}`);
    } finally {
      setBusy(false);
    }
  };

  const set = (key: keyof BgdConfig, value: string | boolean | string[]) => {
    setConfig((prev) => (prev ? { ...prev, [key]: value } : prev));
  };

  const save = async () => {
    if (!config) return;
    setBusy(true);
    try {
      await api.saveConfig(config);
      setMessage("✔ 配置已保存");
    } catch (e) {
      setMessage(`✘ 保存失败: ${String(e)}`);
    } finally {
      setBusy(false);
    }
  };

  const checkUpdate = async () => {
    setBusy(true);
    setMessage("");
    try {
      setUpdateInfo(await api.checkFrameworkUpdate());
    } catch (e) {
      setMessage(`✘ 检查失败: ${String(e)}`);
    } finally {
      setBusy(false);
    }
  };

  /** 版本号归一化（去 v 前缀后比较） */
  const sameVersion = (a?: string | null, b?: string | null) =>
    (a ?? "").replace(/^v/, "") !== "" &&
    (a ?? "").replace(/^v/, "") === (b ?? "").replace(/^v/, "");

  const doUpdate = async () => {
    setBusy(true);
    setReport(null);
    setMessage("正在下载框架...");
    let unlisten: (() => void) | undefined;
    try {
      // 下载进度事件：显示 已下载/总大小（M，2 位小数）
      unlisten = await listen<{ downloaded: number; total: number | null }>(
        "framework-download-progress",
        (ev) => {
          const done = (ev.payload.downloaded / 1048576).toFixed(2);
          const total = ev.payload.total ? (ev.payload.total / 1048576).toFixed(2) : "?";
          setMessage(`正在下载框架... ${done}M / ${total}M`);
        },
      );
      const rep = await api.updateFramework();
      setReport(rep);
      setMessage(
        `✔ 框架已更新到 ${rep.version || "最新"}：更新 ${rep.updated}，新增 ${rep.added}，删除 ${rep.removed}，保留本地 ${rep.kept_local}，冲突 ${rep.conflicts.length}`
      );
      setUpdateInfo(null);
    } catch (e) {
      setMessage(`✘ 更新失败: ${String(e)}`);
    } finally {
      unlisten?.();
      setBusy(false);
    }
  };

  /** 恢复默认按钮（当前值与内建默认不同时显示；点击设回默认值，保存时自然不落盘覆盖项） */
  const resetButton = (key: keyof BgdConfig) => {
    if (!config || !defaults) return null;
    if (JSON.stringify(config[key]) === JSON.stringify(defaults[key])) return null;
    return (
      <button
        onClick={() => set(key, defaults[key] as string & boolean & string[])}
        className="ml-2 text-slate-400 hover:text-slate-600 dark:hover:text-slate-200"
        title={`恢复默认：${JSON.stringify(defaults[key])}`}
      >
        恢复默认
      </button>
    );
  };

  const textField = (
    label: string,
    key: keyof BgdConfig,
    hint?: string
  ) => {
    if (!config) return null;
    return (
      <label className="block">
        <span className="mb-1 block text-xs font-medium text-slate-500 dark:text-slate-400">
          {label}
          {resetButton(key)}
          {hint && <span className="ml-2 text-slate-400">{hint}</span>}
        </span>
        <input
          value={String(config[key] ?? "")}
          onChange={(e) => set(key, e.target.value)}
          className="w-full rounded-lg border border-slate-300 bg-transparent px-3 py-2 text-sm text-slate-700 dark:border-slate-600 dark:text-slate-200"
        />
      </label>
    );
  };

  return (
    <div className="space-y-5">
      <Card title="日志设置">
        <div className="flex items-center justify-between">
          <div>
            <p className="text-sm font-medium text-slate-700 dark:text-slate-200">保存到本地</p>
            <p className="mt-0.5 text-xs text-slate-500 dark:text-slate-400">
              构建/监听日志写入 .bgd/log/build-YYYY-MM-DD.log，按天滚动
            </p>
          </div>
          <button
            onClick={async () => {
              const next = { ...appSettings, save_log: !appSettings.save_log };
              setAppSettings(next);
              try {
                await api.saveAppSettings(next);
                setSettingsMsg(next.save_log ? "✔ 日志保存已开启" : "✔ 日志保存已关闭");
              } catch (e) {
                setSettingsMsg(`✘ 保存失败: ${String(e)}`);
              }
            }}
            className={`relative h-6 w-11 rounded-full transition-colors ${
              appSettings.save_log ? "bg-emerald-500" : "bg-slate-300 dark:bg-slate-600"
            }`}
          >
            <span
              className={`absolute top-0.5 h-5 w-5 rounded-full bg-white shadow transition-all ${
                appSettings.save_log ? "left-[calc(100%-22px)]" : "left-0.5"
              }`}
            />
          </button>
        </div>
        {settingsMsg && (
          <p className="mt-3 text-sm text-slate-600 dark:text-slate-300">{settingsMsg}</p>
        )}
      </Card>

      <Card title="通用设置">
        <label className="block">
          <span className="mb-1 block text-xs font-medium text-slate-500 dark:text-slate-400">
            网络代理
            <span className="ml-2 text-slate-400">
              如 http://127.0.0.1:7897，留空表示直连（对检查更新、框架下载生效）
            </span>
          </span>
          <input
            value={appSettings.proxy}
            onChange={(e) => setAppSettings((prev) => ({ ...prev, proxy: e.target.value }))}
            placeholder="http://127.0.0.1:7897"
            className="w-full rounded-lg border border-slate-300 bg-transparent px-3 py-2 text-sm text-slate-700 dark:border-slate-600 dark:text-slate-200"
          />
        </label>
        <label className="mt-4 block">
          <span className="mb-1 block text-xs font-medium text-slate-500 dark:text-slate-400">
            GitHub Token
            <span className="ml-2 text-slate-400">
              fine-grained PAT（Contents 只读）；私有仓库的框架下载/更新、应用市场、自我更新均需要
            </span>
          </span>
          <input
            type="password"
            value={appSettings.github_token}
            onChange={(e) => setAppSettings((prev) => ({ ...prev, github_token: e.target.value }))}
            placeholder="github_pat_..."
            className="w-full rounded-lg border border-slate-300 bg-transparent px-3 py-2 text-sm text-slate-700 dark:border-slate-600 dark:text-slate-200"
          />
        </label>
        <button
          onClick={saveAppSettings}
          disabled={busy}
          className="mt-3 rounded-lg bg-indigo-600 px-4 py-2 text-sm text-white hover:bg-indigo-500 disabled:opacity-50"
        >
          保存
        </button>
      </Card>

      {!config && (
        <Card title="项目配置">
          <p className="text-sm text-slate-500 dark:text-slate-400">
            请先在「项目」页选择一个已初始化的项目
          </p>
        </Card>
      )}

      {config && (
        <>
      <Card title="框架设置">
        <div className="space-y-3">
          {textField("框架仓库", "framework_repo", "如 woaye168/bgd_sce_framework")}
          <div className="flex flex-wrap items-center gap-3">
            <button
              onClick={checkUpdate}
              disabled={busy}
              className="rounded-lg bg-indigo-600 px-4 py-2 text-sm text-white hover:bg-indigo-500 disabled:opacity-50"
            >
              检查框架更新
            </button>
            {updateInfo && (
              <span className="text-sm text-slate-600 dark:text-slate-300">
                当前: {updateInfo.current || "未知"} / 最新: {updateInfo.latest ?? "无 release"}
              </span>
            )}
            {updateInfo?.latest && !sameVersion(updateInfo.latest, updateInfo.current) && (
              <button
                onClick={doUpdate}
                disabled={busy}
                className="rounded-lg bg-emerald-600 px-4 py-2 text-sm text-white hover:bg-emerald-500 disabled:opacity-50"
              >
                更新框架
              </button>
            )}
          </div>

          {report && report.conflicts.length > 0 && (
            <div className="rounded-lg border border-amber-300 bg-amber-50 p-3 dark:border-amber-700 dark:bg-amber-950">
              <p className="mb-2 text-sm font-medium text-amber-800 dark:text-amber-300">
                {report.conflicts.length} 个冲突文件（本地已保留，上游新版另存为 .framework-new，请手动合并）
              </p>
              <ul className="space-y-1 font-mono text-xs text-amber-700 dark:text-amber-400">
                {report.conflicts.map((p) => (
                  <li key={p}>{p}</li>
                ))}
              </ul>
            </div>
          )}
          {report && report.notes.length > 0 && (
            <ul className="space-y-1 text-xs text-slate-500 dark:text-slate-400">
              {report.notes.map((n, i) => (
                <li key={i}>{n}</li>
              ))}
            </ul>
          )}
        </div>
      </Card>

      <Card title="构建路径配置（bgd.json）">
        <p className="mb-3 text-xs text-slate-400 dark:text-slate-500">
          本卡片所有路径统一相对项目根目录（如 .bgd/libs、script/bgd_libs_server）
        </p>
        <div className="grid grid-cols-1 gap-4 md:grid-cols-2">
          {textField("框架源码目录", "libs_dir")}
          {textField("游戏源码目录", "game_dir")}
          {textField("框架服务端产物", "libs_server_target")}
          {textField("框架客户端产物", "libs_client_target")}
          {textField("游戏服务端产物", "game_server_target")}
          {textField("游戏客户端产物", "game_client_target")}
          {textField("服务端入口", "server_entrance")}
          {textField("客户端入口", "client_entrance")}
          {textField("行级跳过注解", "rewrite_skip_annotation", "某行含此文本时其下一行跳过全部替换（留空禁用）")}
        </div>

        {/* 替换排除（rewrite_excludes）：正常进构建产物但跳过模块名/res 路径替换；一行一条 */}
        <label className="mt-4 block">
          <span className="mb-1 block text-xs font-medium text-slate-500 dark:text-slate-400">
            替换排除（rewrite_excludes）
            {resetButton("rewrite_excludes")}
            <span className="ml-2 text-slate-400">
              相对项目根的完整路径（不含扩展名），命中文件或整个目录；单行内可用 | 分隔多个，一行一条。
              命中的文件正常进产物但跳过模块名/res 路径替换。
              默认含 .bgd/src/client/path_rules（工具自产盖戳保护——删除该项后盖戳会被替换损坏！）
            </span>
          </span>
          <textarea
            rows={2}
            value={(config.rewrite_excludes ?? []).join("\n")}
            onChange={(e) =>
              set("rewrite_excludes", e.target.value.split("\n").map((s) => s.trim()).filter((s) => s !== ""))
            }
            className="w-full rounded-lg border border-slate-300 bg-transparent px-3 py-2 font-mono text-xs text-slate-700 dark:border-slate-600 dark:text-slate-200"
          />
        </label>

        {/* 资源路径规则：生效值展示 + 行级覆盖编辑（写 bgd.json res_rules 稀疏覆盖） */}
        <div className="mt-4">
          <p className="mb-1 text-xs font-medium text-slate-500 dark:text-slate-400">
            资源路径规则（res_rules）
            <span className="ml-2 text-slate-400">
              源码 res 字面量 → 运行时路径；{'{prefix}'} = 构建目标目录名，{'{project}'} = 地图 ProjectName；
              行级自定义写入 bgd.json 覆盖，恢复默认删除该行覆盖
            </span>
          </p>
          <div className="overflow-x-auto rounded-lg border border-slate-200 dark:border-slate-700">
            <table className="w-full text-xs">
              <thead>
                <tr className="bg-slate-50 text-left text-slate-500 dark:bg-slate-800 dark:text-slate-400">
                  <th className="px-2 py-1.5">类型</th>
                  <th className="px-2 py-1.5">期望扩展</th>
                  <th className="px-2 py-1.5">引用去扩展名</th>
                  <th className="px-2 py-1.5">磁盘落位前缀</th>
                  <th className="px-2 py-1.5">运行时引用前缀</th>
                  <th className="px-2 py-1.5"></th>
                </tr>
              </thead>
              <tbody>
                {resRules.map((rule) => {
                  const overridden = (config?.res_rules ?? []).some((o) => o.res_type === rule.res_type);
                  const patchRule = (field: keyof ResRule, value: string | boolean) => {
                    if (!config) return;
                    const nextRow = { ...rule, [field]: value };
                    const others = (config.res_rules ?? []).filter((o) => o.res_type !== rule.res_type);
                    setConfig({
                      ...config,
                      res_rules: [...others, {
                        res_type: rule.res_type,
                        expect_ext: nextRow.expect_ext,
                        strip_ext_in_ref: nextRow.strip_ext_in_ref,
                        disk_prefix: nextRow.disk_prefix,
                        runtime_prefix: nextRow.runtime_prefix,
                      }],
                    });
                    setResRules((prev) => prev.map((r) => (r.res_type === rule.res_type ? nextRow : r)));
                  };
                  const resetRule = () => {
                    if (!config) return;
                    setConfig({ ...config, res_rules: (config.res_rules ?? []).filter((o) => o.res_type !== rule.res_type) });
                    api.getEffectiveResRules().then(setResRules).catch(() => {});
                  };
                  const cellInput = (field: "expect_ext" | "disk_prefix" | "runtime_prefix") => (
                    <input
                      value={String(rule[field] ?? "")}
                      onChange={(e) => patchRule(field, e.target.value)}
                      className="w-full min-w-32 rounded border border-slate-200 bg-transparent px-1.5 py-1 font-mono dark:border-slate-600"
                    />
                  );
                  return (
                    <tr key={rule.res_type} className={overridden ? "bg-indigo-50/50 dark:bg-indigo-950/30" : ""}>
                      <td className="px-2 py-1.5 font-mono">{rule.res_type}{overridden && <span className="ml-1 text-indigo-500">*</span>}</td>
                      <td className="px-2 py-1.5">{cellInput("expect_ext")}</td>
                      <td className="px-2 py-1.5">
                        <input type="checkbox" checked={rule.strip_ext_in_ref} onChange={(e) => patchRule("strip_ext_in_ref", e.target.checked)} />
                      </td>
                      <td className="px-2 py-1.5">{cellInput("disk_prefix")}</td>
                      <td className="px-2 py-1.5">{cellInput("runtime_prefix")}</td>
                      <td className="px-2 py-1.5">
                        {overridden && (
                          <button onClick={resetRule} className="text-slate-400 hover:text-slate-600 dark:hover:text-slate-200">恢复默认</button>
                        )}
                      </td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
          </div>
        </div>
        <p className="mt-3 text-xs text-slate-400 dark:text-slate-500">
          enable_build_log / libs_excludes / game_excludes / rewrite_excludes / rewrite_skip_annotation / res_rules 也可用 CLI：
          bgd_sce_tools config set &lt;键&gt; &lt;值&gt; --project &lt;项目路径&gt;（数组用 JSON 数组形式）；config reset &lt;键&gt; 恢复默认
        </p>
        <button
          onClick={save}
          disabled={busy}
          className="mt-4 rounded-lg bg-indigo-600 px-4 py-2 text-sm text-white hover:bg-indigo-500 disabled:opacity-50"
        >
          {busy ? "保存中..." : "保存配置"}
        </button>
        {message && (
          <p className="mt-3 text-sm text-slate-600 dark:text-slate-300">{message}</p>
        )}
      </Card>
        </>
      )}
    </div>
  );
}
