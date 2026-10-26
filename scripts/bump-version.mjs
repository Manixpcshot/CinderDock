import fs from 'node:fs';

const version = process.argv[2];
if (!/^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?$/.test(version ?? '')) {
  console.error('Usage: bump-version.mjs <X.Y.Z>');
  process.exit(1);
}

const root = process.cwd();
const packagePath = `${root}/package.json`;
const tauriPath = `${root}/src-tauri/tauri.conf.json`;
const cargoPath = `${root}/src-tauri/Cargo.toml`;
const lockPath = `${root}/src-tauri/Cargo.lock`;

const packageJson = JSON.parse(fs.readFileSync(packagePath, 'utf8'));
packageJson.version = version;
fs.writeFileSync(packagePath, `${JSON.stringify(packageJson, null, 2)}\n`);

const tauriConf = JSON.parse(fs.readFileSync(tauriPath, 'utf8'));
tauriConf.version = version;
fs.writeFileSync(tauriPath, `${JSON.stringify(tauriConf, null, 2)}\n`);

let cargo = fs.readFileSync(cargoPath, 'utf8');
cargo = cargo.replace(/^version\s*=\s*"[^"]+"/m, `version = "${version}"`);
fs.writeFileSync(cargoPath, cargo);

if (fs.existsSync(lockPath)) {
  let lock = fs.readFileSync(lockPath, 'utf8');
  const pattern = /(\[\[package\]\]\nname = "cinderdock"\nversion = ")[^"]+(")/;
  if (pattern.test(lock)) {
    lock = lock.replace(pattern, `$1${version}$2`);
    fs.writeFileSync(lockPath, lock);
  }
}

console.log(`Bumped CinderDock to ${version}. Update CHANGELOG.md before committing.`);
