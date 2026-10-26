import fs from 'node:fs';

const version = process.argv[2];
const output = process.argv[3] ?? 'release-notes.md';
if (!version) {
  console.error('Usage: release-notes.mjs <X.Y.Z> [output]');
  process.exit(1);
}
const changelog = fs.readFileSync('CHANGELOG.md', 'utf8');
const lines = changelog.split(/\r?\n/);
const marker = `## [${version}]`;
const startIndex = lines.findIndex((line) => line.startsWith(marker));
if (startIndex === -1) {
  console.error(`No CHANGELOG.md section found for ${version}.`);
  process.exit(1);
}
let endIndex = lines.length;
for (let index = startIndex + 1; index < lines.length; index += 1) {
  if (/^## \[/.test(lines[index])) {
    endIndex = index;
    break;
  }
}
const body = lines.slice(startIndex + 1, endIndex).join('\n').trim();
if (!/known limitations/i.test(body)) {
  console.error('Release section must include a Known limitations heading.');
  process.exit(1);
}
const notes = `# CinderDock v${version}\n\n${body}\n\n## Artifacts\n\n- CinderDock-v${version}-windows-x64-portable.exe\n- CinderDock-v${version}-windows-x64-setup.exe\n- CinderDock-v${version}-source.zip\n- SHA256SUMS.txt\n\nVerify downloads with \`SHA256SUMS.txt\` before running them.\n`;
fs.writeFileSync(output, notes);
console.log(`Wrote ${output}`);
