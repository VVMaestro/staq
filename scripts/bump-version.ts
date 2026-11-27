import { spawnSync } from "node:child_process";
import { readFileSync, writeFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

export type BumpKind = "major" | "minor" | "patch";

export interface SemVer {
  major: number;
  minor: number;
  patch: number;
  prerelease?: string;
  build?: string;
}

export interface BumpResult {
  previousVersions: {
    packageJson: string;
    tauriConf: string;
    cargoToml: string;
  };
  nextVersion: string;
  dryRun: boolean;
  updatedFiles: string[];
}

export interface BumpOptions {
  rootDir: string;
  target: string;
  dryRun?: boolean;
  syncCargoLock?: (cargoTomlPath: string) => void;
}

const BUMP_KINDS = new Set<string>(["major", "minor", "patch"]);

const SEMVER_RE =
  /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-((?:0|[1-9]\d*|\d*[a-zA-Z-][0-9a-zA-Z-]*)(?:\.(?:0|[1-9]\d*|\d*[a-zA-Z-][0-9a-zA-Z-]*))*))?(?:\+([0-9a-zA-Z-]+(?:\.[0-9a-zA-Z-]+)*))?$/;

const TOP_LEVEL_JSON_VERSION_RE = /^([ \t]+"version"[ \t]*:[ \t]*")([^"]*)(")/m;

export function isBumpKind(value: string): value is BumpKind {
  return BUMP_KINDS.has(value);
}

export function parseSemver(raw: string): SemVer {
  const match = SEMVER_RE.exec(raw);
  if (!match) {
    throw new Error(`Invalid SemVer version: "${raw}"`);
  }

  return {
    major: Number(match[1]),
    minor: Number(match[2]),
    patch: Number(match[3]),
    prerelease: match[4],
    build: match[5],
  };
}

export function resolveNextVersion(current: string, target: string): string {
  const trimmed = target.trim();
  if (!trimmed) {
    throw new Error("Version argument must not be empty.");
  }

  if (isBumpKind(trimmed)) {
    const parsed = parseSemver(current);
    switch (trimmed) {
      case "major":
        if (parsed.prerelease && parsed.minor === 0 && parsed.patch === 0) {
          return `${parsed.major}.0.0`;
        }
        return `${parsed.major + 1}.0.0`;
      case "minor":
        if (parsed.prerelease && parsed.patch === 0) {
          return `${parsed.major}.${parsed.minor}.0`;
        }
        return `${parsed.major}.${parsed.minor + 1}.0`;
      case "patch":
        if (parsed.prerelease) {
          return `${parsed.major}.${parsed.minor}.${parsed.patch}`;
        }
        return `${parsed.major}.${parsed.minor}.${parsed.patch + 1}`;
    }
  }

  const normalized = trimmed.startsWith("v") ? trimmed.slice(1) : trimmed;
  parseSemver(normalized);
  return normalized;
}

export function readJsonVersion(content: string, fileLabel: string): string {
  let parsed: unknown;
  try {
    parsed = JSON.parse(content);
  } catch (error) {
    throw new Error(`Failed to parse ${fileLabel}: ${(error as Error).message}`);
  }

  if (
    typeof parsed !== "object" ||
    parsed === null ||
    typeof (parsed as Record<string, unknown>).version !== "string"
  ) {
    throw new Error(`Missing string "version" field in ${fileLabel}.`);
  }

  return (parsed as { version: string }).version;
}

export function updateJsonVersion(
  content: string,
  nextVersion: string,
  fileLabel: string,
): string {
  readJsonVersion(content, fileLabel);

  if (!TOP_LEVEL_JSON_VERSION_RE.test(content)) {
    throw new Error(`Could not locate top-level "version" field in ${fileLabel}.`);
  }

  const updated = content.replace(TOP_LEVEL_JSON_VERSION_RE, `$1${nextVersion}$3`);

  if (readJsonVersion(updated, fileLabel) !== nextVersion) {
    throw new Error(`Failed to update "version" field in ${fileLabel}.`);
  }

  return updated;
}

interface CargoPackageVersionMatch {
  version: string;
  lineIndex: number;
  lines: string[];
  eol: string;
}

function findCargoPackageVersion(content: string): CargoPackageVersionMatch {
  const eol = content.includes("\r\n") ? "\r\n" : "\n";
  const lines = content.split(/\r?\n/);

  let inPackageSection = false;
  for (let i = 0; i < lines.length; i += 1) {
    const line = lines[i];
    if (/^\s*\[package\]\s*(?:#.*)?$/.test(line)) {
      inPackageSection = true;
      continue;
    }
    if (inPackageSection && /^\s*\[/.test(line)) {
      break;
    }
    if (inPackageSection) {
      const match = /^(\s*version\s*=\s*")([^"]*)(".*)$/.exec(line);
      if (match) {
        return { version: match[2], lineIndex: i, lines, eol };
      }
    }
  }

  throw new Error('Could not locate [package] version in "src-tauri/Cargo.toml".');
}

export function readCargoTomlVersion(content: string): string {
  return findCargoPackageVersion(content).version;
}

export function updateCargoTomlVersion(content: string, nextVersion: string): string {
  const { lineIndex, lines, eol } = findCargoPackageVersion(content);
  lines[lineIndex] = lines[lineIndex].replace(
    /^(\s*version\s*=\s*")([^"]*)(".*)$/,
    `$1${nextVersion}$3`,
  );
  return lines.join(eol);
}

export function defaultSyncCargoLock(cargoTomlPath: string): void {
  const result = spawnSync(
    "cargo",
    ["update", "--workspace", "--offline", "--manifest-path", cargoTomlPath],
    { encoding: "utf8" },
  );

  if (result.error) {
    throw new Error(`Failed to run cargo: ${result.error.message}`);
  }
  if (result.status !== 0) {
    const details = (result.stderr || result.stdout || "").trim();
    throw new Error(
      `cargo update failed with exit code ${result.status}${details ? `:\n${details}` : ""}`,
    );
  }
}

export function bumpVersions(options: BumpOptions): BumpResult {
  const { rootDir, target, dryRun = false, syncCargoLock = defaultSyncCargoLock } = options;

  const packageJsonPath = resolve(rootDir, "package.json");
  const tauriConfPath = resolve(rootDir, "src-tauri/tauri.conf.json");
  const cargoTomlPath = resolve(rootDir, "src-tauri/Cargo.toml");

  const packageJsonContent = readFileSync(packageJsonPath, "utf8");
  const tauriConfContent = readFileSync(tauriConfPath, "utf8");
  const cargoTomlContent = readFileSync(cargoTomlPath, "utf8");

  const previousVersions = {
    packageJson: readJsonVersion(packageJsonContent, "package.json"),
    tauriConf: readJsonVersion(tauriConfContent, "src-tauri/tauri.conf.json"),
    cargoToml: readCargoTomlVersion(cargoTomlContent),
  };

  const allSame =
    previousVersions.packageJson === previousVersions.tauriConf &&
    previousVersions.packageJson === previousVersions.cargoToml;

  if (isBumpKind(target.trim()) && !allSame) {
    throw new Error(
      [
        `Versions are out of sync across files; cannot apply "${target.trim()}" bump:`,
        `  package.json:              ${previousVersions.packageJson}`,
        `  src-tauri/tauri.conf.json: ${previousVersions.tauriConf}`,
        `  src-tauri/Cargo.toml:      ${previousVersions.cargoToml}`,
        "Pass an explicit version (e.g. 0.2.0) to synchronize them.",
      ].join("\n"),
    );
  }

  const nextVersion = resolveNextVersion(previousVersions.packageJson, target);

  if (allSame && previousVersions.packageJson === nextVersion) {
    throw new Error(`All files are already at version ${nextVersion}.`);
  }

  const nextPackageJson = updateJsonVersion(packageJsonContent, nextVersion, "package.json");
  const nextTauriConf = updateJsonVersion(
    tauriConfContent,
    nextVersion,
    "src-tauri/tauri.conf.json",
  );
  const nextCargoToml = updateCargoTomlVersion(cargoTomlContent, nextVersion);

  const updatedFiles = [
    "package.json",
    "src-tauri/tauri.conf.json",
    "src-tauri/Cargo.toml",
    "src-tauri/Cargo.lock",
  ];

  if (!dryRun) {
    writeFileSync(packageJsonPath, nextPackageJson, "utf8");
    writeFileSync(tauriConfPath, nextTauriConf, "utf8");
    writeFileSync(cargoTomlPath, nextCargoToml, "utf8");
    syncCargoLock(cargoTomlPath);
  }

  return {
    previousVersions,
    nextVersion,
    dryRun,
    updatedFiles,
  };
}

const USAGE = `Usage: bun run version:bump <major|minor|patch|x.y.z> [--dry-run]

Examples:
  bun run version:bump patch
  bun run version:bump minor
  bun run version:bump major
  bun run version:bump 0.2.0
  bun run version:bump minor --dry-run`;

export function parseCliArgs(argv: string[]): { target: string; dryRun: boolean; help: boolean } {
  let dryRun = false;
  let help = false;
  const positional: string[] = [];

  for (const arg of argv) {
    if (arg === "--dry-run") {
      dryRun = true;
    } else if (arg === "--help" || arg === "-h") {
      help = true;
    } else if (arg.startsWith("-")) {
      throw new Error(`Unknown option: ${arg}`);
    } else {
      positional.push(arg);
    }
  }

  if (help) {
    return { target: "", dryRun, help: true };
  }

  if (positional.length !== 1) {
    throw new Error("Expected exactly one version argument (major, minor, patch, or x.y.z).");
  }

  return { target: positional[0], dryRun, help: false };
}

function main(): void {
  try {
    const { target, dryRun, help } = parseCliArgs(process.argv.slice(2));
    if (help) {
      console.log(USAGE);
      return;
    }

    const scriptDir = dirname(fileURLToPath(import.meta.url));
    const rootDir = resolve(scriptDir, "..");

    const result = bumpVersions({ rootDir, target, dryRun });
    const prev = result.previousVersions.packageJson;
    const prefix = result.dryRun ? "[dry-run] " : "";
    console.log(`${prefix}Version: ${prev} -> ${result.nextVersion}`);
    for (const file of result.updatedFiles) {
      console.log(`  ${result.dryRun ? "would update" : "updated"} ${file}`);
    }
  } catch (error) {
    console.error(`Error: ${(error as Error).message}\n\n${USAGE}`);
    process.exit(1);
  }
}

if (import.meta.main) {
  main();
}
