import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useState,
  type ReactNode,
} from "react";
import { api, onNavigate, onOpenProject } from "./lib/api";
import { applyLocale, detectLocale, dictionaries, subscribeLocale, type Dictionary } from "./i18n";
import type {
  AppUpdate,
  HostInfo,
  InstalledPair,
  Locale,
  PageId,
  StartupProbe,
  Studio,
  VersionCatalog,
} from "./types";

interface NavState {
  page: PageId;
  setPage: (page: PageId) => void;
  locale: Locale;
  setLocale: (locale: Locale) => void;
  t: Dictionary;
  pendingProject: string | null;
  clearPendingProject: () => void;
}

interface DataState {
  catalog: VersionCatalog | null;
  catalogError: string | null;
  refreshCatalog: (prerelease?: boolean, force?: boolean) => Promise<void>;
  toolchains: InstalledPair[];
  refreshToolchains: () => Promise<void>;
  studios: Studio[];
  selectedStudioIds: string[];
  preferredStudioId: string | null;
  setPreferredStudio: (id: string) => void;
  toggleStudio: (id: string) => void;
  refreshStudios: () => Promise<void>;
  ensureStudios: () => Promise<void>;
  addStudio: (studio: Studio) => void;
  host: HostInfo | null;
  includePrerelease: boolean;
  setIncludePrerelease: (value: boolean) => void;
  probe: StartupProbe | null;
  refreshProbe: () => Promise<void>;
  appUpdate: AppUpdate | null;
  refreshAppUpdate: (force?: boolean) => Promise<void>;
  dismissUpdate: () => void;
  skipUpdate: () => void;
  offerUpdate: boolean;
}

type AppState = NavState & DataState;

const NavCtx = createContext<NavState | null>(null);
const DataCtx = createContext<DataState | null>(null);

