import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { existsSync, mkdirSync, readFileSync, readdirSync, statSync, writeFileSync } from "node:fs";
import { dirname, join, relative, resolve } from "node:path";
import process from "node:process";

const root = resolve(import.meta.dirname, "..");
const outputDirectory = join(root, "qa", "public-beta");
const allowNetwork = process.argv.includes("--online");
const publicationRefs = process.argv
  .filter((argument) => argument.startsWith("--public-ref="))
  .map((argument) => argument.slice("--public-ref=".length))
  .filter(Boolean);

function run(command, args, options = {}) {
  return execFileSync(command, args, {
    cwd: root,
    encoding: "utf8",
    maxBuffer: 128 * 1024 * 1024,
    stdio: ["pipe", "pipe", options.showStderr ? "inherit" : "pipe"],
    ...options,
  }).trim();
}

function sha256(value) {
  return createHash("sha256").update(value).digest("hex");
}

function fileSha256(path) {
  return sha256(readFileSync(path));
}

function writeJson(name, value) {
  mkdirSync(outputDirectory, { recursive: true });
  writeFileSync(join(outputDirectory, name), `${JSON.stringify(value, null, 2)}\n`, "utf8");
}

function stripQuotes(value) {
  if ((value.startsWith("'") && value.endsWith("'")) || (value.startsWith('"') && value.endsWith('"'))) {
    return value.slice(1, -1);
  }
  return value;
}

function packageKey(name, reference) {
  let value = stripQuotes(reference.trim());
  if (value.startsWith("link:") || value.startsWith("workspace:")) return null;
  if (value.startsWith("npm:")) {
    value = value.slice(4);
    const aliasMatch = value.match(/^(.+)@([^@()]+)(?:\(.*\))?$/);
    return aliasMatch ? `${aliasMatch[1]}@${aliasMatch[2]}` : null;
  }
  const version = value.split("(", 1)[0];
  return `${name}@${version}`;
}

function splitPackageKey(key) {
  const match = key.match(/^(.+)@([^@]+)$/);
  if (!match) throw new Error(`Unbekannter pnpm-Paketschluessel: ${key}`);
  return { name: match[1], version: match[2] };
}

