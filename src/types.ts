export type PageId =
  | "home"
  | "install"
  | "toolchain"
  | "studios"
  | "plugins"
  | "doctor"
  | "projects"
  | "playground"
  | "hex"
  | "learn"
  | "settings";

export type { Locale } from "./i18n";

export interface ElixirRelease {
  version: string;
  otpMajors: number[];
  isLatest: boolean;
  isPrerelease: boolean;
}

export interface OtpRelease {
  version: string;
  major: number;
  zipUrl?: string | null;
  exeUrl?: string | null;
  isLatest: boolean;
  isPrerelease: boolean;
  publishedAt?: string | null;
}

export interface VersionCatalog {
  elixir: ElixirRelease[];
  otp: OtpRelease[];
  latestElixir?: string | null;
  latestOtp?: string | null;
  recommendedElixir?: string | null;
  recommendedOtp?: string | null;
  fetchedAt: string;
  source: string;
}

export interface InstalledPair {
  elixir: string;
  otp: string;
  elixirPath: string;
  otpPath: string;
  isActive: boolean;
}

export interface InstallProgress {
  stage: string;
  message: string;
  percent: number;
}

export interface InstallResult {
  pair: InstalledPair;
  elixirVersionOutput: string;
}

export type StudioFamily =
  | "vscode"
  | "jetbrains"
  | "neovim"
  | "zed"
  | "emacs"
  | "sublime"
  | "other";

export interface Studio {
  id: string;
  name: string;
  family: StudioFamily;
  executable?: string | null;
  cli?: string | null;
  detected: boolean;
  pluginCapable: boolean;
  iconDataUrl?: string | null;
  notes: string;
}

export interface Plugin {
  id: string;
  name: string;
  publisher: string;
  family: StudioFamily;
  marketplaceId?: string | null;
  url: string;
  summary: string;
  why: string;
  recommended: boolean;
  beginner: boolean;
}

export interface PluginStatus {
  plugin: Plugin;
  installedIn: string[];
}

export interface DoctorCheck {
  id: string;
  group: string;
  title: string;
  ok: boolean;
  detail: string;
  hint: string;
  severity: string;
  path?: string | null;
  fixId?: string | null;
}

export interface BinaryHit {
  name: string;
  path: string;
  version?: string | null;
  onProcessPath: boolean;
  onUserPath: boolean;
  onMachinePath: boolean;
  callable: boolean;
  needsPathFix: boolean;
  source: string;
  why: string;
}

export interface StartupProbe {
  elixir?: BinaryHit | null;
  erlang?: BinaryHit | null;
  mix?: BinaryHit | null;
  git?: BinaryHit | null;
  managedCount: number;
  userPathHasElixir: boolean;
  processPathHasElixir: boolean;
  notes: string[];
}

export interface DoctorReport {
  score: number;
  checks: DoctorCheck[];
  elixirVersion?: string | null;
  erlangVersion?: string | null;
  probe: StartupProbe;
}

export interface HexPackage {
  name: string;
  description?: string | null;
  downloads?: number | null;
  downloadsRecent?: number | null;
  latest?: string | null;
  htmlUrl: string;
  docsUrl: string;
  licenses?: string[];
  links?: Record<string, string>;
}

export interface HostInfo {
  os: string;
  arch: string;
  home?: string | null;
  installsDir: string;
}

export interface SparkResult {
  path: string;
  output: string;
}

export interface MixDep {
  name: string;
  spec: string;
}

export interface MixProject {
  name: string;
  path: string;
  mixExs: string;
  deps: MixDep[];
  locked: MixDep[];
  hasPhoenix: boolean;
  hasLiveview: boolean;
  hasApplication?: boolean;
  elixirReq?: string | null;
  pinnedElixir?: string | null;
  pinnedOtp?: string | null;
  resolvedElixir?: string | null;
  resolvedOtp?: string | null;
  starred?: boolean;
  lastOpened?: number | null;
}

export interface GraphNode {
  id: string;
  label: string;
  path?: string | null;
  kind?: string;
  git?: string | null;
  boundary?: string | null;
  notes?: string[];
  ignored?: boolean;
  wired?: boolean;
  role?: string;
  loc?: number;
  defs?: number;
  defps?: number;
  behaviours?: string[];
  fanIn?: number;
  fanOut?: number;
}

export interface GraphEdge {
  from: string;
  to: string;
  kind: string;
  isNew?: boolean;
}

export interface ElinComment {
  file: string;
  line: number;
  tag: string;
  value: string;
  module?: string | null;
}

export interface ProjectEntry {
  rel: string;
  isDir: boolean;
  module?: string | null;
}

export interface GraphStats {
  modules: number;
  tests: number;
  edges: number;
  unwired: number;
  cycles: number;
}

export interface ModuleGraph {
  nodes: GraphNode[];
  edges: GraphEdge[];
  comments?: ElinComment[];
  files?: ProjectEntry[];
  stats?: GraphStats;
  cycles?: string[][];
}

export interface GitFile {
  path: string;
  status: string;
  added: number;
  deleted: number;
}

export interface DepChange {
  name: string;
  kind: string;
  from?: string | null;
  to?: string | null;
}

export interface GitSnapshot {
  repo?: string | null;
  branch?: string | null;
  identityOk: boolean;
  identityHint?: string | null;
  files: GitFile[];
  depChanges: DepChange[];
}

export interface Kit {
  id: string;
  name: string;
  summary: string;
  hex?: string | null;
  requirement: string;
  mixTuple?: string | null;
  defaultOn: boolean;
  phoenixOnly: boolean;
  advanced: boolean;
  configFile?: string | null;
  mixTask?: string | null;
}

export interface KitStatus {
  kit: Kit;
  installed: boolean;
  configPresent: boolean;
  credoStrict?: boolean | null;
}

export interface ScanFinding {
  layer: string;
  severity: string;
  file?: string | null;
  line?: number | null;
  message: string;
  tool: string;
}

export interface ScanLayer {
  id: string;
  name: string;
  ran: boolean;
  ok: boolean;
  detail: string;
}

export interface ScanReport {
  path: string;
  full: boolean;
  layers: ScanLayer[];
  findings: ScanFinding[];
  graph: ModuleGraph;
  git: GitSnapshot;
  kits: KitStatus[];
}

export interface CacheStatus {
  catalogAgeSecs?: number | null;
  catalogFresh: boolean;
  hexAgeSecs?: number | null;
  hexFresh: boolean;
  dir: string;
}

export interface ToastPayload {
  id: string;
  title: string;
  body: string;
  kind: string;
  page?: string | null;
}

export interface ScanProgress {
  visited: number;
  found: number;
  current: string;
  done: boolean;
}
