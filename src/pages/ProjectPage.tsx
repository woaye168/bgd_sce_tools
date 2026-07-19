import { useEffect, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { api } from "../lib/api";
import type { ProjectInfo } from "../lib/types";
import Card from "../components/Card";

interface ProjectPageProps {
  onProjectChanged: () => void;
}

/** 项目页：选择/打开项目、最近项目、初始化项目 */
export default function ProjectPage({ onProjectChanged }: ProjectPageProps) {
  const [current, setCurrent] = useState<string | null>(null);
  const [recent, setRecent] = useState<string[]>([]);
  const [info, setInfo] = useState<ProjectInfo | null>(null);
  const [repo, setRepo] = useState("");
  const [message, setMessage] = useState("");
  const [busy, setBusy] = useState(false);

  const refresh = async () => {
    setCurrent(await api.getCurrentProject());
    setRecent(await api.getRecentProjects());
    try {
      setInfo(await api.getProjectInfo());
    } catch {
      setInfo(null);
    }
  };

  useEffect(() => {
    refresh();
  }, []);

  const pickAndSelect = async () => {
    const selected = await open({ directory: true, title: "选择 SCE 项目文件夹" });
    if (typeof selected === "string") {
      setRecent(await api.selectProject(selected));
      await refresh();
      onProjectChanged();
    }
  };

  const selectPath = async (path: string) => {
    setRecent(await api.selectProject(path));
    await refresh();
    onProjectChanged();
  };

  const initProject = async () => {
    const selected = await open({ directory: true, title: "选择要初始化的项目文件夹" });
    if (typeof selected !== "string") return;

    const doInit = async (force: boolean) => {
      setBusy(true);
      setMessage("正在初始化（下载框架中，请稍候）...");
      try {
        const msg = await api.initProject(selected, repo.trim(), force);
        setMessage(`✔ ${msg}`);
        await refresh();
        onProjectChanged();
      } catch (e) {
        const errText = String(e);
        // 初始化锁：确认后强制解锁重试
        if (!force && errText.includes("已初始化")) {
          const ok = window.confirm(
            "该项目已初始化（存在 init.lock）。\n\n重新初始化将覆盖 .bgd 目录（旧目录会自动备份为 .bgd.bak-时间戳）。\n\n是否继续？"
          );
          if (ok) {
            await doInit(true);
            return;
          }
          setMessage("已取消初始化");
        } else {
          setMessage(`✘ 初始化失败: ${errText}`);
        }
      } finally {
        setBusy(false);
      }
    };

    await doInit(false);
  };

  return (
    <div className="space-y-5">
      <Card title="当前项目">
        {current ? (
          <div className="space-y-2">
            <p className="break-all text-sm text-slate-700 dark:text-slate-200">{current}</p>
            <p className="text-xs text-slate-500 dark:text-slate-400">
              {info?.initialized
                ? `框架版本: ${info.framework_version || "未知"}`
                : "未初始化（缺少 .bgd）"}
            </p>
          </div>
        ) : (
          <p className="text-sm text-slate-500 dark:text-slate-400">尚未选择项目</p>
        )}
        <button
          onClick={pickAndSelect}
          className="mt-4 rounded-lg bg-indigo-600 px-4 py-2 text-sm text-white hover:bg-indigo-500"
        >
          选择项目文件夹
        </button>
      </Card>

      <Card title="最近项目">
        {recent.length === 0 ? (
          <p className="text-sm text-slate-500 dark:text-slate-400">暂无记录</p>
        ) : (
          <ul className="space-y-1">
            {recent.map((path) => (
              <li key={path}>
                <button
                  onClick={() => selectPath(path)}
                  className={`w-full truncate rounded-lg px-3 py-2 text-left text-sm transition-colors ${
                    path === current
                      ? "bg-indigo-50 text-indigo-700 dark:bg-indigo-950 dark:text-indigo-300"
                      : "text-slate-600 hover:bg-slate-100 dark:text-slate-300 dark:hover:bg-slate-700"
                  }`}
                >
                  {path}
                </button>
              </li>
            ))}
          </ul>
        )}
      </Card>

      <Card title="初始化新项目">
        <p className="mb-3 text-sm text-slate-500 dark:text-slate-400">
          从框架仓库下载最新框架，生成 .bgd 目录、.emmyrc.json、.gitignore。
        </p>
        <input
          value={repo}
          onChange={(e) => setRepo(e.target.value)}
          placeholder="框架仓库（如 yourname/bgd-framework，留空用默认）"
          className="mb-3 w-full rounded-lg border border-slate-300 bg-transparent px-3 py-2 text-sm text-slate-700 dark:border-slate-600 dark:text-slate-200"
        />
        <button
          onClick={initProject}
          disabled={busy}
          className="rounded-lg bg-emerald-600 px-4 py-2 text-sm text-white hover:bg-emerald-500 disabled:opacity-50"
        >
          {busy ? "初始化中..." : "初始化项目"}
        </button>
        {message && (
          <p className="mt-3 text-sm text-slate-600 dark:text-slate-300">{message}</p>
        )}
      </Card>
    </div>
  );
}
