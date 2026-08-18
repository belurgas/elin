export function posix(path: string) {
  return path.replace(/\\/g, "/");
}

/** True when two project paths name the same file (slash, case, relative vs absolute). */
export function samePath(a: string, b?: string | null) {
  if (!b) return false;
  const na = posix(a).toLowerCase();
  const nb = posix(b).toLowerCase();
  if (na === nb) return true;
  return na.endsWith(`/${nb}`) || nb.endsWith(`/${na}`);
}

export function fileName(path: string) {
  const n = posix(path);
  return n.split("/").pop() || n;
}

export function appFolder(path: string): string | null {
  const hit = posix(path).match(/(?:^|\/)apps\/([^/]+)/);
  return hit ? hit[1] : null;
}

/** Shared `foo_` prefix across umbrella apps, otherwise empty. */
export function sharedAppPrefix(folders: string[]): string {
  const unique = [...new Set(folders.filter(Boolean))];
  if (unique.length < 2) return "";
  const prefixes = unique.map((n) => {
    const i = n.indexOf("_");
    return i > 0 ? n.slice(0, i + 1) : "";
  });
  const first = prefixes[0];
  if (!first) return "";
  return prefixes.every((p) => p === first) ? first : "";
}

export function shortAppLabels(folders: string[]): Map<string, string> {
  const prefix = sharedAppPrefix(folders);
  const map = new Map<string, string>();
  for (const folder of folders) {
    map.set(folder, prefix && folder.startsWith(prefix) ? folder.slice(prefix.length) || folder : folder);
  }
  return map;
}

/** Umbrella app folder. Strips a shared `foo_` prefix only when `peers` prove it is shared. */
export function appLabel(path: string, peers?: string[]) {
  const folder = appFolder(path);
  if (folder) {
    if (peers?.length) return shortAppLabels(peers).get(folder) ?? folder;
    return folder;
  }
  const n = posix(path);
  if (n.includes("/test/") || n.startsWith("test/")) return "test";
  return "lib";
}

/** `handle/assigned.ex` instead of `apps/supportly_flow/lib/supportly/flow/handle/assigned.ex`. */
export function shortPath(path: string) {
  const parts = posix(path).split("/").filter(Boolean);
  const file = parts[parts.length - 1] ?? path;
  const parent = parts[parts.length - 2];
  if (!parent || parent === "lib" || parent === "test" || parent === "apps") return file;
  return `${parent}/${file}`;
}

export function moduleTail(id: string, n = 2) {
  const parts = id.split(".");
  return parts.slice(Math.max(0, parts.length - n)).join(".");
}
