import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { mkdtempSync, mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";

const releaseScript = readFileSync(new URL("./private-release.ps1", import.meta.url), "utf8");
const sha = "a".repeat(40);

// Execute the real PowerShell control flow; replace only external command boundaries.
// The cold fixture has neither node_modules nor a Cargo registry. No Git commits,
// network downloads, builds or packaging are performed by these regression tests.
function fixture({ expected = sha, dirty = false, failInstall = false } = {}) {
  const root = mkdtempSync(join(tmpdir(), "ldtg-release-"));
  mkdirSync(join(root, "scripts"));
  const calls = join(root, "calls.jsonl");
  const mocks = `
function Record-Call($Command, $CommandArguments) {
    Add-Content -LiteralPath $env:LDTG_RELEASE_TEST_CALLS -Value (ConvertTo-Json -Compress -InputObject (@($Command) + @($CommandArguments)))
}
function Invoke-Captured {
    param([string]$Command, [string[]]$CommandArguments)
    Record-Call $Command $CommandArguments
    if ($Command -eq 'git') {
        if ($CommandArguments[0] -eq 'rev-parse') { return ('a' * 40) }
        if ($CommandArguments[0] -eq 'status') { return $env:LDTG_RELEASE_TEST_DIRTY }
        throw 'Unexpected git call'
    }
    if ($Command -eq 'cargo') {
        return '{"packages":[{"name":"ldtg","targets":[{"name":"ldtg","kind":["bin"]},{"name":"ldtg-firewall-cleanup","kind":["bin"]},{"name":"generate-contracts","kind":["example"]}]}]}'
    }
    if ($Command -eq 'node') {
        if ($CommandArguments -notcontains '--source-only' -and
            (-not $script:installed -or -not $script:fetched)) { throw 'Notices ran before bootstrap' }
        return '{"version":"test"}'
    }
    throw 'Unexpected captured command'
}
function Invoke-Logged {
    param([string]$Label, [string]$Command, [string[]]$CommandArguments, [string]$LogPath)
    Record-Call $Command $CommandArguments
    if ($Command -eq 'pnpm' -and $CommandArguments[0] -eq 'install') {
        if ($env:LDTG_RELEASE_TEST_FAIL_INSTALL -eq '1') { throw 'Install failed' }
        $script:installed = $true
    } elseif ($Command -eq 'cargo' -and $CommandArguments[0] -eq 'fetch') {
        $script:fetched = $true
    } else { throw 'ValidateOnly must not run quality or packaging' }
}
$script:installed = $false
$script:fetched = $false
`;
  const path = join(root, "scripts", "private-release.ps1");
  writeFileSync(path, releaseScript.replace("Push-Location -LiteralPath $repoRoot", `${mocks}\nPush-Location -LiteralPath $repoRoot`));
  try {
    const result = spawnSync("powershell.exe", ["-NoProfile", "-ExecutionPolicy", "Bypass", "-File", path, "-ExpectedRevision", expected, "-ValidateOnly"], {
      encoding: "utf8",
      env: { ...process.env, LDTG_RELEASE_TEST_CALLS: calls, LDTG_RELEASE_TEST_DIRTY: dirty ? " M source.rs" : "", LDTG_RELEASE_TEST_FAIL_INSTALL: failInstall ? "1" : "0" },
    });
    assert.ifError(result.error);
    return { ...result, calls: readFileSync(calls, "utf8").trim().split(/\r?\n/).map((line) => JSON.parse(line)) };
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
}

const windows = { skip: process.platform !== "win32" };
test("cold release bootstraps both lockfiles before installed notice validation", windows, () => {
  const result = fixture();
  assert.equal(result.status, 0, result.stderr);
  assert.deepEqual(result.calls.map((call) => call.slice(0, 2)), [
    ["git", "rev-parse"], ["git", "status"], ["cargo", "metadata"],
    ["node", "scripts/release-metadata.mjs"], ["pnpm", "install"],
    ["cargo", "fetch"], ["node", "scripts/release-metadata.mjs"],
  ]);
  assert.ok(result.calls[3].includes("--source-only"));
  assert.ok(!result.calls[6].includes("--source-only"));
  assert.ok(result.calls[4].includes("--frozen-lockfile"));
  assert.ok(result.calls[5].includes("--locked"));
});
test("wrong source SHA fails before bootstrap", windows, () => {
  const result = fixture({ expected: "b".repeat(40) });
  assert.notEqual(result.status, 0);
  assert.deepEqual(result.calls, [["git", "rev-parse", "HEAD"]]);
});
test("dirty source tree fails before bootstrap", windows, () => {
  const result = fixture({ dirty: true });
  assert.notEqual(result.status, 0);
  assert.equal(result.calls.length, 2);
  assert.equal(result.calls[1][1], "status");
});
test("failed locked install stops before fetch and installed validation", windows, () => {
  const result = fixture({ failInstall: true });
  assert.notEqual(result.status, 0);
  assert.equal(result.calls.at(-1)[0], "pnpm");
  assert.equal(result.calls.at(-1)[1], "install");
});
