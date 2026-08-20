import { useCallback, useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { api } from "../lib/api";
import type { AppInfo, InstalledApp } from "../lib/types";
import Card from "../components/Card";

/** 默认应用清单仓库（raw URL） */
const DEFAULT_REGISTRY_URL =
  "https://raw.githubusercontent.com/woaye168/bgd_sce_appsdk/main/registry.json";

/** 应用页：应用市场（远程清单）+ 已安装应用（打开/卸载） */
export default function AppPage() {
  const registryUrl = DEFAULT_REGISTRY_URL;
  const [market, setMarket] = useState<AppInfo[]>([]);
  const [installed, setInstalled] = useState<InstalledApp[]>([]);
  const [message, setMessage] = useState("");
  const [busyId, setBusyId] = useState<string | null>(null);
  const [autoStart, setAutoStart] = useState<string[]>([]);

  useEffect(() => {
    api
      .getAppSettings()
      .then((s) => setAutoStart(s.auto_start_apps ?? []))
      .catch(() => {});
  }, []);

  /** 切换应用「静默自启」（本机记忆优先；取消后 registry 下发默认不再播种） */
  const toggleAutoStart = async (id: string, on: boolean) => {
    const prev = autoStart;
    const next = on ? [...autoStart, id] : autoStart.filter((x) => x !== id);
    setAutoStart(next);
    try {
      const s = await api.getAppSettings();
      const disabled = (s.auto_start_disabled ?? []).filter((x) => x !== id);
      await api.saveAppSettings({
        ...s,
        auto_start_apps: next,
        auto_start_disabled: on ? disabled : [...disabled, id],
      });
      setMessage(on ? `✔ 已设置静默自启` : `✔ 已取消静默自启`);
    } catch (e) {
      // 乐观更新失败回滚
      setAutoStart(prev);
      setMessage(`✘ 保存静默自启配置失败: ${String(e)}`);
    }
  };

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
      // 立即显示骨架（仅 id/name），逐应用异步补全元数据（加载失败的条目隐藏）
      setMarket(reg.apps);
      setMessage(reg.apps.length ? "" : "清单中没有应用");
      reg.apps.forEach((app) => {
        api
          .enrichApp(app)
          .then((full) => {
            setMarket((prev) => prev.map((a) => (a.id === full.id ? full : a)));
          })
          .catch(() => {
            setMarket((prev) => prev.filter((a) => a.id !== app.id));
          });
      });
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

  /** 已安装版本（未安装返回 undefined） */
  const installedVersion = (id: string) =>
    installed.find((a) => a.id === id)?.version;

  /** 比较版本号：a > b 返回 1，相等 0，a < b 返回 -1（按数字段比较） */
  const compareVersion = (a: string, b: string): number => {
    const pa = a.split(".").map((n) => parseInt(n, 10) || 0);
    const pb = b.split(".").map((n) => parseInt(n, 10) || 0);
    for (let i = 0; i < Math.max(pa.length, pb.length); i++) {
      const x = pa[i] ?? 0;
      const y = pb[i] ?? 0;
      if (x !== y) return x > y ? 1 : -1;
    }
    return 0;
  };

  /** 清单版本高于已安装版本 → 可升级 */
  const hasUpdate = (app: AppInfo): boolean => {
    const v = installedVersion(app.id);
    return v !== undefined && compareVersion(app.version, v) > 0;
  };

  const install = async (app: AppInfo) => {
    setBusyId(app.id);
    setMessage(`正在安装 ${app.name}...`);
    let unlisten: (() => void) | undefined;
    try {
      // 下载进度事件：显示 已下载/总大小（M，2 位小数）
      unlisten = await listen<{ id: string; downloaded: number; total: number | null }>(
        "app-download-progress",
        (ev) => {
          if (ev.payload.id !== app.id) return;
          const done = (ev.payload.downloaded / 1048576).toFixed(2);
          const total = ev.payload.total ? (ev.payload.total / 1048576).toFixed(2) : "?";
          setMessage(`正在安装 ${app.name}... ${done}M / ${total}M`);
        },
      );
      await api.installApp(app);
      setMessage(`✔ ${app.name} 安装完成`);
      await refreshInstalled();
    } catch (e) {
      setMessage(`✘ 安装失败: ${String(e)}`);
    } finally {
      unlisten?.();
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
                <div className="ml-4 flex shrink-0 items-center gap-3">
                  <label className="flex cursor-pointer items-center gap-1 text-xs text-slate-500 dark:text-slate-400">
                    <input
                      type="checkbox"
                      checked={autoStart.includes(app.id)}
                      onChange={(e) => toggleAutoStart(app.id, e.target.checked)}
                    />
                    静默自启
                  </label>
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
                    {app.version ? (
                      <span className="ml-2 text-xs text-slate-400">v{app.version}</span>
                    ) : (
                      <span className="ml-2 text-xs text-slate-300 dark:text-slate-600">加载中…</span>
                    )}
                    {app.author && (
                      <span className="ml-2 text-xs text-slate-400">· {app.author}</span>
                    )}
                  </p>
                  {app.description && (
                    <p className="mt-0.5 truncate text-xs text-slate-500 dark:text-slate-400">
                      {app.description}
                    </p>
                  )}
                  {hasUpdate(app) && app.release_notes && (
                    <details className="mt-1 text-xs text-slate-500 dark:text-slate-400">
                      <summary className="cursor-pointer text-amber-600 dark:text-amber-400">
                        版本说明（v{app.version}）
                      </summary>
                      <pre className="mt-1 max-h-48 overflow-auto whitespace-pre-wrap rounded bg-slate-100 p-2 dark:bg-slate-800">
                        {app.release_notes}
                      </pre>
                    </details>
                  )}
                </div>
                <div className="ml-4 flex shrink-0 items-center gap-2">
                  {isInstalled(app.id) ? (
                    <>
                      {hasUpdate(app) ? (
                        <button
                          onClick={() => install(app)}
                          disabled={busyId === app.id}
                          className="rounded-lg bg-amber-600 px-3 py-1.5 text-sm text-white hover:bg-amber-500 disabled:opacity-50"
                        >
                          {busyId === app.id
                            ? "处理中..."
                            : `升级 v${installedVersion(app.id)} → v${app.version}`}
                        </button>
                      ) : (
                        <span className="text-sm text-emerald-500">已安装</span>
                      )}
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
