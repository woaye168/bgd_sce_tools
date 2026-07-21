import { useState } from "react";
import { check } from "@tauri-apps/plugin-updater";
import { api } from "../lib/api";
import Card from "../components/Card";

const APP_VERSION = "0.2.3";

/** 关于页：版本信息 + 检查更新（自动更新） */
export default function AboutPage() {
  const [message, setMessage] = useState("");
  const [busy, setBusy] = useState(false);

  const checkUpdate = async () => {
    setBusy(true);
    setMessage("正在检查更新...");
    try {
      // 读取代理设置（留空则直连）
      const settings = await api
        .getAppSettings()
        .catch(() => ({ proxy: "", watch_enabled: false }));
      const proxy = settings.proxy?.trim();
      const update = await check(proxy ? { proxy } : undefined);
      if (update) {
        setMessage(`发现新版本 ${update.version}，正在下载安装...`);
        await update.downloadAndInstall();
        setMessage("✔ 更新已下载，请重启应用完成安装");
      } else {
        setMessage("✔ 已是最新版本");
      }
    } catch (e) {
      setMessage(`✘ 检查更新失败: ${String(e)}`);
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="space-y-5">
      <Card title="bgd_sce_tools">
        <div className="space-y-2 text-sm text-slate-600 dark:text-slate-300">
          <p>版本: {APP_VERSION}</p>
          <p>BGD 工作室 · 星火编辑器 Lua 框架构建工具</p>
          <p className="text-xs text-slate-400">
            功能：项目初始化 / 全量构建 / 清除构建 / 监听更新 / 清理日志 / 框架更新
          </p>
        </div>
        <button
          onClick={checkUpdate}
          disabled={busy}
          className="mt-4 rounded-lg bg-indigo-600 px-4 py-2 text-sm text-white hover:bg-indigo-500 disabled:opacity-50"
        >
          {busy ? "检查中..." : "检查更新"}
        </button>
        {message && (
          <p className="mt-3 text-sm text-slate-600 dark:text-slate-300">{message}</p>
        )}
      </Card>
    </div>
  );
}
