import { useEffect, useState } from "react";
import { api } from "../lib/api";
import type { AppSettings, BgdConfig, FrameworkUpdateInfo } from "../lib/types";
import Card from "../components/Card";

/** 设置页：通用设置（代理）、项目配置（bgd.json 表单）、框架更新 */
export default function SettingsPage() {
  const [config, setConfig] = useState<BgdConfig | null>(null);
  const [appSettings, setAppSettings] = useState<AppSettings>({ proxy: "" });
  const [updateInfo, setUpdateInfo] = useState<FrameworkUpdateInfo | null>(null);
  const [message, setMessage] = useState("");
  const [settingsMsg, setSettingsMsg] = useState("");
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    api.getConfig().then(setConfig).catch(() => setConfig(null));
    api.getAppSettings().then(setAppSettings).catch(() => {});
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

  const doUpdate = async () => {
    setBusy(true);
    try {
      const msg = await api.updateFramework();
      setMessage(`✔ ${msg}`);
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
            onChange={(e) => setAppSettings({ proxy: e.target.value })}
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
        {settingsMsg && (
          <p className="mt-3 text-sm text-slate-600 dark:text-slate-300">{settingsMsg}</p>
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
          {textField("框架仓库", "framework_repo", "如 yourname/bgd-framework")}
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
            {updateInfo?.latest && updateInfo.latest !== updateInfo.current && (
              <button
                onClick={doUpdate}
                disabled={busy}
                className="rounded-lg bg-emerald-600 px-4 py-2 text-sm text-white hover:bg-emerald-500 disabled:opacity-50"
              >
                更新框架
              </button>
            )}
          </div>
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
          {textField("资源输出目录", "asset_target")}
          {textField("模板目录", "templates_dir")}
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
