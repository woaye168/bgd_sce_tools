import { useState } from "react";
import { api } from "../lib/api";
import Card from "../components/Card";
import LogPanel from "../components/LogPanel";

/** 构建页：全量构建 / 清除构建 / 清理日志 + 实时日志 */
export default function BuildPage() {
  const [busy, setBusy] = useState<string | null>(null);
  const [message, setMessage] = useState("");

  const run = async (name: string, action: () => Promise<unknown>) => {
    setBusy(name);
    setMessage("");
    try {
      const result = await action();
      setMessage(
        typeof result === "number" ? `✔ 完成，共清理 ${result} 个日志文件` : "✔ 完成"
      );
    } catch (e) {
      setMessage(`✘ 失败: ${String(e)}`);
    } finally {
      setBusy(null);
    }
  };

  return (
    <div className="flex h-full flex-col space-y-5">
      <Card title="构建操作">
        <div className="flex flex-wrap gap-3">
          <button
            onClick={() => run("build", api.fullBuild)}
            disabled={busy !== null}
            className="rounded-lg bg-indigo-600 px-4 py-2 text-sm text-white hover:bg-indigo-500 disabled:opacity-50"
          >
            {busy === "build" ? "构建中..." : "全量构建"}
          </button>
          <button
            onClick={() => run("clean", api.cleanBuild)}
            disabled={busy !== null}
            className="rounded-lg bg-rose-600 px-4 py-2 text-sm text-white hover:bg-rose-500 disabled:opacity-50"
          >
            {busy === "clean" ? "清理中..." : "清除构建"}
          </button>
          <button
            onClick={() => run("logs", api.cleanLogs)}
            disabled={busy !== null}
            className="rounded-lg bg-slate-600 px-4 py-2 text-sm text-white hover:bg-slate-500 disabled:opacity-50"
          >
            {busy === "logs" ? "清理中..." : "清理日志"}
          </button>
        </div>
        {message && (
          <p className="mt-3 text-sm text-slate-600 dark:text-slate-300">{message}</p>
        )}
      </Card>

      <Card title="构建输出" className="flex min-h-0 flex-1 flex-col">
        <LogPanel source="build" className="min-h-0 flex-1" />
      </Card>
    </div>
  );
}
