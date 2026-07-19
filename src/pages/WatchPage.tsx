import { useEffect, useState } from "react";
import { api } from "../lib/api";
import Card from "../components/Card";
import LogPanel from "../components/LogPanel";

/** 监听页：监听开关 + 实时事件流 */
export default function WatchPage() {
  const [watching, setWatching] = useState(false);
  const [message, setMessage] = useState("");

  useEffect(() => {
    api.isWatching().then(setWatching).catch(() => {});
  }, []);

  const toggle = async () => {
    setMessage("");
    try {
      if (watching) {
        await api.stopWatch();
        setWatching(false);
      } else {
        await api.startWatch();
        setWatching(true);
      }
    } catch (e) {
      setMessage(`✘ ${String(e)}`);
    }
  };

  return (
    <div className="flex h-full flex-col space-y-5">
      <Card title="监听更新">
        <div className="flex items-center gap-4">
          <button
            onClick={toggle}
            className={`relative h-7 w-14 rounded-full transition-colors ${
              watching ? "bg-emerald-500" : "bg-slate-400"
            }`}
          >
            <span
              className={`absolute top-1 h-5 w-5 rounded-full bg-white transition-all ${
                watching ? "left-8" : "left-1"
              }`}
            />
          </button>
          <span className="text-sm text-slate-600 dark:text-slate-300">
            {watching ? "监听中：源码修改将实时增量构建" : "监听已停止"}
          </span>
        </div>
        {message && (
          <p className="mt-3 text-sm text-slate-600 dark:text-slate-300">{message}</p>
        )}
      </Card>

      <Card title="事件流" className="flex min-h-0 flex-1 flex-col">
        <LogPanel source="watch" className="min-h-0 flex-1" />
      </Card>
    </div>
  );
}
