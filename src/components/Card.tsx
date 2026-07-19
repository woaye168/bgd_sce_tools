import type { ReactNode } from "react";

interface CardProps {
  title: string;
  children: ReactNode;
  className?: string;
}

/** 内容卡片（PilotDeck 风格：圆角白底/暗色底 + 细边框） */
export default function Card({ title, children, className = "" }: CardProps) {
  return (
    <section
      className={`rounded-xl border border-slate-200 bg-white p-5 shadow-sm dark:border-slate-700 dark:bg-slate-800 ${className}`}
    >
      <h2 className="mb-4 text-base font-semibold text-slate-800 dark:text-slate-100">
        {title}
      </h2>
      {children}
    </section>
  );
}
