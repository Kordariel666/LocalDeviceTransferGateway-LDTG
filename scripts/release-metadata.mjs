import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { mkdirSync, readFileSync, readdirSync, writeFileSync } from "node:fs";
import { basename, join, resolve } from "node:path";
import process from "node:process";

const root = resolve(import.meta.dirname, "..");

function fail(message) {
  throw new Error(message);
}

function run(command, args) {
  const windowsCommandShim = process.platform === "win32" && command === "pnpm";
  const executable = windowsCommandShim ? (process.env.ComSpec ?? "cmd.exe") : command;
  const commandArguments = windowsCommandShim ? ["/d", "/s", "/c", "pnpm.cmd --version"] : args;
  return execFileSync(executable, commandArguments, {
    cwd: root,
    encoding: "utf8",
    maxBuffer: 16 * 1024 * 1024,
    stdio: ["ignore", "pipe", "pipe"],
  }).trim();
}

function readJson(path) {
  return JSON.parse(readFileSync(path, "utf8"));
}

function fileSha256(path) {
  return createHash("sha256").update(readFileSync(path)).digest("hex");
}

function property(properties, name) {
  return properties.find((entry) => entry.name === name)?.value;
}

function setProperty(properties, name, value) {
  const existing = properties.find((entry) => entry.name === name);
  if (existing) existing.value = value;
  else properties.push({ name, value });
}

function deterministicUuid(seed) {
  const bytes = Buffer.from(createHash("sha256").update(seed).digest("hex").slice(0, 32), "hex");
  bytes[6] = (bytes[6] & 0x0f) | 0x50;
  bytes[8] = (bytes[8] & 0x3f) | 0x80;
  const hex = bytes.toString("hex");
  return `${hex.slice(0, 8)}-${hex.slice(8, 12)}-${hex.slice(12, 16)}-${hex.slice(16, 20)}-${hex.slice(20)}`;
}

function argumentValue(prefix) {
  return process.argv.find((argument) => argument.startsWith(prefix))?.slice(prefix.length) ?? null;
}

const trackedManifests = run("git", [
  "ls-files",
  "--",
  "package.json",
  "apps/*/package.json",
  "packages/*/package.json",
]).split(/\r?\n/).filter(Boolean);

if (!trackedManifests.includes("package.json") || trackedManifests.length < 2) {
  fail("Keine vollstaendige, eingecheckte npm-Workspace-Manifestmenge gefunden.");
}

const applicationManifest = readJson(join(root, "package.json"));
const version = applicationManifest.version;
if (!version) fail("package.json enthaelt keine Version.");

const manifestVersions = trackedManifests.map((path) => ({
  path,
  name: readJson(join(root, path)).name,
  version: readJson(join(root, path)).version,
}));
for (const manifest of manifestVersions) {
  if (manifest.version !== version) {
    fail(`Versionskonflikt: ${manifest.path} enthaelt ${manifest.version ?? "keine Version"}, erwartet ${version}.`);
  }
}

const cargoToml = readFileSync(join(root, "src-tauri", "Cargo.toml"), "utf8");
const cargoVersion = cargoToml.match(/^version\s*=\s*"([^"]+)"/m)?.[1];
if (cargoVersion !== version) fail(`Versionskonflikt: src-tauri/Cargo.toml enthaelt ${cargoVersion}, erwartet ${version}.`);

const tauriVersion = readJson(join(root, "src-tauri", "tauri.conf.json")).version;
if (tauriVersion !== version) fail(`Versionskonflikt: src-tauri/tauri.conf.json enthaelt ${tauriVersion}, erwartet ${version}.`);

const changelog = readFileSync(join(root, "CHANGELOG.md"), "utf8");
const escapedVersion = version.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
if (!new RegExp(`^## \\[${escapedVersion}\\] - \\d{4}-\\d{2}-\\d{2}$`, "m").test(changelog)) {
  fail(`CHANGELOG.md enthaelt keinen datierten Abschnitt fuer ${version}.`);
}

for (const lockfile of ["pnpm-lock.yaml", "src-tauri/Cargo.lock"]) {
  run("git", ["ls-files", "--error-unmatch", "--", lockfile]);
}

const pnpmLockHash = fileSha256(join(root, "pnpm-lock.yaml"));
const cargoLockHash = fileSha256(join(root, "src-tauri", "Cargo.lock"));
const inventory = readJson(join(root, "qa", "public-beta", "dependency-licenses.json"));
const sbomTemplate = readJson(join(root, "qa", "public-beta", "sbom.cdx.json"));
const blockers = readJson(join(root, "qa", "public-beta", "blockers.json"));

if (inventory.lockfiles?.pnpm?.sha256 !== pnpmLockHash || inventory.lockfiles?.cargo?.sha256 !== cargoLockHash) {
  fail("Der Abhaengigkeitsaudit passt nicht zu den eingecheckten Lockfiles.");
}
if (inventory.summary?.unknownLicenses !== 0 || blockers.dependencyLicenseBlockers?.length !== 0) {
  fail("Der Abhaengigkeitsaudit ist unvollstaendig oder enthaelt Lizenzblocker.");
}
const sbomProperties = sbomTemplate.metadata?.properties ?? [];
if (property(sbomProperties, "ldtg:pnpm-lock-sha256") !== pnpmLockHash ||
    property(sbomProperties, "ldtg:cargo-lock-sha256") !== cargoLockHash) {
  fail("Die SBOM-Vorlage passt nicht zu den eingecheckten Lockfiles.");
}
if (sbomTemplate.metadata?.component?.version !== version ||
    sbomTemplate.components?.length !== inventory.summary?.totalThirdPartyPackages) {
  fail("Die SBOM-Vorlage passt nicht zu Version oder Umfang des Abhaengigkeitsaudits.");
}

