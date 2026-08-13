#!/usr/bin/env node
/**
 * Bump Elin version, commit, tag, push. Maintainers only.
 *
 *   npm run release           # patch  0.1.0 → 0.1.1
 *   npm run release -- minor  #        0.1.0 → 0.2.0
 *   npm run release -- major
 *   npm run release -- 0.2.0  # explicit
 *
 * Working tree must be clean. Bumps package.json, tauri.conf.json, Cargo.toml.
 * After a successful push, tag v* runs .github/workflows/release.yml and opens
 * a draft GitHub Release with the NSIS .exe and WiX .msi. Then:
 * GitHub → Releases → the draft → notes → Publish release.
 * A push to main/master without a tag only builds the elin-windows artifact.
 */
import { execSync } from "node:child_process";
import { readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");

function sh(cmd, opts = {}) {
  return execSync(cmd, { cwd: root, encoding: "utf8", stdio: ["ignore", "pipe", "pipe"], ...opts }).trim();
}

function read(rel) {
  return readFileSync(join(root, rel), "utf8");
}

function write(rel, text) {
  writeFileSync(join(root, rel), text);
}

const dirty = sh("git status --porcelain");
if (dirty) {
  console.error("Uncommitted files. Commit or stash them first:\n");
  console.error(dirty);
  process.exit(1);
}

const pkg = JSON.parse(read("package.json"));
const current = String(pkg.version);
const bump = (process.argv[2] ?? "patch").toLowerCase();

function nextVersion(v, kind) {
  if (/^\d+\.\d+\.\d+$/.test(kind)) return kind;
  const [maj, min, pat] = v.split(".").map(Number);
  if (kind === "major") return `${maj + 1}.0.0`;
  if (kind === "minor") return `${maj}.${min + 1}.0`;
  if (kind === "patch") return `${maj}.${min}.${pat + 1}`;
  console.error(`Unknown bump "${kind}". Use patch | minor | major | x.y.z`);
  process.exit(1);
}

const version = nextVersion(current, bump);
const tag = `v${version}`;

pkg.version = version;
write("package.json", `${JSON.stringify(pkg, null, 2)}\n`);

const tauri = read("src-tauri/tauri.conf.json").replace(
  /("version"\s*:\s*")[^"]+(")/,
  `$1${version}$2`,
);
write("src-tauri/tauri.conf.json", tauri);

const cargo = read("src-tauri/Cargo.toml").replace(/^version = "[^"]+"/m, `version = "${version}"`);
write("src-tauri/Cargo.toml", cargo);

sh(`git add package.json src-tauri/tauri.conf.json src-tauri/Cargo.toml`);
sh(`git commit -m ${JSON.stringify(`Release ${tag}`)}`);
sh(`git tag ${tag}`);

try {
  sh("git push origin HEAD");
  sh(`git push origin ${tag}`);
} catch (err) {
  console.error("Version committed and tagged locally, but push failed.");
  console.error(err.stderr || err.message);
  console.error(`\nPush yourself:\n  git push origin HEAD && git push origin ${tag}`);
  process.exit(1);
}

console.log(`${current} → ${version}`);
console.log(`Tagged ${tag} and pushed. Actions will open a draft GitHub Release.`);
console.log("GitHub → Releases → the draft → Publish release.");
