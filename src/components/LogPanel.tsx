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
  const [lines, setLines] = useState<{ id: number; text: string }[]>([]);
  const bottomRef = useRef<HTMLDivElement>(null);
  const nextId = useRef(0);
  const scrollScheduled = useRef(false);

  useEffect(() => {
    const unlisten = onLog((event: LogEvent) => {
      if (source && event.source !== source) return;
      const time = new Date().toLocaleTimeString("zh-CN", { hour12: false });
      setLines((prev) => [
        ...prev.slice(-999),
        { id: ++nextId.current, text: `[${time}] ${event.line}` },
      ]);
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, [source]);

  // 滚动节流：高频日志下每帧最多滚动一次
  useEffect(() => {
    if (scrollScheduled.current) return;
    scrollScheduled.current = true;
    requestAnimationFrame(() => {
      scrollScheduled.current = false;
      bottomRef.current?.scrollIntoView({ behavior: "smooth" });
    });
  }, [lines]);

  return (
    <div
      className={`log-scroll overflow-y-auto rounded-lg bg-slate-950 p-4 font-mono text-xs leading-5 text-emerald-300 ${className}`}
    >
      {lines.length === 0 ? (
        <span className="text-slate-500">暂无日志输出</span>
      ) : (
        lines.map((line) => (
          <div
            key={line.id}
            className={
              line.text.includes("[warn]")
                ? "text-amber-400"
                : line.text.includes("[error]")
                  ? "text-red-400"
                  : undefined
            }
          >
            {line.text}
          </div>
        ))
      )}
      <div ref={bottomRef} />
    </div>
  );
}
