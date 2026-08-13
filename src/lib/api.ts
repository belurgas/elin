import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { openUrl } from "@tauri-apps/plugin-opener";
import type {
  CacheStatus,
  DoctorReport,
  GitSnapshot,
  HexPackage,
  HostInfo,
  InstallProgress,
  InstallResult,
  InstalledPair,
  Kit,
  KitStatus,
  MixProject,
  ModuleGraph,
  PluginStatus,
  ScanProgress,
  ScanReport,
  SparkResult,
  StartupProbe,
  Studio,
  ToastPayload,
  VersionCatalog,
} from "../types";

export const api = {
  host: () => invoke<HostInfo>("get_host_info"),
  catalog: (includePrerelease = false, force = false) =>
    invoke<VersionCatalog>("fetch_version_catalog", { includePrerelease, force }),
  install: (elixir: string, otp: string, addToPath: boolean, installHex: boolean) =>
    invoke<InstallResult>("install_toolchain", { elixir, otp, addToPath, installHex }),
  toolchains: () => invoke<InstalledPair[]>("list_toolchains"),
  activate: (elixir: string, otp: string) =>
    invoke<InstalledPair>("activate_toolchain", { elixir, otp }),
  remove: (elixir: string, otp: string) => invoke<void>("remove_toolchain", { elixir, otp }),
  studios: () => invoke<Studio[]>("scan_studios"),
  importStudio: (path: string) => invoke<Studio>("import_studio", { path }),
  plugins: (studios: Studio[]) => invoke<PluginStatus[]>("list_plugins", { studios }),
  installPlugin: (studio: Studio, marketplaceId: string) =>
    invoke<string>("install_studio_plugin", { studio, marketplaceId }),
  neovimSnippet: () => invoke<string>("get_neovim_snippet"),
  doctor: () => invoke<DoctorReport>("doctor_report"),
  doctorFix: (fixId: string) => invoke<string>("doctor_fix", { fixId }),
  spark: (name: string, directory: string, template: string, kits: string[] = []) =>
    invoke<SparkResult>("spark_create", { request: { name, directory, template, kits } }),
  eval: (code: string) => invoke<string>("playground_eval", { code }),
  hex: (query: string, force = false) => invoke<HexPackage[]>("hex_search", { query, force }),
  hexPackage: (name: string) => invoke<HexPackage>("hex_package", { name }),
  openPath: (path: string) => invoke<void>("open_path", { path }),
  probe: () => invoke<StartupProbe>("startup_probe"),
  cacheStatus: () => invoke<CacheStatus>("cache_status"),
  cacheClear: () => invoke<void>("cache_clear"),
  projects: () => invoke<MixProject[]>("list_projects"),
  scanQuick: () => invoke<MixProject[]>("scan_projects_quick"),
  scanDeep: (roots: string[] = []) => invoke<MixProject[]>("scan_projects_deep", { roots }),
  cancelScan: () => invoke<void>("cancel_project_scan"),
  inspectProject: (path: string) => invoke<MixProject>("inspect_project", { path }),
  installProjectToolchain: (path: string) =>
    invoke<MixProject>("install_project_toolchain", { path }),
  pinProjectToolchain: (path: string, elixir: string, otp: string) =>
    invoke<MixProject>("pin_project_toolchain", { path, elixir, otp }),
  graph: (path: string) => invoke<ModuleGraph>("project_graph", { path }),
  openInStudio: (studio: Studio, path: string, file?: string | null, line?: number | null) =>
    invoke<void>("open_project_in_studio", { studio, path, file: file ?? null, line: line ?? null }),
  addProject: (path: string) => invoke<MixProject>("add_project", { path }),
  starProject: (path: string) => invoke<MixProject>("star_project", { path }),
  projectGit: (path: string) => invoke<GitSnapshot>("project_git", { path }),
  projectCommit: (path: string, message: string, files: string[]) =>
    invoke<string>("project_commit", { path, message, files }),
  projectScan: (path: string, full = false, mixLayers = true) =>
    invoke<ScanReport>("project_scan", { path, full, mixLayers }),
  projectFormat: (path: string, check = false) =>
    invoke<string>("project_format", { path, check }),
  listKits: (path: string) => invoke<KitStatus[]>("list_kits", { path }),
  kitCatalog: () => invoke<Kit[]>("kit_catalog"),
  applyKits: (path: string, ids: string[]) =>
    invoke<string>("apply_project_kits", { path, ids }),
  removeKit: (path: string, id: string) => invoke<string>("remove_project_kit", { path, id }),
  writeKitConfig: (path: string, id: string) => invoke<string>("write_kit_config", { path, id }),
  setCredoStrict: (path: string, strict: boolean) => invoke<string>("set_credo_strict", { path, strict }),
  takeOpenProject: () => invoke<string | null>("take_open_project"),
  openWorkspace: (path: string) => invoke<void>("open_project_workspace", { path }),
  workspaceContext: () => invoke<string | null>("workspace_context"),
  projectMix: (path: string, task: string, session?: string) =>
    invoke<string>("project_mix", { path, task, session }),
  projectShell: (path: string, session: string, command: string) =>
    invoke<string>("project_shell", { path, session, command }),
  addHexDep: (path: string, name: string, requirement: string) =>
    invoke<string>("add_hex_dep", { path, name, requirement }),
  removeHexDep: (path: string, name: string) => invoke<string>("remove_hex_dep", { path, name }),
  watchStart: (path: string) => invoke<void>("workspace_watch_start", { path }),
  watchStop: (path: string) => invoke<void>("workspace_watch_stop", { path }),
  gitLicenses: () => invoke<Array<{ id: string; name: string }>>("git_licenses"),
  gitInit: (path: string, license: string) => invoke<GitSnapshot>("git_init", { path, license }),
  addComment: (path: string, file: string, tag: string, value: string) =>
    invoke<void>("add_elin_comment", { path, file, tag, value }),
  addElinToPath: () => invoke<string>("add_elin_to_path"),
  addToPath: (name: string) => invoke<string>("add_bin_to_path", { name }),
  toast: (toast: ToastPayload) => invoke<void>("show_toast", { toast }),
  hideToast: () => invoke<void>("hide_toast_window"),
  lastToast: () => invoke<ToastPayload | null>("last_toast"),
  openPage: (page: string) => invoke<void>("open_page", { page }),
  focusMain: () => invoke<void>("focus_main"),
  quit: () => invoke<void>("quit_app"),
};

