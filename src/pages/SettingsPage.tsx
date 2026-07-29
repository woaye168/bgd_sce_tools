import { useEffect, useState } from "react";
import { api } from "../lib/api";
import type { AppSettings, BgdConfig, FrameworkUpdateInfo, UpdateReport } from "../lib/types";
import Card from "../components/Card";

/** 官方插件仓库（固定第一行，不可删除） */
const OFFICIAL_PLUGIN_REGISTRY =
  "https://raw.githubusercontent.com/woaye168/bgd_sce_plugins/main/registry.json";

/** 设置页：通用设置（代理）、项目配置（bgd.json 表单）、框架更新 */
export default function SettingsPage() {
  const [config, setConfig] = useState<BgdConfig | null>(null);
  const [appSettings, setAppSettings] = useState<AppSettings>({ proxy: "", watch_enabled: false, save_log: false, plugin_registries: [], plugin_enabled: {} });
  const [registries, setRegistries] = useState<string[]>([]);
  const [newRegistry, setNewRegistry] = useState("");
  const [registryMsg, setRegistryMsg] = useState("");
  const [testing, setTesting] = useState<Record<string, string>>({});
  const [updateInfo, setUpdateInfo] = useState<FrameworkUpdateInfo | null>(null);
  const [report, setReport] = useState<UpdateReport | null>(null);
  const [message, setMessage] = useState("");
  const [settingsMsg, setSettingsMsg] = useState("");
  const [busy, setBusy] = useState(false);

  // 进入页面即刷新配置（含 init.lock / bgd.json 最新状态）
  useEffect(() => {
    api.getConfig().then(setConfig).catch(() => setConfig(null));
    api.getAppSettings().then(setAppSettings).catch(() => {});
    api.getPluginRegistries().then(setRegistries).catch(() => {});
  });

  const persistRegistries = async (list: string[]) => {
    // 官方仓库固定第一行
    const normalized = [OFFICIAL_PLUGIN_REGISTRY, ...list.filter((u) => u !== OFFICIAL_PLUGIN_REGISTRY)];
    try {
      await api.savePluginRegistries(normalized);
      setRegistries(normalized);
      setRegistryMsg("✔ 插件仓库已保存");
    } catch (e) {
      setRegistryMsg(`✘ 保存失败: ${String(e)}`);
    }
  };

  const addRegistry = async () => {
    const url = newRegistry.trim();
    if (!url) return;
    if (registries.includes(url)) {
      setRegistryMsg("✘ 该仓库已存在");
      return;
    }
    setNewRegistry("");
    await persistRegistries([...registries, url]);
  };

  const removeRegistry = async (url: string) => {
    if (url === OFFICIAL_PLUGIN_REGISTRY) return;
    await persistRegistries(registries.filter((u) => u !== url));
  };

  const testRegistry = async (url: string) => {
    setTesting((prev) => ({ ...prev, [url]: "测试中..." }));
    try {
      const entries = await api.fetchPluginRegistry(url);
      setTesting((prev) => ({ ...prev, [url]: `✔ 可访问（${entries.length} 个插件）` }));
    } catch (e) {
      setTesting((prev) => ({ ...prev, [url]: `✘ ${String(e)}` }));
    }
  };

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
    try {
      const rep = await api.updateFramework();
      setReport(rep);
      setMessage(
        `✔ 框架已更新到 ${rep.version || "最新"}：更新 ${rep.updated}，新增 ${rep.added}，删除 ${rep.removed}，保留本地 ${rep.kept_local}，冲突 ${rep.conflicts.length}`
      );
      setUpdateInfo(null);
    } catch (e) {
      setMessage(`✘ 更新失败: ${String(e)}`);
    } finally {
      setBusy(false);
    }
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
        <button
          onClick={saveAppSettings}
          disabled={busy}
          className="mt-3 rounded-lg bg-indigo-600 px-4 py-2 text-sm text-white hover:bg-indigo-500 disabled:opacity-50"
        >
          保存
        </button>
      </Card>

      <Card title="插件仓库">
        <p className="mb-3 text-xs text-slate-500 dark:text-slate-400">
          仓库地址指向 registry.json；官方仓库固定第一行，不可删除
        </p>
        <ul className="space-y-2">
          {registries.map((url) => {
            const isOfficial = url === OFFICIAL_PLUGIN_REGISTRY;
            return (
              <li
                key={url}
                className="flex items-center gap-2 rounded-lg border border-slate-200 px-3 py-2 dark:border-slate-700"
              >
                <span className="min-w-0 flex-1 truncate font-mono text-xs text-slate-600 dark:text-slate-300">
                  {url}
                </span>
                {isOfficial && (
                  <span className="shrink-0 rounded bg-indigo-100 px-1.5 py-0.5 text-xs text-indigo-700 dark:bg-indigo-900 dark:text-indigo-300">
                    官方
                  </span>
                )}
                {testing[url] && (
                  <span className="shrink-0 text-xs text-slate-500 dark:text-slate-400">
                    {testing[url]}
                  </span>
                )}
                <button
                  onClick={() => testRegistry(url)}
                  className="shrink-0 rounded-lg border border-slate-300 px-2.5 py-1 text-xs text-slate-600 hover:bg-slate-50 dark:border-slate-600 dark:text-slate-300 dark:hover:bg-slate-700"
                >
                  测试连接
                </button>
                <button
                  onClick={() => removeRegistry(url)}
                  disabled={isOfficial}
                  title={isOfficial ? "官方仓库不可删除" : "删除"}
                  className="shrink-0 rounded-lg border border-red-300 px-2.5 py-1 text-xs text-red-600 hover:bg-red-50 disabled:cursor-not-allowed disabled:opacity-40 dark:border-red-800 dark:text-red-400 dark:hover:bg-red-950"
                >
                  删除
                </button>
              </li>
            );
          })}
        </ul>
        <div className="mt-3 flex gap-2">
          <input
            value={newRegistry}
            onChange={(e) => setNewRegistry(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && addRegistry()}
            placeholder="https://example.com/registry.json"
            className="min-w-0 flex-1 rounded-lg border border-slate-300 bg-transparent px-3 py-2 text-sm text-slate-700 dark:border-slate-600 dark:text-slate-200"
          />
          <button
            onClick={addRegistry}
            disabled={!newRegistry.trim()}
            className="shrink-0 rounded-lg bg-indigo-600 px-4 py-2 text-sm text-white hover:bg-indigo-500 disabled:opacity-50"
          >
            添加仓库
          </button>
        </div>
        {registryMsg && (
          <p className="mt-3 text-sm text-slate-600 dark:text-slate-300">{registryMsg}</p>
        )}
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
        <div className="grid grid-cols-1 gap-4 md:grid-cols-2">
          {textField("框架源码目录", "libs_dir")}
          {textField("游戏源码目录", "game_dir")}
          {textField("框架服务端产物", "libs_server_target")}
          {textField("框架客户端产物", "libs_client_target")}
          {textField("游戏服务端产物", "game_server_target")}
          {textField("游戏客户端产物", "game_client_target")}
          {textField("服务端入口", "server_entrance")}
          {textField("客户端入口", "client_entrance")}

        </div>
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