export function AppProvider({ children, lite = false }: { children: ReactNode; lite?: boolean }) {
  const [page, setPage] = useState<PageId>("home");
  const [pendingProject, setPendingProject] = useState<string | null>(null);
  const [locale, setLocaleState] = useState<Locale>(detectLocale);
  const [catalog, setCatalog] = useState<VersionCatalog | null>(null);
  const [catalogError, setCatalogError] = useState<string | null>(null);
  const [toolchains, setToolchains] = useState<InstalledPair[]>([]);
  const [studios, setStudios] = useState<Studio[]>([]);
  const [selectedStudioIds, setSelectedStudioIds] = useState<string[]>([]);
  const [preferredStudioId, setPreferredStudioId] = useState<string | null>(() => localStorage.getItem("elin.preferredStudio"));
  const [host, setHost] = useState<HostInfo | null>(null);
  const [includePrerelease, setIncludePrerelease] = useState(false);
  const [probe, setProbe] = useState<StartupProbe | null>(null);
  const [appUpdate, setAppUpdate] = useState<AppUpdate | null>(null);
  const [updateDismissed, setUpdateDismissed] = useState(false);

  const t = dictionaries[locale];

  const setLocale = useCallback((next: Locale) => {
    applyLocale(next);
    setLocaleState(next);
  }, []);

  useEffect(() => {
    document.documentElement.lang = locale;
    return subscribeLocale(setLocaleState);
  }, []);

  const refreshCatalog = useCallback(async (prerelease = includePrerelease, force = false) => {
    setCatalogError(null);
    try {
      setCatalog(await api.catalog(prerelease, force));
    } catch (error) {
      setCatalogError(error instanceof Error ? error.message : String(error));
    }
  }, [includePrerelease]);

  const refreshToolchains = useCallback(async () => {
    try {
      setToolchains(await api.toolchains());
    } catch {
      setToolchains([]);
    }
  }, []);

  const refreshStudios = useCallback(async () => {
    try {
      const found = await api.studios();
      setStudios(found);
      setSelectedStudioIds((current) => {
        if (current.length) return current;
        return found.filter((s) => s.detected).map((s) => s.id);
      });
      setPreferredStudioId((current) => {
        if (current && found.some((s) => s.id === current && s.detected)) return current;
        const first = found.find((s) => s.detected && (s.cli || s.executable));
        if (first) {
          localStorage.setItem("elin.preferredStudio", first.id);
          return first.id;
        }
        return current;
      });
    } catch {
      setStudios([]);
    }
  }, []);

  const ensureStudios = useCallback(async () => {
    if (studios.length) return;
    await refreshStudios();
  }, [refreshStudios, studios.length]);

  const refreshProbe = useCallback(async () => {
    try {
      const next = await api.probe();
      setProbe(next);
      if (next.elixir?.needsPathFix) {
        void api.toast({
          id: "path-missing",
          title: "Elixir is installed",
          body: next.elixir.why,
          kind: "warn",
          page: "doctor",
        });
      }
    } catch {
      setProbe(null);
    }
  }, []);

  const refreshAppUpdate = useCallback(async (force = false) => {
    try {
      const next = await api.checkAppUpdate(force);
      setAppUpdate((prev) => {
        if (prev && prev.latest !== next.latest) {
          setUpdateDismissed(false);
        }
        return next;
      });
      if (!next.newer) return;
      const skipped = localStorage.getItem("elin.skipVersion");
      if (skipped === next.latest) return;
      const notified = localStorage.getItem("elin.notifiedVersion");
      if (notified === next.latest) return;
      localStorage.setItem("elin.notifiedVersion", next.latest);
      void api.toast({
        id: "app-update",
        title: `Elin ${next.latest}`,
        body: next.notes.split("\n").find((line) => line.trim()) || next.name,
        kind: "ok",
        page: "settings",
      });
    } catch {
      /* offline / no releases yet */
    }
  }, []);

  const dismissUpdate = useCallback(() => setUpdateDismissed(true), []);

  const skipUpdate = useCallback(() => {
    if (appUpdate?.latest) {
      localStorage.setItem("elin.skipVersion", appUpdate.latest);
    }
    setUpdateDismissed(true);
  }, [appUpdate]);

  const setPreferredStudio = useCallback((id: string) => {
    localStorage.setItem("elin.preferredStudio", id);
    setPreferredStudioId(id);
  }, []);

  const toggleStudio = useCallback((id: string) => {
    setSelectedStudioIds((current) =>
      current.includes(id) ? current.filter((x) => x !== id) : [...current, id],
    );
  }, []);

  const addStudio = useCallback((studio: Studio) => {
    setStudios((current) => {
      if (current.some((s) => s.id === studio.id || s.executable === studio.executable)) {
        return current;
      }
      return [studio, ...current];
    });
    setSelectedStudioIds((current) =>
      current.includes(studio.id) ? current : [...current, studio.id],
    );
  }, []);

  useEffect(() => {
    void refreshToolchains();
    if (lite) return;
    void api.host().then(setHost).catch(() => undefined);
    void refreshCatalog();
    void refreshProbe();
    const boot = window.setTimeout(() => void refreshAppUpdate(false), 2200);
    const hour = window.setInterval(() => void refreshAppUpdate(false), 60 * 60 * 1000);
    return () => {
      window.clearTimeout(boot);
      window.clearInterval(hour);
    };
  }, [lite, refreshAppUpdate, refreshCatalog, refreshProbe, refreshToolchains]);

  useEffect(() => {
    if (lite) return;
    let unNav: (() => void) | undefined;
    let unOpen: (() => void) | undefined;
    void onNavigate((next) => {
      if (next) setPage(next as PageId);
    }).then((fn) => {
      unNav = fn;
    });
    void onOpenProject((path) => {
      setPendingProject(path);
      setPage("projects");
    }).then((fn) => {
      unOpen = fn;
    });
    void api.takeOpenProject().then((path) => {
      if (path) {
        setPendingProject(path);
        setPage("projects");
      }
    }).catch(() => undefined);
    return () => {
      unNav?.();
      unOpen?.();
    };
  }, [lite]);

  const clearPendingProject = useCallback(() => setPendingProject(null), []);

  const nav = useMemo<NavState>(
    () => ({ page, setPage, locale, setLocale, t, pendingProject, clearPendingProject }),
    [clearPendingProject, locale, page, pendingProject, setLocale, t],
  );

  const offerUpdate = Boolean(
    appUpdate?.newer && !updateDismissed && localStorage.getItem("elin.skipVersion") !== appUpdate.latest,
  );

  const data = useMemo<DataState>(
    () => ({
      catalog,
      catalogError,
      refreshCatalog,
      toolchains,
      refreshToolchains,
      studios,
      selectedStudioIds,
      preferredStudioId,
      setPreferredStudio,
      toggleStudio,
      refreshStudios,
      ensureStudios,
      addStudio,
      host,
      includePrerelease,
      setIncludePrerelease,
      probe,
      refreshProbe,
      appUpdate,
      refreshAppUpdate,
      dismissUpdate,
      skipUpdate,
      offerUpdate,
    }),
    [
      addStudio,
      appUpdate,
      catalog,
      catalogError,
      dismissUpdate,
      ensureStudios,
      host,
      includePrerelease,
      offerUpdate,
      preferredStudioId,
      probe,
      refreshAppUpdate,
      refreshCatalog,
      refreshProbe,
      refreshStudios,
      refreshToolchains,
      selectedStudioIds,
      setPreferredStudio,
      skipUpdate,
      studios,
      toggleStudio,
      toolchains,
    ],
  );

  return (
    <NavCtx.Provider value={nav}>
      <DataCtx.Provider value={data}>{children}</DataCtx.Provider>
    </NavCtx.Provider>
  );
}

export function useNav() {
  const ctx = useContext(NavCtx);
  if (!ctx) throw new Error("useNav must be used inside AppProvider");
  return ctx;
}

export function useData() {
  const ctx = useContext(DataCtx);
  if (!ctx) throw new Error("useData must be used inside AppProvider");
  return ctx;
}

export function useApp(): AppState {
  return { ...useNav(), ...useData() };
}
