import { existsSync, readdirSync, readFileSync } from "node:fs";
import { dirname, extname, join, relative, resolve } from "node:path";
import process from "node:process";

const root = resolve(import.meta.dirname, "..");
const excludedDirectories = new Set([
  ".git",
  "artifacts",
  "coverage",
  "dist",
  "node_modules",
  "target",
]);
const excludedFiles = new Set(["THIRD_PARTY_NOTICES.md"]);

function markdownFiles(directory) {
  const files = [];
  for (const entry of readdirSync(directory, { withFileTypes: true })) {
    if (entry.isDirectory() && excludedDirectories.has(entry.name)) continue;
    const path = join(directory, entry.name);
    if (entry.isDirectory()) files.push(...markdownFiles(path));
    else if (extname(entry.name).toLowerCase() === ".md" && !excludedFiles.has(entry.name)) files.push(path);
  }
  return files;
}

function localTarget(rawTarget) {
  let target = rawTarget.trim();
  if (target.startsWith("<") && target.endsWith(">")) target = target.slice(1, -1);
  target = target.split(/\s+["']/u, 1)[0];
  if (!target || target.startsWith("#") || /^[a-z][a-z\d+.-]*:/iu.test(target) || target.startsWith("//")) {
    return null;
  }
  target = target.split("#", 1)[0].split("?", 1)[0];
  if (!target) return null;
  try {
    return decodeURIComponent(target);
  } catch {
    return target;
  }
}

const failures = [];
for (const file of markdownFiles(root)) {
  const lines = readFileSync(file, "utf8").split(/\r?\n/u);
  let fence = null;
  for (const [index, line] of lines.entries()) {
    const fenceMatch = line.match(/^\s*(`{3,}|~{3,})/u);
    if (fenceMatch) {
      const marker = fenceMatch[1][0];
      fence = fence === marker ? null : (fence ?? marker);
      continue;
    }
    if (fence) continue;

    const targets = [
      ...line.matchAll(/!?\[[^\]]*\]\(([^)]+)\)/gu),
      ...line.matchAll(/<(?:img|a)\b[^>]+(?:src|href)=["']([^"']+)["'][^>]*>/giu),
    ].map((match) => match[1]);

    for (const rawTarget of targets) {
      const target = localTarget(rawTarget);
      if (!target) continue;
      const resolvedTarget = target.startsWith("/")
        ? resolve(root, `.${target}`)
        : resolve(dirname(file), target);
      if (!existsSync(resolvedTarget)) {
        failures.push(`${relative(root, file)}:${index + 1} -> ${target}`);
      }
    }
  }
}

if (failures.length > 0) {
  console.error("Defekte lokale Markdown-Links:");
  for (const failure of failures) console.error(`- ${failure}`);
  process.exitCode = 1;
} else {
  console.log("Lokale Markdown-Links sind gültig.");
}