export async function onInstallProgress(handler: (payload: InstallProgress) => void): Promise<UnlistenFn> {
  return listen<InstallProgress>("install-progress", (event) => handler(event.payload));
}

export async function onToast(handler: (payload: ToastPayload) => void): Promise<UnlistenFn> {
  return listen<ToastPayload>("elin-toast", (event) => handler(event.payload));
}

export async function onScanProgress(handler: (payload: ScanProgress) => void): Promise<UnlistenFn> {
  return listen<ScanProgress>("project-scan", (event) => handler(event.payload));
}

export async function onOpenProject(handler: (path: string) => void): Promise<UnlistenFn> {
  return listen<string>("elin-open-project", (event) => handler(event.payload));
}

export async function onNavigate(handler: (page: string) => void): Promise<UnlistenFn> {
  return listen<string>("elin-open", (event) => handler(event.payload));
}

export async function onMixLine(
  handler: (payload: { session: string; task: string; line: string }) => void,
): Promise<UnlistenFn> {
  return listen<{ session: string; task: string; line: string }>("mix-line", (event) => handler(event.payload));
}

export async function onWorkspaceFs(
  handler: (payload: { path: string; graph: boolean; git: boolean; lock: boolean }) => void,
): Promise<UnlistenFn> {
  return listen<{ path: string; graph: boolean; git: boolean; lock: boolean }>("workspace-fs", (event) =>
    handler(event.payload),
  );
}

export async function pickExecutable(): Promise<string | null> {
  const selected = await openDialog({
    multiple: false,
    directory: false,
    filters: [{ name: "Applications", extensions: ["exe"] }],
  });
  if (typeof selected === "string") return selected;
  return null;
}

export async function pickFolder(): Promise<string | null> {
  const selected = await openDialog({ directory: true, multiple: false });
  if (typeof selected === "string") return selected;
  return null;
}

export async function browse(url: string): Promise<void> {
  await openUrl(url);
}
