import assert from "node:assert/strict";
import test from "node:test";
import { npmRegistryUrl } from "./npm-registry-url.mjs";

test("registry URLs preserve ordinary and scoped package names", () => {
  assert.equal(npmRegistryUrl("react"), "https://registry.npmjs.org/react");
  assert.equal(npmRegistryUrl("@tauri-apps/api"), "https://registry.npmjs.org/%40tauri-apps%2Fapi");
});

for (const name of [
  "@scope/pkg/extra/slashes", "pkg?next=value&another=value", "pkg#fragment",
  "pkg\\extra\\path", "%2e%2e%2fadmin", "pkg%2Fextra", "//example.invalid/path",
  "../admin", "name with spaces", "paket-ä",
]) {
  test(`registry URL contains the complete name as one component: ${JSON.stringify(name)}`, () => {
    const url = new URL(npmRegistryUrl(name));
    assert.equal(url.origin, "https://registry.npmjs.org");
    assert.equal(url.search, "");
    assert.equal(url.hash, "");
    assert.equal(url.pathname.split("/").length, 2);
    assert.equal(decodeURIComponent(url.pathname.slice(1)), name);
  });
}

test("dot segments and malformed Unicode fail before fetch", () => {
  for (const name of [".", ".."]) assert.throws(() => npmRegistryUrl(name), /Punktsegment/);
  assert.throws(() => npmRegistryUrl("\uD800"), URIError);
});
