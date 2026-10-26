import fs from 'node:fs';
import { execFileSync } from 'node:child_process';

const root = process.cwd();
const packageJson = JSON.parse(fs.readFileSync(`${root}/package.json`, 'utf8'));
const tauriConf = JSON.parse(fs.readFileSync(`${root}/src-tauri/tauri.conf.json`, 'utf8'));
const cargoToml = fs.readFileSync(`${root}/src-tauri/Cargo.toml`, 'utf8');
const cargoVersion = /^version\s*=\s*"([^"]+)"/m.exec(cargoToml)?.[1];

const versions = {
  'package.json': packageJson.version,
  'tauri.conf.json': tauriConf.version,
  'Cargo.toml': cargoVersion,
};

function parseSemver(value) {
  const match = /^(\d+)\.(\d+)\.(\d+)(?:-([0-9A-Za-z.-]+))?(?:\+[0-9A-Za-z.-]+)?$/.exec(value ?? '');
  if (!match) return null;
  return {
    raw: value,
    major: Number(match[1]),
    minor: Number(match[2]),
    patch: Number(match[3]),
    prerelease: match[4] ?? '',
  };
}

function compare(a, b) {
  const left = parseSemver(a);
  const right = parseSemver(b);
  if (!left || !right) throw new Error(`Invalid semantic version: ${a} or ${b}`);
  for (const key of ['major', 'minor', 'patch']) {
    if (left[key] > right[key]) return 1;
    if (left[key] < right[key]) return -1;
  }
  if (left.prerelease === right.prerelease) return 0;
  if (!left.prerelease) return 1;
  if (!right.prerelease) return -1;
  return left.prerelease > right.prerelease ? 1 : -1;
}

function assertConsistent(expected) {
  const parsed = parseSemver(expected);
  if (!parsed) throw new Error(`Invalid semantic version: ${expected}`);
  for (const [file, version] of Object.entries(versions)) {
    if (version !== expected) {
      throw new Error(`Version mismatch in ${file}: expected ${expected}, found ${version ?? 'missing'}`);
    }
  }
  console.log(`Version ${expected} is consistent across package.json, tauri.conf.json and Cargo.toml.`);
}

function git(args) {
  return execFileSync('git', args, { encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'] }).trim();
}

function latestTag() {
  const tags = git(['tag', '--list', 'v*', '--sort=-v:refname'])
    .split('\n')
    .map((tag) => tag.trim())
    .filter(Boolean);
  return tags[0] ?? null;
}

function main() {
  const [mode, requested] = process.argv.slice(2);
  if (mode === 'consistent') {
    assertConsistent(packageJson.version);
    return;
  }
  if (mode === 'bump') {
    assertConsistent(packageJson.version);
    const base = process.env.CINDERDOCK_BASE_REF || 'HEAD^';
    let previous;
    try {
      previous = JSON.parse(git(['show', `${base}:package.json`])).version;
    } catch {
      console.log(`No base package.json at ${base}; skipping bump comparison.`);
      return;
    }
    if (compare(packageJson.version, previous) <= 0) {
      throw new Error(`Version must increase for every main update: ${previous} -> ${packageJson.version}`);
    }
    console.log(`Version bump verified: ${previous} -> ${packageJson.version}.`);
    return;
  }
  if (mode === 'release') {
    assertConsistent(requested);
    const tag = `v${requested}`;
    const latest = latestTag();
    if (latest) {
      const latestVersion = latest.replace(/^v/, '');
      if (compare(requested, latestVersion) <= 0) {
        throw new Error(`Release version must be newer than latest tag ${latest}.`);
      }
    }
    const existing = git(['tag', '--list', tag]);
    if (existing === tag) {
      const tagCommit = git(['rev-list', '-n', '1', tag]);
      const headCommit = git(['rev-parse', 'HEAD']);
      if (tagCommit !== headCommit) {
        throw new Error(`Tag ${tag} already exists on a different commit.`);
      }
      console.log(`Tag ${tag} already points to HEAD; release may be retried.`);
    } else {
      console.log(`Tag ${tag} does not exist and will be created only after successful checks/build.`);
    }
    console.log(`Release ${tag} passed version checks.`);
    return;
  }
  throw new Error('Usage: check-version.mjs <consistent|bump|release> [version]');
}

try {
  main();
} catch (error) {
  console.error(error.message);
  process.exit(1);
}
