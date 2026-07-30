import { useCallback, useEffect, useState } from "react";
import { api, onPluginInstallProgress } from "../lib/api";
import type { PluginInfo, RegistryEntry } from "../lib/types";
import Card from "../components/Card";

/** 从 registry.json URL 提取显示名（如 woaye168/bgd_sce_plugins） */
function registryLabel(url: string): string {
  const m = url.match(/raw\.githubusercontent\.com\/([^/]+\/[^/]+)\//);
  if (m) return m[1];
  const g = url.match(/github\.com\/([^/]+\/[^/]+)/);
  if (g) return g[1].replace(/\.git$/, "");
  try {
    return new URL(url).host;
  } catch {
    return url;
  }
}

interface MarketState {
  loading: boolean;
  error: string;
  entries: RegistryEntry[];
}

/** 注入插件 UI iframe 的桥接脚本：window.bgdPlugin → postMessage 转发给宿主 */
const BRIDGE_SCRIPT = `<script>
window.bgdPlugin = {
  register: function (modules) {
    window.parent.postMessage({ __bgdPlugin: true, action: "register", payload: modules }, "*");
  },
  uninstall: function () {
    window.parent.postMessage({ __bgdPlugin: true, action: "uninstall", payload: null }, "*");
  },
  saveSettings: function (settings) {
    window.parent.postMessage({ __bgdPlugin: true, action: "saveSettings", payload: settings }, "*");
  },
  scanModules: function () {
    return new Promise(function (resolve, reject) {
      var handler = function (e) {
        var d = e.data;
        if (d && d.__bgdPluginScan === true) {
          window.removeEventListener("message", handler);
          if (d.error) { reject(new Error(d.error)); } else { resolve(d.modules); }
        }
      };
      window.addEventListener("message", handler);
      window.parent.postMessage({ __bgdPlugin: true, action: "scanModules", payload: null }, "*");
    });
  }
};
</script>`;

/** 打开中的插件 UI（模态框内 iframe 展示） */
interface PluginUiState {
  id: string;
  name: string;
  html: string;
}

/** 插件管理页：已安装 + 各仓库市场选项卡 */
export default function PluginPage() {
  const [registries, setRegistries] = useState<string[]>([]);
  const [installed, setInstalled] = useState<PluginInfo[] | null>(null);
  const [installedErr, setInstalledErr] = useState("");
  const [activeTab, setActiveTab] = useState<string>("installed");
  const [market, setMarket] = useState<Record<string, MarketState>>({});
  const [progress, setProgress] = useState<Record<string, { downloaded: number; total: number }>>({});
  const [installing, setInstalling] = useState<Record<string, boolean>>({});
  const [needRestart, setNeedRestart] = useState(false);
  const [message, setMessage] = useState("");
  const [pluginUi, setPluginUi] = useState<PluginUiState | null>(null);

  const refreshInstalled = useCallback(() => {
    api
      .getInstalledPlugins()
      .then((list) => {
        setInstalled(list);
        setInstalledErr("");
      })
      .catch((e) => {
        setInstalled(null);
        setInstalledErr(String(e));
      });
  }, []);

  useEffect(() => {
    api.getPluginRegistries().then(setRegistries).catch(() => {});
    refreshInstalled();
    const unlisten = onPluginInstallProgress((p) => {
      setProgress((prev) => ({ ...prev, [p.id]: { downloaded: p.downloaded, total: p.total } }));
    });
    return () => {
      unlisten.then((f) => f());
    };
  }, [refreshInstalled]);

  const refreshMarket = useCallback((url: string) => {
    setMarket((prev) => ({ ...prev, [url]: { loading: true, error: "", entries: prev[url]?.entries ?? [] } }));
    api
      .fetchPluginRegistry(url)
      .then((entries) =>
        setMarket((prev) => ({ ...prev, [url]: { loading: false, error: "", entries } }))
      )
      .catch((e) =>
        setMarket((prev) => ({ ...prev, [url]: { loading: false, error: String(e), entries: [] } }))
      );
  }, []);

  // 切换到某仓库选项卡时自动拉取一次
  useEffect(() => {
    if (activeTab !== "installed" && !market[activeTab]) {
      refreshMarket(activeTab);
    }
  }, [activeTab, market, refreshMarket]);

  const toggleEnabled = async (p: PluginInfo) => {
    try {
      await api.enablePlugin(p.id, !p.enabled);
      refreshInstalled();
      setNeedRestart(true);
    } catch (e) {
      setMessage(`✘ 操作失败: ${String(e)}`);
    }
  };

  const uninstall = async (p: PluginInfo) => {
    try {
      await api.uninstallPlugin(p.id);
      refreshInstalled();
      setNeedRestart(true);
      setMessage(`✔ 已卸载 ${p.name}`);
    } catch (e) {
      setMessage(`✘ 卸载失败: ${String(e)}`);
    }
  };

  const openPluginUi = async (p: PluginInfo) => {
    setMessage("");
    try {
      const html = await api.getPluginUi(p.id);
      setPluginUi({ id: p.id, name: p.name, html });
    } catch (e) {
      setMessage(`✘ 打开插件界面失败: ${String(e)}`);
    }
  };

  // 接收插件 UI iframe 的桥接消息（window.bgdPlugin → postMessage），转发到后端
  useEffect(() => {
    if (!pluginUi) return;
    const handler = (e: MessageEvent) => {
      const d = e.data as { __bgdPlugin?: boolean; action?: string; payload?: unknown } | null;
      if (!d || d.__bgdPlugin !== true || typeof d.action !== "string") return;

      // scanModules：宿主直接扫描文件系统，不经过插件
      if (d.action === "scanModules") {
        api
          .scanApiModules()
          .then((modules) => {
            // 回传扫描结果给 iframe
            const iframe = document.querySelector('iframe[data-plugin-ui]') as HTMLIFrameElement | null;
            iframe?.contentWindow?.postMessage({ __bgdPluginScan: true, modules }, "*");
          })
          .catch((err) => {
            const iframe = document.querySelector('iframe[data-plugin-ui]') as HTMLIFrameElement | null;
            iframe?.contentWindow?.postMessage({ __bgdPluginScan: true, error: String(err) }, "*");
          });
        return;
      }

      api
        .pluginAction(pluginUi.id, d.action, JSON.stringify(d.payload ?? {}))
        .catch((err) => setMessage(`✘ 插件操作失败: ${String(err)}`));
    };
    window.addEventListener("message", handler);
    return () => window.removeEventListener("message", handler);
  }, [pluginUi]);

  const install = async (entry: RegistryEntry) => {
    setInstalling((prev) => ({ ...prev, [entry.id]: true }));
    setProgress((prev) => ({ ...prev, [entry.id]: { downloaded: 0, total: 0 } }));
    setMessage("");
    try {
      await api.installPlugin(entry);
      refreshInstalled();
      setNeedRestart(true);
      setMessage(`✔ ${entry.name} 安装完成`);
    } catch (e) {
      setMessage(`✘ 安装失败: ${String(e)}`);
    } finally {
      setInstalling((prev) => ({ ...prev, [entry.id]: false }));
      setProgress((prev) => {
        const next = { ...prev };
        delete next[entry.id];
        return next;
      });
    }
  };

  const installedOf = (id: string) => installed?.find((p) => p.id === id);

  const tabs = [
    { key: "installed", label: "已安装" },
    ...registries.map((url) => ({ key: url, label: registryLabel(url) })),
  ];

  return (
    <div className="space-y-5">
      {needRestart && (
        <div className="flex items-center justify-between rounded-xl border border-amber-300 bg-amber-50 p-4 dark:border-amber-700 dark:bg-amber-950">
          <p className="text-sm text-amber-800 dark:text-amber-300">
            插件变更需重启后加载生效
          </p>
          <button
            onClick={() => api.restartApp().catch(() => {})}
            className="rounded-lg bg-amber-600 px-4 py-2 text-sm text-white hover:bg-amber-500"
          >
            重启
          </button>
        </div>
      )}

      <Card title="插件">
        {/* 选项卡 */}
        <div className="mb-4 flex flex-wrap gap-1 border-b border-slate-200 dark:border-slate-700">
          {tabs.map((t) => (
            <button
              key={t.key}
              onClick={() => setActiveTab(t.key)}
              className={`-mb-px border-b-2 px-4 py-2 text-sm transition-colors ${
                activeTab === t.key
                  ? "border-indigo-600 font-medium text-indigo-600 dark:text-indigo-400"
                  : "border-transparent text-slate-500 hover:text-slate-700 dark:text-slate-400 dark:hover:text-slate-200"
              }`}
            >
              {t.label}
            </button>
          ))}
        </div>

        {activeTab === "installed" && (
          <>
            {installedErr && (
              <p className="text-sm text-slate-500 dark:text-slate-400">
                请先在「项目」页选择一个已初始化的项目（{installedErr}）
              </p>
            )}
            {installed && installed.length === 0 && (
              <p className="text-sm text-slate-500 dark:text-slate-400">
                尚未安装插件，切换到仓库选项卡浏览可安装的插件
              </p>
            )}
            {installed && installed.length > 0 && (
              <ul className="space-y-3">
                {installed.map((p) => (
                  <li
                    key={p.id}
                    className="flex items-center justify-between rounded-lg border border-slate-200 p-4 dark:border-slate-700"
                  >
                    <div className="min-w-0">
                      <div className="flex items-center gap-2">
                        <span className="text-sm font-medium text-slate-800 dark:text-slate-100">
                          {p.name}
                        </span>
                        {p.version && (
                          <span className="rounded bg-slate-100 px-1.5 py-0.5 text-xs text-slate-500 dark:bg-slate-700 dark:text-slate-300">
                            v{p.version}
                          </span>
                        )}
                        {!p.enabled && (
                          <span className="rounded bg-slate-100 px-1.5 py-0.5 text-xs text-slate-400 dark:bg-slate-700">
                            已禁用
                          </span>
                        )}
                      </div>
                      {p.description && (
                        <p className="mt-1 truncate text-xs text-slate-500 dark:text-slate-400">
                          {p.description}
                        </p>
                      )}
                      <p className="mt-0.5 text-xs text-slate-400">
                        {p.id}
                        {p.author && ` · ${p.author}`}
                      </p>
                    </div>
                    <div className="ml-4 flex shrink-0 items-center gap-3">
                      {p.has_ui && (
                        <button
                          onClick={() => openPluginUi(p)}
                          className="rounded-lg border border-indigo-300 px-3 py-1.5 text-sm text-indigo-600 hover:bg-indigo-50 dark:border-indigo-800 dark:text-indigo-400 dark:hover:bg-indigo-950"
                        >
                          打开
                        </button>
                      )}
                      <button
                        onClick={() => toggleEnabled(p)}
                        title={p.enabled ? "禁用" : "启用"}
                        className={`relative h-6 w-11 rounded-full transition-colors ${
                          p.enabled ? "bg-emerald-500" : "bg-slate-300 dark:bg-slate-600"
                        }`}
                      >
                        <span
                          className={`absolute top-0.5 h-5 w-5 rounded-full bg-white shadow transition-all ${
                            p.enabled ? "left-[calc(100%-22px)]" : "left-0.5"
                          }`}
                        />
                      </button>
                      <button
                        onClick={() => uninstall(p)}
                        className="rounded-lg border border-red-300 px-3 py-1.5 text-sm text-red-600 hover:bg-red-50 dark:border-red-800 dark:text-red-400 dark:hover:bg-red-950"
                      >
                        卸载
                      </button>
                    </div>
                  </li>
                ))}
              </ul>
            )}
          </>
        )}

        {activeTab !== "installed" && (
          <>
            <div className="mb-3 flex items-center justify-between">
              <p className="truncate text-xs text-slate-400">{activeTab}</p>
              <button
                onClick={() => refreshMarket(activeTab)}
                disabled={market[activeTab]?.loading}
                className="shrink-0 rounded-lg border border-slate-300 px-3 py-1.5 text-sm text-slate-600 hover:bg-slate-50 disabled:opacity-50 dark:border-slate-600 dark:text-slate-300 dark:hover:bg-slate-700"
              >
                {market[activeTab]?.loading ? "刷新中..." : "刷新"}
              </button>
            </div>
            {market[activeTab]?.error && (
              <p className="text-sm text-red-600 dark:text-red-400">
                ✘ 拉取仓库失败: {market[activeTab].error}
              </p>
            )}
            {!market[activeTab]?.error &&
              !market[activeTab]?.loading &&
              (market[activeTab]?.entries.length ?? 0) === 0 && (
                <p className="text-sm text-slate-500 dark:text-slate-400">该仓库暂无可安装插件</p>
              )}
            <ul className="space-y-3">
              {(market[activeTab]?.entries ?? []).map((entry) => {
                const inst = installedOf(entry.id);
                const hasUpdate = inst && inst.version !== entry.version;
                const prog = progress[entry.id];
                const pct = prog && prog.total > 0 ? Math.min(100, Math.round((prog.downloaded / prog.total) * 100)) : 0;
                return (
                  <li
                    key={entry.id}
                    className="rounded-lg border border-slate-200 p-4 dark:border-slate-700"
                  >
                    <div className="flex items-center justify-between">
                      <div className="min-w-0">
                        <div className="flex items-center gap-2">
                          <span className="text-sm font-medium text-slate-800 dark:text-slate-100">
                            {entry.name}
                          </span>
                          <span className="rounded bg-slate-100 px-1.5 py-0.5 text-xs text-slate-500 dark:bg-slate-700 dark:text-slate-300">
                            v{entry.version}
                          </span>
                          {inst && !hasUpdate && (
                            <span className="rounded bg-emerald-100 px-1.5 py-0.5 text-xs text-emerald-700 dark:bg-emerald-900 dark:text-emerald-300">
                              已安装
                            </span>
                          )}
                        </div>
                        {entry.description && (
                          <p className="mt-1 truncate text-xs text-slate-500 dark:text-slate-400">
                            {entry.description}
                          </p>
                        )}
                        <p className="mt-0.5 text-xs text-slate-400">
                          {entry.id}
                          {entry.author && ` · ${entry.author}`}
                          {inst && hasUpdate && ` · 当前 v${inst.version}`}
                        </p>
                      </div>
                      <div className="ml-4 shrink-0">
                        {(!inst || hasUpdate) && (
                          <button
                            onClick={() => install(entry)}
                            disabled={installing[entry.id]}
                            className="rounded-lg bg-indigo-600 px-4 py-2 text-sm text-white hover:bg-indigo-500 disabled:opacity-50"
                          >
                            {installing[entry.id] ? "下载中..." : hasUpdate ? "更新" : "安装"}
                          </button>
                        )}
                      </div>
                    </div>
                    {prog && (
                      <div className="mt-3">
                        <div className="h-2 w-full overflow-hidden rounded-full bg-slate-200 dark:bg-slate-700">
                          <div
                            className="h-full bg-indigo-600 transition-all"
                            style={{ width: `${prog.total > 0 ? pct : 100}%` }}
                          />
                        </div>
                        <p className="mt-1 text-xs text-slate-400">
                          {prog.total > 0
                            ? `${pct}%（${(prog.downloaded / 1024).toFixed(0)} / ${(prog.total / 1024).toFixed(0)} KB）`
                            : `已下载 ${(prog.downloaded / 1024).toFixed(0)} KB`}
                        </p>
                      </div>
                    )}
                  </li>
                );
              })}
            </ul>
          </>
        )}

        {message && (
          <p className="mt-3 text-sm text-slate-600 dark:text-slate-300">{message}</p>
        )}
      </Card>

      {/* 插件 UI 模态框（iframe srcdoc 内嵌 render_ui() 返回的 HTML） */}
      {pluginUi && (
        <div
          className="fixed inset-0 z-50 flex items-center justify-center bg-black/50 p-6"
          onClick={() => setPluginUi(null)}
        >
          <div
            className="flex max-h-full w-full max-w-3xl flex-col overflow-hidden rounded-xl bg-white shadow-xl dark:bg-slate-800"
            onClick={(e) => e.stopPropagation()}
          >
            <div className="flex items-center justify-between border-b border-slate-200 px-4 py-3 dark:border-slate-700">
              <h2 className="text-sm font-medium text-slate-800 dark:text-slate-100">
                {pluginUi.name}
              </h2>
              <button
                onClick={() => setPluginUi(null)}
                className="rounded-lg px-2 py-1 text-sm text-slate-500 hover:bg-slate-100 dark:text-slate-400 dark:hover:bg-slate-700"
              >
                关闭
              </button>
            </div>
            <iframe
              title={`插件界面 - ${pluginUi.name}`}
              sandbox="allow-scripts"
              className="h-[70vh] w-full border-0 bg-white"
              srcDoc={BRIDGE_SCRIPT + pluginUi.html}
            />
          </div>
        </div>
      )}
    </div>
  );
}