function parsePnpmLock(path) {
  const source = readFileSync(path, "utf8").replaceAll("\r\n", "\n");
  const lines = source.split("\n");
  const packagesStart = lines.indexOf("packages:");
  const snapshotsStart = lines.indexOf("snapshots:");
  if (packagesStart < 0 || snapshotsStart < 0) throw new Error("pnpm-lock.yaml besitzt nicht die erwarteten Bereiche.");

  const packages = new Map();
  let current = null;
  for (const line of lines.slice(packagesStart + 1, snapshotsStart)) {
    const keyMatch = line.match(/^  ('?)(\S.*)\1:$/);
    if (keyMatch) {
      current = keyMatch[2];
      const { name, version } = splitPackageKey(current);
      packages.set(current, { key: current, name, version, integrity: null, os: [], cpu: [] });
      continue;
    }
    if (!current) continue;
    const integrity = line.match(/^    resolution: \{integrity: ([^}]+)\}/);
    if (integrity) packages.get(current).integrity = integrity[1];
    const constraint = line.match(/^    (os|cpu): \[([^\]]*)\]/);
    if (constraint) packages.get(current)[constraint[1]] = constraint[2].split(",").map((item) => item.trim()).filter(Boolean);
  }

  const direct = new Map();
  let importer = null;
  let scope = null;
  let dependencyName = null;
  for (const line of lines.slice(lines.indexOf("importers:") + 1, packagesStart)) {
    const importerMatch = line.match(/^  ('?)(\S.*)\1:$/);
    if (importerMatch) {
      importer = importerMatch[2];
      scope = null;
      dependencyName = null;
      continue;
    }
    const scopeMatch = line.match(/^    (dependencies|devDependencies|optionalDependencies):$/);
    if (scopeMatch) {
      scope = scopeMatch[1];
      dependencyName = null;
      continue;
    }
    const dependencyMatch = line.match(/^      ('?)(\S.*)\1:$/);
    if (dependencyMatch && scope) {
      dependencyName = dependencyMatch[2];
      continue;
    }
    const versionMatch = line.match(/^        version: (.+)$/);
    if (versionMatch && importer && scope && dependencyName) {
      const key = packageKey(dependencyName, versionMatch[1]);
      if (key) {
        const scopes = direct.get(key) ?? new Set();
        scopes.add(scope === "devDependencies" ? "development" : "runtime");
        direct.set(key, scopes);
      }
      dependencyName = null;
    }
  }

  const resolveKnownKey = (candidate) => {
    if (!candidate) return null;
    if (packages.has(candidate)) return candidate;
    return [...packages.keys()].find((key) => candidate === key || candidate.startsWith(`${key}(`)) ?? null;
  };

  const graph = new Map([...packages.keys()].map((key) => [key, new Set()]));
  current = null;
  let relation = null;
  for (const line of lines.slice(snapshotsStart + 1)) {
    const snapshotMatch = line.match(/^  ('?)(\S.*)\1:$/);
    if (snapshotMatch) {
      current = resolveKnownKey(snapshotMatch[2]);
      relation = null;
      continue;
    }
    const relationMatch = line.match(/^    (dependencies|optionalDependencies):$/);
    if (relationMatch) {
      relation = relationMatch[1];
      continue;
    }
    if (line.match(/^    [^ ]/) && !relationMatch) relation = null;
    const dependencyMatch = line.match(/^      ('?)(\S.*?)\1: (.+)$/);
    if (current && relation && dependencyMatch) {
      const dependency = resolveKnownKey(packageKey(dependencyMatch[2], dependencyMatch[3]));
      if (dependency) graph.get(current).add(dependency);
    }
  }

  function reachable(startKeys) {
    const seen = new Set();
    const queue = [...startKeys].filter((key) => packages.has(key));
    while (queue.length) {
      const key = queue.pop();
      if (seen.has(key)) continue;
      seen.add(key);
      for (const dependency of graph.get(key) ?? []) queue.push(dependency);
    }
    return seen;
  }

  const runtime = reachable([...direct].filter(([, scopes]) => scopes.has("runtime")).map(([key]) => key));
  const development = reachable([...direct].filter(([, scopes]) => scopes.has("development")).map(([key]) => key));
  return { packages, direct, graph, runtime, development };
}

function normalizeRepository(repository) {
  const raw = typeof repository === "string" ? repository : repository?.url;
  return raw?.replace(/^git\+/, "").replace(/\.git$/, "") ?? null;
}

function normalizeAuthor(author) {
  if (typeof author === "string") return author;
  if (author?.name) return author.name;
  return null;
}

function noticeFiles(packageRoot) {
  if (!packageRoot || !existsSync(packageRoot)) return [];
  const candidates = [];
  for (const entry of readdirSync(packageRoot, { withFileTypes: true })) {
    if (entry.isFile() && /^(licen[sc]e|copying|notice|copyright)([._-].*)?$/i.test(entry.name)) {
      candidates.push(join(packageRoot, entry.name));
    }
    if (entry.isDirectory() && /^licenses?$/i.test(entry.name)) {
      for (const nested of readdirSync(join(packageRoot, entry.name), { withFileTypes: true })) {
        if (nested.isFile()) candidates.push(join(packageRoot, entry.name, nested.name));
      }
    }
  }
  return candidates.sort().map((path) => ({
    path: relative(packageRoot, path).replaceAll("\\", "/"),
    sha256: fileSha256(path),
  }));
}

function installedNpmMetadata() {
  const virtualStore = join(root, "node_modules", ".pnpm");
  const result = new Map();
  if (!existsSync(virtualStore)) return result;
  for (const virtualEntry of readdirSync(virtualStore, { withFileTypes: true })) {
    if (!virtualEntry.isDirectory() || virtualEntry.name === "node_modules") continue;
    const modules = join(virtualStore, virtualEntry.name, "node_modules");
    if (!existsSync(modules)) continue;
    for (const first of readdirSync(modules, { withFileTypes: true })) {
      if (!first.isDirectory()) continue;
      if (first.name.startsWith("@")) {
        const scopeRoot = join(modules, first.name);
        for (const second of readdirSync(scopeRoot, { withFileTypes: true })) {
          if (second.isDirectory()) readNpmPackage(join(scopeRoot, second.name), result);
        }
      } else {
        readNpmPackage(join(modules, first.name), result);
      }
    }
  }
  return result;
}

function readNpmPackage(packageRoot, result) {
  const manifestPath = join(packageRoot, "package.json");
  if (!existsSync(manifestPath)) return;
  const manifest = JSON.parse(readFileSync(manifestPath, "utf8"));
  if (!manifest.name || !manifest.version) return;
  result.set(`${manifest.name}@${manifest.version}`, {
    license: typeof manifest.license === "string" ? manifest.license : null,
    author: normalizeAuthor(manifest.author),
    homepage: manifest.homepage ?? null,
    repository: normalizeRepository(manifest.repository),
    noticeFiles: noticeFiles(packageRoot),
    metadataSource: "installed-package",
  });
}

function cachedNpmMetadata() {
  const inventoryPath = join(outputDirectory, "dependency-licenses.json");
  if (!existsSync(inventoryPath)) return new Map();
  const inventory = JSON.parse(readFileSync(inventoryPath, "utf8"));
  const expectedLockHash = inventory.lockfiles?.pnpm?.sha256;
  if (expectedLockHash !== fileSha256(join(root, "pnpm-lock.yaml"))) return new Map();

  return new Map(inventory.packages
    .filter((pkg) => pkg.ecosystem === "npm")
    .map((pkg) => [`${pkg.name}@${pkg.version}`, {
      license: pkg.licenseDeclared,
      author: pkg.author,
      homepage: pkg.homepage,
      repository: pkg.repository,
      noticeFiles: pkg.noticeFiles,
      metadataSource: pkg.metadataSource,
    }]));
}

async function registryMetadata(name, version) {
  const response = await fetch(`https://registry.npmjs.org/${name.replace("/", "%2F")}`);
  if (!response.ok) throw new Error(`npm registry lieferte ${response.status} fuer ${name}@${version}`);
  const document = await response.json();
  const manifest = document.versions?.[version];
  if (!manifest) throw new Error(`npm registry enthaelt ${name}@${version} nicht.`);
  return {
    license: typeof manifest.license === "string" ? manifest.license : null,
    author: normalizeAuthor(manifest.author),
    homepage: manifest.homepage ?? null,
    repository: normalizeRepository(manifest.repository),
    noticeFiles: [],
    metadataSource: "registry.npmjs.org",
  };
}

async function completeNpmMetadata(lock) {
  const result = cachedNpmMetadata();
  for (const [key, metadata] of installedNpmMetadata()) result.set(key, metadata);
  const missing = [...lock.packages.values()].filter((pkg) => !result.has(pkg.key));
  if (missing.length && !allowNetwork) {
    throw new Error(`${missing.length} plattformspezifische npm-Pakete fehlen lokal; erneut mit --online ausfuehren.`);
  }
  const queue = [...missing];
  const workers = Array.from({ length: Math.min(8, queue.length) }, async () => {
    while (queue.length) {
      const pkg = queue.shift();
      result.set(pkg.key, await registryMetadata(pkg.name, pkg.version));
    }
  });
  await Promise.all(workers);
  return result;
}

function normalizeLicense(expression) {
  if (!expression) return "NOASSERTION";
  return expression
    .replaceAll("MIT/Apache-2.0", "MIT OR Apache-2.0")
    .replaceAll("Apache-2.0/MIT", "Apache-2.0 OR MIT")
    .replaceAll("Apache-2.0 / MIT", "Apache-2.0 OR MIT")
    .replaceAll("Unlicense/MIT", "Unlicense OR MIT")
    .replaceAll("BSD-3-Clause/MIT", "BSD-3-Clause OR MIT");
}

function cargoLockChecksums(path) {
  const result = new Map();
  for (const block of readFileSync(path, "utf8").split("[[package]]").slice(1)) {
    const name = block.match(/\nname = "([^"]+)"/)?.[1];
    const version = block.match(/\nversion = "([^"]+)"/)?.[1];
    const checksum = block.match(/\nchecksum = "([^"]+)"/)?.[1] ?? null;
    if (name && version) result.set(`${name}@${version}`, checksum);
  }
  return result;
}

function packageUrl(ecosystem, name, version) {
  const encodedVersion = encodeURIComponent(version);
  if (ecosystem === "npm") {
    const encodedName = name.split("/").map(encodeURIComponent).join("/");
    return `pkg:npm/${encodedName}@${encodedVersion}`;
  }
  return `pkg:cargo/${encodeURIComponent(name)}@${encodedVersion}`;
}

function integrityHash(integrity) {
  const match = integrity?.match(/^sha512-(.+)$/);
  return match ? Buffer.from(match[1], "base64").toString("hex") : null;
}

function licenseGroups(items) {
  const groups = new Map();
  for (const item of items) groups.set(item.license, (groups.get(item.license) ?? 0) + 1);
  return Object.fromEntries([...groups].sort(([a], [b]) => a.localeCompare(b)));
}

function cargoScopes(metadata, rootId) {
  const nodes = new Map(metadata.resolve.nodes.map((node) => [node.id, node]));
  const result = new Map();
  const queue = [];
  const rootNode = nodes.get(rootId);
  for (const dependency of rootNode.deps) {
    for (const kind of dependency.dep_kinds) {
      queue.push([dependency.pkg, kind.kind === "build" ? "build" : kind.kind === "dev" ? "test" : "runtime"]);
    }
  }

  while (queue.length) {
    const [id, scope] = queue.pop();
    const scopes = result.get(id) ?? new Set();
    if (scopes.has(scope)) continue;
    scopes.add(scope);
    result.set(id, scopes);
    for (const dependency of nodes.get(id)?.deps ?? []) {
      for (const kind of dependency.dep_kinds) {
        if (kind.kind === "dev") continue;
        const nextScope = scope === "runtime" && kind.kind === "build" ? "build" : scope;
        queue.push([dependency.pkg, nextScope]);
      }
    }
  }
  return result;
}

function imageEvidence(path) {
  const data = readFileSync(path);
  const extension = path.split(".").at(-1).toLowerCase();
  const result = {
    path: relative(root, path).replaceAll("\\", "/"),
    bytes: data.length,
    sha256: sha256(data),
    width: null,
    height: null,
    metadataMarkers: [],
  };
  if (extension === "png" && data.subarray(0, 8).equals(Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]))) {
    let offset = 8;
    while (offset + 12 <= data.length) {
      const length = data.readUInt32BE(offset);
      const type = data.toString("ascii", offset + 4, offset + 8);
      if (type === "IHDR") {
        result.width = data.readUInt32BE(offset + 8);
        result.height = data.readUInt32BE(offset + 12);
      }
      if (["tEXt", "zTXt", "iTXt", "eXIf"].includes(type)) result.metadataMarkers.push(type);
      offset += 12 + length;
      if (type === "IEND") break;
    }
  } else if (["jpg", "jpeg"].includes(extension) && data[0] === 0xff && data[1] === 0xd8) {
    let offset = 2;
    while (offset + 4 <= data.length) {
      if (data[offset] !== 0xff) break;
      while (data[offset] === 0xff) offset += 1;
      const marker = data[offset++];
      if (marker === 0xda || marker === 0xd9) break;
      if ((marker >= 0xd0 && marker <= 0xd7) || marker === 0x01) continue;
      const length = data.readUInt16BE(offset);
      if (marker === 0xe1 || marker === 0xfe) result.metadataMarkers.push(`FF${marker.toString(16).toUpperCase()}`);
      if ([0xc0, 0xc1, 0xc2, 0xc3, 0xc5, 0xc6, 0xc7, 0xc9, 0xca, 0xcb, 0xcd, 0xce, 0xcf].includes(marker)) {
        result.height = data.readUInt16BE(offset + 3);
        result.width = data.readUInt16BE(offset + 5);
      }
      offset += length;
    }
  }
  return result;
}

function repositoryEvidence() {
  const refScope = publicationRefs.length ? publicationRefs : ["--all"];
  const scannedRefs = publicationRefs.length
    ? publicationRefs.map((ref) => `${ref}\t${run("git", ["rev-parse", "--verify", ref])}`)
    : run("git", ["for-each-ref", "--format=%(refname)%09%(objectname)"]).split("\n").filter(Boolean);
  const tracked = run("git", ["ls-files"]).split("\n").filter(Boolean);
  const history = run("git", ["log", "-p", "--no-ext-diff", "--no-textconv", ...refScope]);
  const patterns = {
    privateKey: /-----BEGIN (?:RSA |EC |OPENSSH |DSA |PGP )?PRIVATE KEY-----/g,
    awsAccessKey: /AKIA[0-9A-Z]{16}/g,
    githubToken: /(?:gh[pousr]_[A-Za-z0-9]{20,}|github_pat_[A-Za-z0-9_]{20,})/g,
    npmToken: /npm_[A-Za-z0-9]{20,}/g,
    slackToken: /xox[baprs]-[A-Za-z0-9-]{10,}/g,
    jwt: /eyJ[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,}/g,
    credentialUrl: /[a-z][a-z0-9+.-]*:\/\/[^\s/@:]+:[^\s/@]+@/gi,
    genericSecretAssignment: /(?:api[_-]?key|client[_-]?secret|access[_-]?token|auth[_-]?token|password|passwd)\s*[:=]\s*["'][^"']{8,}["']/gi,
    personalPath: /(?:[A-Z]:[\\/](?:Users|Dokumente und Einstellungen)[\\/][^\\/\s"']+|\/(?:Users|home)\/[^/\s"']+)/gi,
    emailLike: /[A-Z0-9._%+-]+@[A-Z0-9.-]+\.[A-Z]{2,63}/gi,
  };
  const historyPatternCounts = Object.fromEntries(Object.entries(patterns).map(([name, regex]) => [name, [...history.matchAll(regex)].length]));
  const identities = run("git", ["log", "--format=%an%x09%ae%x09%cn%x09%ce", ...refScope]).split("\n").filter(Boolean);
  const identityRecords = new Map();
  for (const line of identities) {
    const [authorName, authorEmail, committerName, committerEmail] = line.split("\t");
    for (const [role, name, email] of [["author", authorName, authorEmail], ["committer", committerName, committerEmail]]) {
      const key = `${role}:${name}:${email}`;
      const record = identityRecords.get(key) ?? { role, name, emailSha256: sha256(email.toLowerCase()), domain: email.split("@").at(-1), commits: 0 };
      record.commits += 1;
      identityRecords.set(key, record);
    }
  }

  const tagRefs = publicationRefs.length
    ? publicationRefs.filter((ref) => ref.startsWith("refs/tags/"))
    : run("git", ["for-each-ref", "--format=%(refname)", "refs/tags"]).split("\n").filter(Boolean);
  const taggers = tagRefs.map((ref) => {
    const [name, email] = run("git", ["for-each-ref", "--format=%(taggername)%09%(taggeremail)", ref]).split("\t");
    const normalizedEmail = email?.replace(/^<|>$/g, "") ?? "";
    return {
      ref,
      name: name || null,
      emailSha256: normalizedEmail ? sha256(normalizedEmail.toLowerCase()) : null,
      domain: normalizedEmail.includes("@") ? normalizedEmail.split("@").at(-1) : null,
    };
  });

  const objectLines = run("git", ["rev-list", "--objects", ...refScope]).split("\n").filter(Boolean);
  const objectPaths = new Map(objectLines.map((line) => {
    const [oid, ...pathParts] = line.split(" ");
    return [oid, pathParts.join(" ")];
  }));
  const objectInfo = run("git", ["cat-file", "--batch-check=%(objectname) %(objecttype) %(objectsize)"], { input: `${[...objectPaths.keys()].join("\n")}\n` })
    .split("\n").filter(Boolean).map((line) => {
      const [oid, type, size] = line.split(" ");
      return { oid, type, size: Number(size), path: objectPaths.get(oid) || null };
    });
  const blobs = objectInfo.filter((item) => item.type === "blob");
  const imagePaths = tracked.filter((path) => /\.(?:png|jpe?g)$/i.test(path)).map((path) => join(root, path));
  return {
    schemaVersion: 1,
    sourceRevision: run("git", ["rev-parse", "HEAD"]),
    refScope: publicationRefs.length ? "explicit-publication-refs" : "all-local-refs",
    refs: scannedRefs,
    commitCount: Number(run("git", ["rev-list", "--count", ...refScope])),
    trackedFiles: tracked.length,
    trackedBytes: tracked.reduce((total, path) => total + statSync(join(root, path)).size, 0),
    reachableObjects: objectInfo.length,
    uniqueBlobs: blobs.length,
    largestBlobBytes: Math.max(...blobs.map((item) => item.size)),
    blobsAtLeastFiveMiB: blobs.filter((item) => item.size >= 5 * 1024 * 1024).length,
    identities: [...identityRecords.values()].sort((a, b) => `${a.role}:${a.name}`.localeCompare(`${b.role}:${b.name}`)),
    taggers,
    historyPatternCounts,
    images: imagePaths.map(imageEvidence),
  };
}

const pnpmLockPath = join(root, "pnpm-lock.yaml");
const cargoLockPath = join(root, "src-tauri", "Cargo.lock");
const pnpmLock = parsePnpmLock(pnpmLockPath);
const npmMetadata = await completeNpmMetadata(pnpmLock);
const npmPackages = [...pnpmLock.packages.values()].map((pkg) => {
  const metadata = npmMetadata.get(pkg.key);
  const directScopes = [...(pnpmLock.direct.get(pkg.key) ?? [])].sort();
  const scopes = [];
  if (pnpmLock.runtime.has(pkg.key)) scopes.push("runtime");
  if (pnpmLock.development.has(pkg.key)) scopes.push("development");
  return {
    ecosystem: "npm",
    name: pkg.name,
    version: pkg.version,
    purl: packageUrl("npm", pkg.name, pkg.version),
    licenseDeclared: metadata.license,
    license: normalizeLicense(metadata.license),
    author: metadata.author,
    homepage: metadata.homepage,
    repository: metadata.repository,
    direct: directScopes.length > 0,
    directScopes,
    scopes,
    platform: { os: pkg.os, cpu: pkg.cpu },
    installedOnAuditHost: metadata.metadataSource === "installed-package",
    metadataSource: metadata.metadataSource,
    integrity: pkg.integrity,
    noticeFiles: metadata.noticeFiles,
  };
}).sort((a, b) => a.name.localeCompare(b.name) || a.version.localeCompare(b.version));

const cargoMetadataArgs = (filterPlatform = null) => [
  "metadata",
  ...(!allowNetwork ? ["--offline"] : []),
  "--manifest-path", "src-tauri/Cargo.toml",
  "--locked",
  ...(filterPlatform ? ["--filter-platform", filterPlatform] : []),
  "--format-version", "1",
];
const cargoMetadata = JSON.parse(run("cargo", cargoMetadataArgs()));
const windowsCargoMetadata = JSON.parse(run("cargo", cargoMetadataArgs("x86_64-pc-windows-msvc")));
const cargoChecksums = cargoLockChecksums(cargoLockPath);
const cargoRoot = cargoMetadata.packages.find((pkg) => pkg.name === "ldtg" && !pkg.source);
const cargoRootNode = cargoMetadata.resolve.nodes.find((node) => node.id === cargoRoot.id);
const cargoPackageScopes = cargoScopes(cargoMetadata, cargoRoot.id);
const windowsCargoPackageScopes = cargoScopes(windowsCargoMetadata, cargoRoot.id);
const cargoDirect = new Map(cargoRootNode.deps.map((dependency) => [dependency.pkg, dependency.dep_kinds.map((kind) => ({
  kind: kind.kind ?? "normal",
  target: kind.target ?? null,
}))]));
const windowsCargoIds = new Set(windowsCargoMetadata.resolve.nodes.map((node) => node.id));
const cargoPackages = cargoMetadata.packages.filter((pkg) => pkg.source).map((pkg) => {
  const packageRoot = dirname(pkg.manifest_path);
  return {
    ecosystem: "cargo",
    name: pkg.name,
    version: pkg.version,
    purl: packageUrl("cargo", pkg.name, pkg.version),
    licenseDeclared: pkg.license,
    license: normalizeLicense(pkg.license),
    authors: pkg.authors,
    homepage: pkg.homepage,
    repository: pkg.repository,
    direct: cargoDirect.has(pkg.id),
    directKinds: cargoDirect.get(pkg.id) ?? [],
    scopes: [...(cargoPackageScopes.get(pkg.id) ?? [])].sort(),
    windowsScopes: [...(windowsCargoPackageScopes.get(pkg.id) ?? [])].sort(),
    windowsResolved: windowsCargoIds.has(pkg.id),
    source: pkg.source,
    checksumSha256: cargoChecksums.get(`${pkg.name}@${pkg.version}`) ?? null,
    noticeFiles: noticeFiles(packageRoot),
  };
}).sort((a, b) => a.name.localeCompare(b.name) || a.version.localeCompare(b.version));

const allPackages = [...npmPackages, ...cargoPackages];
const unknownLicenses = allPackages.filter((pkg) => pkg.license === "NOASSERTION");
const inventory = {
  schemaVersion: 1,
  generator: "scripts/public-beta-audit.mjs",
  sourceRevision: run("git", ["rev-parse", "HEAD"]),
  lockfiles: {
    pnpm: { path: "pnpm-lock.yaml", sha256: fileSha256(pnpmLockPath), packages: npmPackages.length },
    cargo: { path: "src-tauri/Cargo.lock", sha256: fileSha256(cargoLockPath), packages: cargoPackages.length },
  },
  summary: {
    totalThirdPartyPackages: allPackages.length,
    npmPackages: npmPackages.length,
    npmRuntimePackages: npmPackages.filter((pkg) => pkg.scopes.includes("runtime")).length,
    npmDevelopmentPackages: npmPackages.filter((pkg) => pkg.scopes.includes("development")).length,
    cargoPackages: cargoPackages.length,
    cargoWindowsResolvedPackages: cargoPackages.filter((pkg) => pkg.windowsResolved).length,
    cargoWindowsRuntimePackages: cargoPackages.filter((pkg) => pkg.windowsScopes.includes("runtime")).length,
    cargoWindowsBuildPackages: cargoPackages.filter((pkg) => pkg.windowsScopes.includes("build")).length,
    cargoWindowsTestPackages: cargoPackages.filter((pkg) => pkg.windowsScopes.includes("test")).length,
    unknownLicenses: unknownLicenses.length,
    licenseGroups: licenseGroups(allPackages),
  },
  packages: allPackages,
};

const npmRefs = new Map(npmPackages.map((pkg) => [`${pkg.name}@${pkg.version}`, pkg.purl]));
const cargoRefs = new Map(cargoPackages.map((pkg) => [`${pkg.name}@${pkg.version}`, pkg.purl]));
const components = allPackages.map((pkg) => {
  const hashes = [];
  if (pkg.ecosystem === "npm") {
    const hash = integrityHash(pkg.integrity);
    if (hash) hashes.push({ alg: "SHA-512", content: hash });
  } else if (pkg.checksumSha256) {
    hashes.push({ alg: "SHA-256", content: pkg.checksumSha256 });
  }
  const properties = [{ name: "ldtg:ecosystem", value: pkg.ecosystem }, { name: "ldtg:direct", value: String(pkg.direct) }];
  if (pkg.ecosystem === "npm") {
    properties.push({ name: "ldtg:scopes", value: pkg.scopes.join(",") || "lockfile-only" });
    properties.push({ name: "ldtg:installed-on-audit-host", value: String(pkg.installedOnAuditHost) });
  } else {
    properties.push({ name: "ldtg:windows-resolved", value: String(pkg.windowsResolved) });
    properties.push({ name: "ldtg:scopes", value: pkg.scopes.join(",") || "lockfile-only" });
    properties.push({ name: "ldtg:windows-scopes", value: pkg.windowsScopes.join(",") || "not-resolved" });
  }
  return {
    type: "library",
    "bom-ref": pkg.purl,
    name: pkg.name,
    version: pkg.version,
    scope: (pkg.ecosystem === "npm" ? pkg.scopes.includes("runtime") : pkg.windowsScopes.includes("runtime")) ? "required" : "optional",
    hashes,
    licenses: [{ expression: pkg.license }],
    purl: pkg.purl,
    externalReferences: [pkg.repository && { type: "vcs", url: pkg.repository }, pkg.homepage && { type: "website", url: pkg.homepage }].filter(Boolean),
    properties,
  };
});

const dependencyMap = new Map();
for (const [key, dependencies] of pnpmLock.graph) {
  const reference = npmRefs.get(key);
  if (reference) dependencyMap.set(reference, new Set([...dependencies].map((item) => npmRefs.get(item)).filter(Boolean)));
}
const cargoIdRefs = new Map(cargoMetadata.packages.filter((pkg) => pkg.source).map((pkg) => [pkg.id, cargoRefs.get(`${pkg.name}@${pkg.version}`)]));
for (const node of cargoMetadata.resolve.nodes) {
  const reference = cargoIdRefs.get(node.id);
  if (reference) dependencyMap.set(reference, new Set(node.dependencies.map((id) => cargoIdRefs.get(id)).filter(Boolean)));
}
const applicationManifest = JSON.parse(readFileSync(join(root, "package.json"), "utf8"));
const applicationRef = `pkg:generic/ldtg@${encodeURIComponent(applicationManifest.version)}`;
const directDependencies = new Set([
  ...[...pnpmLock.direct.keys()].map((key) => npmRefs.get(key)).filter(Boolean),
  ...cargoRootNode.dependencies.map((id) => cargoIdRefs.get(id)).filter(Boolean),
]);
dependencyMap.set(applicationRef, directDependencies);
const sbom = {
  bomFormat: "CycloneDX",
  specVersion: "1.6",
  serialNumber: `urn:uuid:${run("git", ["rev-parse", "HEAD"]).slice(0, 8)}-0000-4000-8000-${fileSha256(pnpmLockPath).slice(0, 12)}`,
  version: 1,
  metadata: {
    tools: { components: [{ type: "application", name: "LDTG public beta audit generator", version: "1" }] },
    component: { type: "application", "bom-ref": applicationRef, name: "ldtg", version: applicationManifest.version },
    properties: [
      { name: "ldtg:source-revision", value: run("git", ["rev-parse", "HEAD"]) },
      { name: "ldtg:pnpm-lock-sha256", value: fileSha256(pnpmLockPath) },
      { name: "ldtg:cargo-lock-sha256", value: fileSha256(cargoLockPath) },
      { name: "ldtg:status", value: "draft-not-for-release" },
    ],
  },
  components,
  dependencies: [...dependencyMap].map(([ref, dependencies]) => ({ ref, dependsOn: [...dependencies].sort() })).sort((a, b) => a.ref.localeCompare(b.ref)),
};

writeJson("dependency-licenses.json", inventory);
writeJson("sbom.cdx.json", sbom);
writeJson("repository-evidence.json", repositoryEvidence());

if (unknownLicenses.length) {
  throw new Error(`${unknownLicenses.length} Abhaengigkeiten besitzen keine deklarierte Lizenz.`);
}

console.log(`Erzeugt: ${npmPackages.length} npm- und ${cargoPackages.length} Cargo-Pakete; ${components.length} SBOM-Komponenten.`);
