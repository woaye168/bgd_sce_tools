import { useCallback, useEffect, useState } from "react";
import { api } from "../lib/api";
import type { AppInfo, InstalledApp } from "../lib/types";
import Card from "../components/Card";

/** 默认应用清单仓库（raw URL） */
const DEFAULT_REGISTRY_URL =
  "https://raw.githubusercontent.com/woaye168/bgd_sce_plugins/main/registry.json";

/** 应用页：应用市场（远程清单）+ 已安装应用（打开/卸载） */
export default function AppPage() {
  const registryUrl = DEFAULT_REGISTRY_URL;
  const [market, setMarket] = useState<AppInfo[]>([]);
  const [installed, setInstalled] = useState<InstalledApp[]>([]);
  const [message, setMessage] = useState("");
  const [busyId, setBusyId] = useState<string | null>(null);

  const refreshInstalled = useCallback(async () => {
    try {
      setInstalled(await api.getInstalledApps());
    } catch (e) {
      setMessage(`✘ 读取已安装应用失败: ${String(e)}`);
    }
  }, []);

  const fetchMarket = useCallback(async () => {
    setMessage("正在拉取应用清单...");
    try {
      const reg = await api.fetchAppRegistry(registryUrl);
      setMarket(reg.apps);
      setMessage(reg.apps.length ? "" : "清单中没有应用");
    } catch (e) {
      setMessage(`✘ 拉取清单失败: ${String(e)}`);
    }
    // registryUrl 为常量，无需列入依赖
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    refreshInstalled();
    fetchMarket();
  }, [refreshInstalled, fetchMarket]);

  const isInstalled = (id: string) => installed.some((a) => a.id === id);

  const install = async (app: AppInfo) => {
    setBusyId(app.id);
    setMessage(`正在安装 ${app.name}...`);
    try {
      await api.installApp(app);
      setMessage(`✔ ${app.name} 安装完成`);
      await refreshInstalled();
    } catch (e) {
      setMessage(`✘ 安装失败: ${String(e)}`);
    } finally {
      setBusyId(null);
    }
  };

  const uninstall = async (app: InstalledApp) => {
    setBusyId(app.id);
    setMessage(`正在卸载 ${app.name}...`);
    try {
      await api.uninstallApp(app.id);
      setMessage(`✔ ${app.name} 已卸载`);
      await refreshInstalled();
    } catch (e) {
      setMessage(`✘ 卸载失败: ${String(e)}`);
    } finally {
      setBusyId(null);
    }
  };

  const open = async (app: InstalledApp) => {
    setBusyId(app.id);
    setMessage(`正在启动 ${app.name}...`);
    try {
      await api.startApp(app.id);
      setMessage(`✔ ${app.name} 已启动`);
    } catch (e) {
      setMessage(`✘ 启动失败: ${String(e)}`);
    } finally {
      setBusyId(null);
    }
  };

  return (
    <div className="space-y-5">
      <Card title="已安装应用">
        {installed.length === 0 ? (
          <p className="text-sm text-slate-500 dark:text-slate-400">
            尚未安装应用，从下方应用市场安装
          </p>
        ) : (
          <ul className="space-y-3">
            {installed.map((app) => (
              <li
                key={app.id}
                className="flex items-center justify-between rounded-lg border border-slate-200 px-4 py-3 dark:border-slate-700"
              >
                <div className="min-w-0">
                  <p className="text-sm font-medium text-slate-800 dark:text-slate-100">
                    {app.name}
                    <span className="ml-2 text-xs text-slate-400">v{app.version}</span>
                  </p>
                  {app.description && (
                    <p className="mt-0.5 truncate text-xs text-slate-500 dark:text-slate-400">
                      {app.description}
                    </p>
                  )}
                </div>
                <div className="ml-4 shrink-0">
                  <button
                    onClick={() => open(app)}
                    disabled={busyId === app.id}
                    className="rounded-lg bg-indigo-600 px-3 py-1.5 text-sm text-white hover:bg-indigo-500 disabled:opacity-50"
                  >
                    {busyId === app.id ? "处理中..." : "打开"}
                  </button>
                </div>
              </li>
            ))}
          </ul>
        )}
      </Card>

      <Card title="应用市场">
        {market.length === 0 ? (
          <p className="text-sm text-slate-500 dark:text-slate-400">
            暂无应用（检查清单 URL 或网络/代理）
          </p>
        ) : (
          <ul className="space-y-3">
            {market.map((app) => (
              <li
                key={app.id}
                className="flex items-center justify-between rounded-lg border border-slate-200 px-4 py-3 dark:border-slate-700"
              >
                <div className="min-w-0">
                  <p className="text-sm font-medium text-slate-800 dark:text-slate-100">
                    {app.name}
                    <span className="ml-2 text-xs text-slate-400">v{app.version}</span>
                    {app.author && (
                      <span className="ml-2 text-xs text-slate-400">· {app.author}</span>
                    )}
                  </p>
                  {app.description && (
                    <p className="mt-0.5 truncate text-xs text-slate-500 dark:text-slate-400">
                      {app.description}
                    </p>
                  )}
                </div>
                <div className="ml-4 flex shrink-0 items-center gap-2">
                  {isInstalled(app.id) ? (
                    <>
                      <span className="text-sm text-emerald-500">已安装</span>
                      <button
                        onClick={() =>
                          uninstall({
                            id: app.id,
                            name: app.name,
                            version: app.version,
                          })
                        }
                        disabled={busyId === app.id}
                        className="rounded-lg border border-slate-300 px-3 py-1.5 text-sm text-slate-600 hover:bg-slate-100 disabled:opacity-50 dark:border-slate-600 dark:text-slate-300 dark:hover:bg-slate-700"
                      >
                        {busyId === app.id ? "处理中..." : "卸载"}
                      </button>
                    </>
                  ) : (
                    <button
                      onClick={() => install(app)}
                      disabled={busyId === app.id}
                      className="rounded-lg bg-indigo-600 px-3 py-1.5 text-sm text-white hover:bg-indigo-500 disabled:opacity-50"
                    >
                      {busyId === app.id ? "安装中..." : "安装"}
                    </button>
                  )}
                </div>
              </li>
            ))}
          </ul>
        )}
      </Card>

      {message && (
        <p className="text-sm text-slate-600 dark:text-slate-300">{message}</p>
      )}
    </div>
  );
}
