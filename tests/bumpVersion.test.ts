import { describe, expect, test } from "bun:test";
import { mkdtempSync, mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import {
  bumpVersions,
  parseCliArgs,
  parseSemver,
  resolveNextVersion,
  updateCargoTomlVersion,
  updateJsonVersion,
} from "../scripts/bump-version";

describe("parseSemver", () => {
  test("parses standard and prerelease versions", () => {
    expect(parseSemver("0.1.1")).toEqual({
      major: 0,
      minor: 1,
      patch: 1,
      prerelease: undefined,
      build: undefined,
    });
    expect(parseSemver("1.2.3-rc.1+build.4")).toEqual({
      major: 1,
      minor: 2,
      patch: 3,
      prerelease: "rc.1",
      build: "build.4",
    });
  });

  test("rejects invalid semver strings", () => {
    expect(() => parseSemver("1.2")).toThrow();
    expect(() => parseSemver("01.2.3")).toThrow();
    expect(() => parseSemver("latest")).toThrow();
  });
});

describe("resolveNextVersion", () => {
  test("increments major, minor, and patch", () => {
    expect(resolveNextVersion("0.1.1", "patch")).toBe("0.1.2");
    expect(resolveNextVersion("0.1.1", "minor")).toBe("0.2.0");
    expect(resolveNextVersion("0.1.1", "major")).toBe("1.0.0");
  });

  test("handles bumping prerelease versions", () => {
    expect(resolveNextVersion("0.2.0-rc.1", "patch")).toBe("0.2.0");
    expect(resolveNextVersion("0.2.0-rc.1", "minor")).toBe("0.2.0");
    expect(resolveNextVersion("0.2.1-rc.1", "minor")).toBe("0.3.0");
    expect(resolveNextVersion("1.0.0-alpha.1", "major")).toBe("1.0.0");
    expect(resolveNextVersion("1.2.0-alpha.1", "major")).toBe("2.0.0");
  });

  test("accepts explicit versions with or without leading v", () => {
    expect(resolveNextVersion("0.1.1", "0.2.0")).toBe("0.2.0");
    expect(resolveNextVersion("0.1.1", "v0.2.0")).toBe("0.2.0");
    expect(resolveNextVersion("0.1.1", "0.2.0-beta.1")).toBe("0.2.0-beta.1");
  });

  test("rejects invalid arguments", () => {
    expect(() => resolveNextVersion("0.1.1", "")).toThrow();
    expect(() => resolveNextVersion("0.1.1", "foo")).toThrow();
    expect(() => resolveNextVersion("0.1.1", "1.2")).toThrow();
  });
});

describe("updateJsonVersion", () => {
  test("updates top-level version while preserving formatting and line endings", () => {
    const input = '{\r\n  "name": "staq",\r\n  "version": "0.1.1"\r\n}';
    expect(updateJsonVersion(input, "0.2.0", "package.json")).toBe(
      '{\r\n  "name": "staq",\r\n  "version": "0.2.0"\r\n}',
    );
  });

  test("throws when version field is missing", () => {
    expect(() => updateJsonVersion('{"name":"staq"}', "0.2.0", "package.json")).toThrow();
  });
});

describe("updateCargoTomlVersion", () => {
  test("updates only [package] version and leaves dependency versions untouched", () => {
    const input = [
      "[package]",
      'name = "staq"',
      'version = "0.1.1"',
      'edition = "2021"',
      "",
      "[dependencies]",
      'serde = { version = "1", features = ["derive"] }',
      "",
    ].join("\n");

    const updated = updateCargoTomlVersion(input, "0.2.0");
    expect(updated).toContain('version = "0.2.0"');
    expect(updated).toContain('serde = { version = "1", features = ["derive"] }');
  });

  test("throws if [package] has no version", () => {
    const input = '[package]\nname = "staq"\n\n[dependencies]\nversion = "1"\n';
    expect(() => updateCargoTomlVersion(input, "0.2.0")).toThrow();
  });
});

describe("parseCliArgs", () => {
  test("parses target, dry-run, and help flags", () => {
    expect(parseCliArgs(["minor"])).toEqual({ target: "minor", dryRun: false, help: false });
    expect(parseCliArgs(["0.2.0", "--dry-run"])).toEqual({
      target: "0.2.0",
      dryRun: true,
      help: false,
    });
    expect(parseCliArgs(["--help"])).toEqual({ target: "", dryRun: false, help: true });
  });

  test("rejects missing, extra, or unknown arguments", () => {
    expect(() => parseCliArgs([])).toThrow();
    expect(() => parseCliArgs(["minor", "patch"])).toThrow();
    expect(() => parseCliArgs(["minor", "--unknown"])).toThrow();
  });
});

describe("bumpVersions", () => {
  function createFixture(versions = { pkg: "0.1.1", tauri: "0.1.1", cargo: "0.1.1" }): string {
    const dir = mkdtempSync(join(tmpdir(), "staq-bump-"));
    mkdirSync(join(dir, "src-tauri"), { recursive: true });
    writeFileSync(
      join(dir, "package.json"),
      JSON.stringify({ name: "staq", version: versions.pkg }, null, 2) + "\n",
    );
    writeFileSync(
      join(dir, "src-tauri/tauri.conf.json"),
      JSON.stringify({ productName: "staq", version: versions.tauri }, null, 2) + "\n",
    );
    writeFileSync(
      join(dir, "src-tauri/Cargo.toml"),
      `[package]\nname = "staq"\nversion = "${versions.cargo}"\n`,
    );
    return dir;
  }

  test("updates all manifest files and invokes cargo lock sync", () => {
    const dir = createFixture();
    try {
      let syncedPath = "";
      const result = bumpVersions({
        rootDir: dir,
        target: "minor",
        syncCargoLock: (path) => {
          syncedPath = path;
        },
      });

      expect(result.nextVersion).toBe("0.2.0");
      expect(syncedPath).toBe(join(dir, "src-tauri/Cargo.toml"));
      expect(JSON.parse(readFileSync(join(dir, "package.json"), "utf8")).version).toBe("0.2.0");
      expect(
        JSON.parse(readFileSync(join(dir, "src-tauri/tauri.conf.json"), "utf8")).version,
      ).toBe("0.2.0");
      expect(readFileSync(join(dir, "src-tauri/Cargo.toml"), "utf8")).toContain(
        'version = "0.2.0"',
      );
    } finally {
      rmSync(dir, { recursive: true, force: true });
    }
  });

  test("does not modify files or sync Cargo.lock on dry-run", () => {
    const dir = createFixture();
    try {
      let synced = false;
      const result = bumpVersions({
        rootDir: dir,
        target: "patch",
        dryRun: true,
        syncCargoLock: () => {
          synced = true;
        },
      });

      expect(result.nextVersion).toBe("0.1.2");
      expect(result.dryRun).toBe(true);
      expect(synced).toBe(false);
      expect(JSON.parse(readFileSync(join(dir, "package.json"), "utf8")).version).toBe("0.1.1");
    } finally {
      rmSync(dir, { recursive: true, force: true });
    }
  });

  test("rejects relative bump when versions are out of sync, but allows explicit version", () => {
    const dir = createFixture({ pkg: "0.1.1", tauri: "0.1.0", cargo: "0.1.1" });
    try {
      expect(() =>
        bumpVersions({ rootDir: dir, target: "patch", syncCargoLock: () => {} }),
      ).toThrow(/out of sync/);

      const result = bumpVersions({
        rootDir: dir,
        target: "0.2.0",
        syncCargoLock: () => {},
      });
      expect(result.nextVersion).toBe("0.2.0");
      expect(
        JSON.parse(readFileSync(join(dir, "src-tauri/tauri.conf.json"), "utf8")).version,
      ).toBe("0.2.0");
    } finally {
      rmSync(dir, { recursive: true, force: true });
    }
  });
});
