import { useCallback, useEffect, useState } from "react";
import Sidebar from "./components/Sidebar";
import ProjectPage from "./pages/ProjectPage";
import BuildPage from "./pages/BuildPage";
import WatchPage from "./pages/WatchPage";
import SettingsPage from "./pages/SettingsPage";
import AboutPage from "./pages/AboutPage";
import type { PageKey } from "./lib/types";

function getInitialTheme(): "light" | "dark" {
  const saved = localStorage.getItem("bgd-theme");
  if (saved === "light" || saved === "dark") return saved;
  return "dark";
}

export default function App() {
  const [page, setPage] = useState<PageKey>("project");
  const [theme, setTheme] = useState<"light" | "dark">(getInitialTheme);
  // 用于在切换项目后强制刷新设置页
  const [projectStamp, setProjectStamp] = useState(0);

  useEffect(() => {
    document.documentElement.classList.toggle("dark", theme === "dark");
    localStorage.setItem("bgd-theme", theme);
  }, [theme]);

  const onProjectChanged = useCallback(() => {
    setProjectStamp((s) => s + 1);
  }, []);

  return (
    <div className="flex h-full bg-slate-100 text-slate-900 dark:bg-slate-950 dark:text-slate-100">
      <Sidebar
        page={page}
        onNavigate={setPage}
        theme={theme}
        onToggleTheme={() => setTheme(theme === "dark" ? "light" : "dark")}
      />
      <main className="min-w-0 flex-1 overflow-y-auto p-6">
        {page === "project" && <ProjectPage onProjectChanged={onProjectChanged} />}
        {page === "build" && <BuildPage />}
        {page === "watch" && <WatchPage />}
        {page === "settings" && <SettingsPage key={projectStamp} />}
        {page === "about" && <AboutPage />}
      </main>
    </div>
  );
}
