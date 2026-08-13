import { Component, lazy, Suspense, type ErrorInfo, type ReactNode, useEffect, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { Titlebar } from "./components/Titlebar";
import { Sidebar } from "./components/Sidebar";
import { UpdateBar, UpdateInstallProvider } from "./components/UpdateBar";
import { AppProvider, useNav } from "./state";
import { HomePage } from "./pages/Home";
import { ToastShell } from "./components/ToastShell";
import { TrayShell } from "./components/TrayShell";
import { WorkspaceApp } from "./workspace/WorkspaceApp";
import { api } from "./lib/api";
import type { PageId } from "./types";
import { SettingsPage } from "./pages/Settings";
import { InstallPage } from "./pages/Install";

const ToolchainPage = lazy(() => import("./pages/Toolchain").then((m) => ({ default: m.ToolchainPage })));
const StudiosPage = lazy(() => import("./pages/Studios").then((m) => ({ default: m.StudiosPage })));
const PluginsPage = lazy(() => import("./pages/Plugins").then((m) => ({ default: m.PluginsPage })));
const DoctorPage = lazy(() => import("./pages/Doctor").then((m) => ({ default: m.DoctorPage })));
const ProjectsPage = lazy(() => import("./pages/Projects").then((m) => ({ default: m.ProjectsPage })));
const PlaygroundPage = lazy(() => import("./pages/Playground").then((m) => ({ default: m.PlaygroundPage })));
const HexPage = lazy(() => import("./pages/Hex").then((m) => ({ default: m.HexPage })));
const LearnPage = lazy(() => import("./pages/Learn").then((m) => ({ default: m.LearnPage })));

function PageFallback() {
  return <div className="page-enter p-6 text-sm text-mist-300">…</div>;
}

function Lazy({ children }: { children: ReactNode }) {
  return <Suspense fallback={<PageFallback />}>{children}</Suspense>;
}

function PageView({ page }: { page: PageId }) {
  switch (page) {
    case "home":
      return <HomePage />;
    case "install":
      return <InstallPage />;
    case "toolchain":
      return (
        <Lazy>
          <ToolchainPage />
        </Lazy>
      );
    case "studios":
      return (
        <Lazy>
          <StudiosPage />
        </Lazy>
      );
    case "plugins":
      return (
        <Lazy>
          <PluginsPage />
        </Lazy>
      );
    case "doctor":
      return (
        <Lazy>
          <DoctorPage />
        </Lazy>
      );
    case "projects":
      return (
        <Lazy>
          <ProjectsPage />
        </Lazy>
      );
    case "playground":
      return (
        <Lazy>
          <PlaygroundPage />
        </Lazy>
      );
    case "hex":
      return (
        <Lazy>
          <HexPage />
        </Lazy>
      );
    case "learn":
      return (
        <Lazy>
          <LearnPage />
        </Lazy>
      );
    case "settings":
      return <SettingsPage />;
  }
}

function Shell() {
  const { page } = useNav();

  return (
    <UpdateInstallProvider>
      <div className="aurora flex h-full flex-col">
        <div className="grain" aria-hidden />
        <Titlebar />
        <div className="flex min-h-0 flex-1">
          <Sidebar />
          <main className="relative flex min-h-0 min-w-0 flex-1 flex-col">
            <UpdateBar />
            <div
              className={
                page === "hex" || page === "projects"
                  ? "flex min-h-0 flex-1 flex-col overflow-hidden"
                  : "min-h-0 flex-1 overflow-y-auto"
              }
            >
              <PageView page={page} />
            </div>
          </main>
        </div>
      </div>
    </UpdateInstallProvider>
  );
}

function detectShell(): "toast" | "tray" | "workspace" | "main" {
  const fromWin = window.__ELIN_SHELL;
  if (fromWin === "toast" || fromWin === "tray" || fromWin === "workspace") return fromWin;
  try {
    const label = getCurrentWindow().label;
    if (label.startsWith("ws-")) return "workspace";
    if (label === "toast") return "toast";
    if (label === "tray") return "tray";
  } catch {
    /* vite browser preview */
  }
  return "main";
}

function WorkspaceRoot() {
  const [path, setPath] = useState(window.__ELIN_WORKSPACE ?? "");

  useEffect(() => {
    if (path) return;
    void api.workspaceContext().then((next) => {
      if (next) {
        window.__ELIN_WORKSPACE = next;
        setPath(next);
      }
    });
  }, [path]);

  if (!path) {
    return (
      <div className="flex h-full flex-col items-center justify-center gap-2 bg-ink-900 p-8 text-sm text-mist-300">
        <div>Opening workspace…</div>
        <p className="max-w-md text-center text-[12px] text-mist-300/80">
          If this stays here, close the window and open the project again from Elin.
        </p>
      </div>
    );
  }
  return <WorkspaceApp projectPath={path} />;
}

class ShellError extends Component<{ children: ReactNode }, { message: string | null }> {
  state = { message: null as string | null };

  static getDerivedStateFromError(error: Error) {
    return { message: error.message || String(error) };
  }

  componentDidCatch(error: Error, info: ErrorInfo) {
    console.error(error, info);
  }

  render() {
    if (this.state.message) {
      return (
        <div className="flex h-full flex-col items-center justify-center gap-3 bg-ink-900 p-8 text-sm text-mist-50">
          <div className="font-medium">This window failed to load</div>
          <pre className="max-w-xl whitespace-pre-wrap font-mono text-xs text-otp-400">{this.state.message}</pre>
        </div>
      );
    }
    return this.props.children;
  }
}

export default function App() {
  const [shell, setShell] = useState(detectShell);

  useEffect(() => {
    setShell(detectShell());
    const id = window.setTimeout(() => setShell(detectShell()), 80);
    return () => window.clearTimeout(id);
  }, []);

  if (shell === "toast") return <ToastShell />;
  if (shell === "tray") return <TrayShell />;
  if (shell === "workspace") {
    return (
      <ShellError>
        <AppProvider lite>
          <WorkspaceRoot />
        </AppProvider>
      </ShellError>
    );
  }
  return (
    <ShellError>
      <AppProvider>
        <Shell />
      </AppProvider>
    </ShellError>
  );
}
