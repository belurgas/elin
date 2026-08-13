#!/usr/bin/env node
/**
 * Bump Elin version, commit, tag, push. Maintainers only.
 *
 *   npm run release           # patch  0.9.3 → 0.9.4
 *   npm run release -- minor  #        0.9.3 → 0.10.0
 *   npm run release -- major
 *   npm run release -- 1.0.0  # explicit
 *
 * Working tree must be clean. Writes the version in every place the app
 * and installers read it, then commits, tags vX.Y.Z, and pushes.
 * That tag runs .github/workflows/release.yml and opens a draft GitHub Release.
 */
import { execSync } from "node:child_process";
import { readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

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

/** Every file that stores the Elin version. Keep this list complete. */
export function applyVersion(version) {
  const pkg = JSON.parse(read("package.json"));
  pkg.version = version;
  write("package.json", `${JSON.stringify(pkg, null, 2)}\n`);

  write(
    "package-lock.json",
    read("package-lock.json").replace(
      /("name": "elin",\s*"version": ")[^"]+"/g,
      `$1${version}"`,
    ),
  );

  write(
    "src-tauri/tauri.conf.json",
    read("src-tauri/tauri.conf.json").replace(/("version"\s*:\s*")[^"]+(")/, `$1${version}$2`),
  );

  write(
    "src-tauri/Cargo.toml",
    read("src-tauri/Cargo.toml").replace(/^version = "[^"]+"/m, `version = "${version}"`),
  );

  write(
    "src-tauri/Cargo.lock",
    read("src-tauri/Cargo.lock").replace(
      /(name = "elin"\r?\nversion = ")[^"]+(")/,
      `$1${version}$2`,
    ),
  );

  write(
    ".github/ISSUE_TEMPLATE/bug.yml",
    read(".github/ISSUE_TEMPLATE/bug.yml").replace(
      /placeholder: .* \/ commit hash \/ tauri dev/,
      `placeholder: ${version} / commit hash / tauri dev`,
    ),
  );

  return [
    "package.json",
    "package-lock.json",
    "src-tauri/tauri.conf.json",
    "src-tauri/Cargo.toml",
    "src-tauri/Cargo.lock",
    ".github/ISSUE_TEMPLATE/bug.yml",
  ];
}

function isMain() {
  try {
    return import.meta.url === pathToFileURL(process.argv[1]).href;
  } catch {
    return false;
  }
}

if (isMain()) {
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
  const files = applyVersion(version);

  sh(`git add ${files.join(" ")}`);
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
}
