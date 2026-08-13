import { useEffect, useState } from "react";
import { getVersion } from "@tauri-apps/api/app";
import { api } from "../lib/api";
import Card from "../components/Card";

/** 关于页：版本信息 + 检查更新（自建逻辑：私有仓库下 tauri updater 插件无法携带 token） */
export default function AboutPage() {
  const [message, setMessage] = useState("");
  const [busy, setBusy] = useState(false);
  const [appVersion, setAppVersion] = useState("...");

  // 版本号来自 tauri.conf.json（CI 构建时由 git tag 注入），不再硬编码
  useEffect(() => {
    getVersion().then(setAppVersion).catch(() => setAppVersion("unknown"));
  }, []);

  const checkUpdate = async () => {
    setBusy(true);
    setMessage("正在检查更新...");
    try {
      const current = await getVersion();
      const info = await api.checkSelfUpdate(current);
      if (info.has_update) {
        setMessage(`发现新版本 v${info.latest}，正在下载安装包...`);
        await api.startSelfUpdate();
        setMessage("✔ 安装器已启动，请按提示完成安装");
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
          <p>版本: {appVersion}</p>
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
