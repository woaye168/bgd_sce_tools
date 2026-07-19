import { useEffect, useRef, useState } from "react";
import { onLog } from "../lib/api";
import type { LogEvent } from "../lib/types";

interface LogPanelProps {
  /** 只显示该来源的日志；不传则显示全部 */
  source?: "build" | "watch";
  className?: string;
}

/** 实时日志面板：订阅后端 bgd-log 事件，自动滚动到底部 */
export default function LogPanel({ source, className = "" }: LogPanelProps) {
  const [lines, setLines] = useState<string[]>([]);
  const bottomRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const unlisten = onLog((event: LogEvent) => {
      if (source && event.source !== source) return;
      const time = new Date().toLocaleTimeString("zh-CN", { hour12: false });
      setLines((prev) => [...prev.slice(-999), `[${time}] ${event.line}`]);
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, [source]);

  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [lines]);

  return (
    <div
      className={`log-scroll overflow-y-auto rounded-lg bg-slate-950 p-4 font-mono text-xs leading-5 text-emerald-300 ${className}`}
    >
      {lines.length === 0 ? (
        <span className="text-slate-500">暂无日志输出</span>
      ) : (
        lines.map((line, i) => <div key={i}>{line}</div>)
      )}
      <div ref={bottomRef} />
    </div>
  );
}