const packageManagerMatch = applicationManifest.packageManager?.match(/^pnpm@(.+)$/);
if (!packageManagerMatch) fail("package.json muss pnpm mit exakter Version festlegen.");
const expectedPnpm = packageManagerMatch[1];
const expectedNode = readFileSync(join(root, ".node-version"), "utf8").trim();
const toolchainText = readFileSync(join(root, "rust-toolchain.toml"), "utf8");
const expectedRust = toolchainText.match(/^channel\s*=\s*"([^"]+)"/m)?.[1];
if (!/^\d+\.\d+\.\d+$/.test(expectedNode) || !/^\d+\.\d+\.\d+$/.test(expectedRust ?? "")) {
  fail("Node- oder Rust-Toolchain ist nicht auf eine exakte Version festgelegt.");
}

const actualNode = process.version.replace(/^v/, "");
const actualPnpm = run("pnpm", ["--version"]);
const actualRust = run("rustc", ["--version"]).match(/^rustc\s+(\S+)/)?.[1];
if (actualNode !== expectedNode) fail(`Node ${actualNode} aktiv, erwartet ${expectedNode}.`);
if (actualPnpm !== expectedPnpm) fail(`pnpm ${actualPnpm} aktiv, erwartet ${expectedPnpm}.`);
if (actualRust !== expectedRust) fail(`Rust ${actualRust} aktiv, erwartet ${expectedRust}.`);

const actionPins = [];
for (const filename of readdirSync(join(root, ".github", "workflows")).filter((name) => /\.ya?ml$/i.test(name))) {
  const workflow = readFileSync(join(root, ".github", "workflows", filename), "utf8");
  for (const match of workflow.matchAll(/^\s*-?\s*uses:\s*([^\s@]+)@([^\s#]+)/gm)) {
    const [, action, reference] = match;
    if (action.startsWith("./")) continue;
    if (!/^[0-9a-f]{40}$/i.test(reference)) {
      fail(`Nicht unveraenderlich fixierte Action in ${filename}: ${action}@${reference}.`);
    }
    actionPins.push({ workflow: `.github/workflows/${filename}`, action, revision: reference.toLowerCase() });
  }
}

const result = {
  version,
  manifests: manifestVersions,
  packageManager: `pnpm@${expectedPnpm}`,
  nodeVersion: expectedNode,
  rustToolchain: expectedRust,
  lockfiles: {
    "pnpm-lock.yaml": pnpmLockHash,
    "src-tauri/Cargo.lock": cargoLockHash,
  },
  dependencyAuditSourceRevision: inventory.sourceRevision,
  dependencyComponents: sbomTemplate.components?.length ?? 0,
  actionPins,
};

const outputArgument = argumentValue("--output=");
const revision = argumentValue("--revision=");
if (outputArgument) {
  if (!/^[0-9a-f]{40}$/i.test(revision ?? "")) fail("--revision muss ein vollstaendiger Commit-SHA sein.");
  const head = run("git", ["rev-parse", "HEAD"]).toLowerCase();
  if (revision.toLowerCase() !== head) fail(`Quellrevision ${revision} stimmt nicht mit HEAD ${head} ueberein.`);

  const sbom = structuredClone(sbomTemplate);
  const properties = sbom.metadata.properties ?? (sbom.metadata.properties = []);
  sbom.serialNumber = `urn:uuid:${deterministicUuid(`ldtg:${head}:${pnpmLockHash}:${cargoLockHash}`)}`;
  sbom.metadata.tools = {
    components: [{ type: "application", name: "LDTG private release metadata generator", version: "1" }],
  };
  sbom.metadata.component.version = version;
  sbom.metadata.component["bom-ref"] = `pkg:generic/ldtg@${encodeURIComponent(version)}`;
  setProperty(properties, "ldtg:source-revision", head);
  setProperty(properties, "ldtg:pnpm-lock-sha256", pnpmLockHash);
  setProperty(properties, "ldtg:cargo-lock-sha256", cargoLockHash);
  setProperty(properties, "ldtg:dependency-audit-source-revision", inventory.sourceRevision);
  setProperty(properties, "ldtg:status", "private-dry-run-not-published");

  const outputDirectory = resolve(root, outputArgument);
  mkdirSync(outputDirectory, { recursive: true });
  const outputPath = join(outputDirectory, "sbom.cdx.json");
  writeFileSync(outputPath, `${JSON.stringify(sbom, null, 2)}\n`, "utf8");
  result.sbom = { path: basename(outputPath), sha256: fileSha256(outputPath), serialNumber: sbom.serialNumber };
}

console.log(JSON.stringify(result, null, process.argv.includes("--json") ? 0 : 2));
