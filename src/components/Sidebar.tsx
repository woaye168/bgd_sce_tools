import type { PageKey } from "../lib/types";

interface SidebarProps {
  page: PageKey;
  onNavigate: (page: PageKey) => void;
  theme: "light" | "dark";
  onToggleTheme: () => void;
}

const NAV_ITEMS: { key: PageKey; label: string }[] = [
  { key: "project", label: "项目" },
  { key: "build", label: "构建" },
  { key: "watch", label: "监听" },
  { key: "plugins", label: "插件" },
  { key: "settings", label: "设置" },
  { key: "about", label: "关于" },
];

/** 侧边栏导航（PilotDeck 风格：深色栏 + 高亮当前项） */
export default function Sidebar({ page, onNavigate, theme, onToggleTheme }: SidebarProps) {
  return (
    <aside className="flex h-full w-52 flex-col bg-slate-900 text-slate-200">
      <div className="px-5 py-5">
        <h1 className="text-sm font-bold tracking-wider text-white">bgd_sce_tools</h1>
        <p className="mt-1 text-xs text-slate-400">SCE 框架构建工具</p>
      </div>
      <nav className="flex-1 space-y-1 px-3">
        {NAV_ITEMS.map((item) => (
          <button
            key={item.key}
            onClick={() => onNavigate(item.key)}
            className={`flex w-full items-center gap-3 rounded-lg px-3 py-2.5 text-sm transition-colors ${
              page === item.key
                ? "bg-indigo-600 text-white"
                : "text-slate-300 hover:bg-slate-800"
            }`}
          >
            <span
              className={`h-2 w-2 rounded-full ${
                page === item.key ? "bg-white" : "bg-slate-500"
              }`}
            />
            {item.label}
          </button>
        ))}
      </nav>
      <div className="px-3 pb-5">
        <button
          onClick={onToggleTheme}
          className="flex w-full items-center gap-3 rounded-lg px-3 py-2.5 text-sm text-slate-300 hover:bg-slate-800"
        >
          <span
            className={`h-3 w-3 rounded-full border-2 ${
              theme === "dark"
                ? "border-amber-300 bg-amber-300"
                : "border-slate-400 bg-transparent"
            }`}
          />
          {theme === "dark" ? "浅色模式" : "深色模式"}
        </button>
      </div>
    </aside>
  );
}
