import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { existsSync, readFileSync, readdirSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import process from "node:process";

const root = resolve(import.meta.dirname, "..");
const inventoryPath = join(root, "qa", "public-beta", "dependency-licenses.json");
const outputPath = join(root, "THIRD_PARTY_NOTICES.md");
const checkOnly = process.argv.includes("--check");

function run(command, args) {
  return execFileSync(command, args, {
    cwd: root,
    encoding: "utf8",
    maxBuffer: 128 * 1024 * 1024,
    stdio: ["ignore", "pipe", "pipe"],
  }).trim();
}

function sha256(value) {
  return createHash("sha256").update(value).digest("hex");
}

function normalize(value) {
  return value
    .replaceAll("\r\n", "\n")
    .replaceAll("\r", "\n")
    .split("\n")
    .map((line) => line.replace(/[ \t]+$/u, ""))
    .join("\n");
}

function tableText(value) {
  return String(value ?? "—").replaceAll("|", "\\|").replaceAll(/\s+/g, " ").trim() || "—";
}

function selectedLicense(expression) {
  if (expression === "(MIT OR Apache-2.0) AND Unicode-3.0") return "MIT AND Unicode-3.0";
  if (!expression.includes(" OR ")) return expression;
  const alternatives = expression.replaceAll(/[()]/g, "").split(" OR ").map((item) => item.trim());
  const preference = [
    "MIT",
    "Apache-2.0",
    "BSD-3-Clause",
    "BSD-2-Clause",
    "ISC",
    "Zlib",
    "0BSD",
    "MIT-0",
    "CC0-1.0",
    "BSL-1.0",
    "Unlicense",
  ];
  const selected = preference.find((candidate) => alternatives.includes(candidate));
  if (!selected) throw new Error(`Keine deterministische Lizenzwahl fuer ${expression}.`);
  return selected;
}

function runtimePackages(inventory) {
  return inventory.packages.filter((pkg) => (
    (pkg.ecosystem === "npm" && pkg.scopes.includes("runtime"))
    || (pkg.ecosystem === "cargo" && pkg.windowsScopes.includes("runtime"))
  )).sort((a, b) => `${a.ecosystem}:${a.name}:${a.version}`.localeCompare(`${b.ecosystem}:${b.name}:${b.version}`));
}

function npmPackageRoots() {
  const store = join(root, "node_modules", ".pnpm");
  if (!existsSync(store)) throw new Error("node_modules/.pnpm fehlt; zuerst pnpm install --frozen-lockfile ausfuehren.");
  const result = new Map();
  for (const storeEntry of readdirSync(store, { withFileTypes: true })) {
    if (!storeEntry.isDirectory()) continue;
    const modules = join(store, storeEntry.name, "node_modules");
    if (!existsSync(modules)) continue;
    for (const entry of readdirSync(modules, { withFileTypes: true })) {
      if (!entry.isDirectory()) continue;
      if (entry.name.startsWith("@")) {
        const scope = join(modules, entry.name);
        for (const child of readdirSync(scope, { withFileTypes: true })) {
          if (child.isDirectory()) addNpmRoot(result, join(scope, child.name));
        }
      } else {
        addNpmRoot(result, join(modules, entry.name));
      }
    }
  }
  return result;
}

function addNpmRoot(result, packageRoot) {
  const manifestPath = join(packageRoot, "package.json");
  if (!existsSync(manifestPath)) return;
  const manifest = JSON.parse(readFileSync(manifestPath, "utf8"));
  if (manifest.name && manifest.version) result.set(`${manifest.name}@${manifest.version}`, packageRoot);
}

function cargoPackageRoots() {
  const metadata = JSON.parse(run("cargo", [
    "metadata",
    "--manifest-path", "src-tauri/Cargo.toml",
    "--format-version", "1",
    "--filter-platform", "x86_64-pc-windows-msvc",
    "--locked",
    "--offline",
  ]));
  return new Map(metadata.packages
    .filter((pkg) => pkg.source)
    .map((pkg) => [`${pkg.name}@${pkg.version}`, dirname(pkg.manifest_path)]));
}

function packageAuthors(pkg) {
  const values = pkg.ecosystem === "npm" ? [pkg.author] : pkg.authors;
  return values.filter(Boolean).join("; ") || "nicht in den Paketmetadaten angegeben";
}

function fallbackNotice(pkg, selected) {
  const authors = packageAuthors(pkg);
  if (selected === "MIT") {
    return `No standalone upstream license file was present in the package root.\n\nPackage authors: ${authors}\n\nPermission is hereby granted, free of charge, to any person obtaining a copy\nof this software and associated documentation files (the "Software"), to deal\nin the Software without restriction, including without limitation the rights\nto use, copy, modify, merge, publish, distribute, sublicense, and/or sell\ncopies of the Software, and to permit persons to whom the Software is\nfurnished to do so, subject to the following conditions:\n\nThe above copyright notice and this permission notice shall be included in all\ncopies or substantial portions of the Software.\n\nTHE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR\nIMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,\nFITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE\nAUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER\nLIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,\nOUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE\nSOFTWARE.`;
  }
  if (selected === "BSD-3-Clause") {
    return `No standalone upstream license file was present in the package root.\n\nPackage authors: ${authors}\n\nRedistribution and use in source and binary forms, with or without\nmodification, are permitted provided that the following conditions are met:\n\n1. Redistributions of source code must retain the above copyright notice,\n   this list of conditions and the following disclaimer.\n2. Redistributions in binary form must reproduce the above copyright notice,\n   this list of conditions and the following disclaimer in the documentation\n   and/or other materials provided with the distribution.\n3. Neither the name of the copyright holder nor the names of its contributors\n   may be used to endorse or promote products derived from this software\n   without specific prior written permission.\n\nTHIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS"\nAND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE\nIMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE\nDISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE\nFOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL\nDAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR\nSERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER\nCAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY,\nOR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE\nOF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.`;
  }
  if (selected === "Apache-2.0") {
    return `No standalone upstream license file was present in the package root.\n\nPackage authors: ${authors}\n\nThe complete Apache License 2.0 text is supplied in the release file LICENSE.`;
  }
  if (selected === "MPL-2.0") {
    return `No standalone upstream license file was present in the package root.\n\nPackage authors: ${authors}\n\nThe complete MPL-2.0 text is included below through the other MPL-2.0 runtime packages. The exact package source remains available from the source URL and checksum listed in the component table.`;
  }
  throw new Error(`Kein Fallback-Notice fuer ${pkg.name}@${pkg.version} unter ${selected}.`);
}

const inventory = JSON.parse(readFileSync(inventoryPath, "utf8"));
const packages = runtimePackages(inventory);
const npmRoots = npmPackageRoots();
const cargoRoots = cargoPackageRoots();
const notices = new Map();
const packageRecords = [];

for (const pkg of packages) {
  const key = `${pkg.name}@${pkg.version}`;
  const packageRoot = (pkg.ecosystem === "npm" ? npmRoots : cargoRoots).get(key);
  if (!packageRoot) throw new Error(`Installierte Quelle fehlt fuer ${pkg.ecosystem}:${key}.`);
  const selected = selectedLicense(pkg.license);
  const noticeIds = [];
  for (const noticeFile of pkg.noticeFiles ?? []) {
    const path = join(packageRoot, ...noticeFile.path.split("/"));
    if (!existsSync(path)) throw new Error(`Notice-Datei fehlt fuer ${key}: ${noticeFile.path}`);
    const data = readFileSync(path);
    const actualHash = sha256(data);
    if (actualHash !== noticeFile.sha256) {
      throw new Error(`Notice-Hash stimmt nicht fuer ${key}: ${noticeFile.path}`);
    }
    if (data.includes(0)) throw new Error(`Notice-Datei ist nicht textuell: ${key}/${noticeFile.path}`);
    const id = `sha256:${actualHash}`;
    const record = notices.get(id) ?? {
      id,
      source: "upstream file",
      text: normalize(data.toString("utf8")).trimEnd(),
      uses: [],
    };
    record.uses.push(`${pkg.ecosystem}:${key}/${noticeFile.path}`);
    notices.set(id, record);
    noticeIds.push(id);
  }
  if (!noticeIds.length) {
    const id = `generated:${pkg.ecosystem}:${key}:${selected}`;
    notices.set(id, {
      id,
      source: "SPDX fallback from package metadata",
      text: fallbackNotice(pkg, selected),
      uses: [`${pkg.ecosystem}:${key}`],
    });
    noticeIds.push(id);
  }
  const sourceUrl = pkg.repository
    ?? (pkg.ecosystem === "cargo" ? `https://crates.io/crates/${encodeURIComponent(pkg.name)}/${encodeURIComponent(pkg.version)}` : pkg.homepage)
    ?? "—";
  packageRecords.push({
    ecosystem: pkg.ecosystem,
    name: pkg.name,
    version: pkg.version,
    declared: pkg.license,
    selected,
    authors: packageAuthors(pkg),
    sourceUrl,
    checksum: pkg.checksumSha256 ?? pkg.integrity ?? "—",
    noticeIds,
  });
}

if (packages.some((pkg) => selectedLicense(pkg.license).includes("MPL-2.0"))
    && ![...notices.values()].some((notice) => notice.text.includes("Mozilla Public License Version 2.0"))) {
  throw new Error("MPL-2.0 wird verwendet, aber kein vollstaendiger MPL-2.0-Text wurde gefunden.");
}

const lines = [
  "# Third-party notices for LDTG",
  "",
  `Generated for LDTG ${JSON.parse(readFileSync(join(root, "package.json"), "utf8")).version}.`,
  "",
  "LDTG itself is licensed under Apache-2.0; see `LICENSE` and `NOTICE`. The",
  "components below retain their own licenses. This file covers the npm runtime",
  "packages embedded in the web interfaces and the Cargo packages reachable in",
  "the `x86_64-pc-windows-msvc` runtime graph. Build-only and test-only tools are",
  "recorded separately in `qa/public-beta/dependency-licenses.json`.",
  "",
  `- pnpm lock SHA-256: \`${inventory.lockfiles.pnpm.sha256}\``,
  `- Cargo lock SHA-256: \`${inventory.lockfiles.cargo.sha256}\``,
  `- Runtime components: **${packages.length}**`,
  `- Distinct notice texts: **${notices.size}**`,
  "",
  "For expressions containing `OR`, the selected alternative is shown below.",
  "Expressions containing `AND` retain every listed obligation. Repository URLs,",
  "package checksums and original notice-file hashes provide the version-specific",
  "source trail. No project copyright is asserted over third-party components.",
  "",
  "## Components",
  "",
  "| Ecosystem | Component | Declared license | Selected license | Authors | Source | Package checksum | Notice IDs |",
  "|---|---|---|---|---|---|---|---|",
];

for (const pkg of packageRecords) {
  lines.push(`| ${tableText(pkg.ecosystem)} | ${tableText(`${pkg.name} ${pkg.version}`)} | ${tableText(pkg.declared)} | ${tableText(pkg.selected)} | ${tableText(pkg.authors)} | ${tableText(pkg.sourceUrl)} | ${tableText(pkg.checksum)} | ${pkg.noticeIds.map((id) => `\`${id}\``).join("<br>")} |`);
}

lines.push("", "## License and notice texts", "");
for (const notice of [...notices.values()].sort((a, b) => a.id.localeCompare(b.id))) {
  lines.push(
    `### \`${notice.id}\``,
    "",
    `Source: ${notice.source}. Used by:`,
    "",
    ...notice.uses.sort().map((use) => `- \`${use}\``),
    "",
    "~~~~~~text",
    notice.text,
    "~~~~~~",
    "",
  );
}

const generated = `${lines.join("\n").trimEnd()}\n`;
if (checkOnly) {
  if (!existsSync(outputPath) || normalize(readFileSync(outputPath, "utf8")) !== generated) {
    throw new Error("THIRD_PARTY_NOTICES.md ist nicht aktuell; pnpm notices:generate ausfuehren.");
  }
  console.log(`THIRD_PARTY_NOTICES.md ist aktuell: ${packages.length} Komponenten, ${notices.size} Notice-Texte.`);
} else {
  writeFileSync(outputPath, generated, "utf8");
  console.log(`THIRD_PARTY_NOTICES.md erzeugt: ${packages.length} Komponenten, ${notices.size} Notice-Texte.`);
}
