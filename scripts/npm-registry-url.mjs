// npm package metadata uses one URL path component, including for scoped names.
export function npmRegistryUrl(name) {
  if (name === "." || name === "..") {
    throw new Error("Ein npm-Paketname darf kein URL-Punktsegment sein.");
  }
  return `https://registry.npmjs.org/${encodeURIComponent(name)}`;
}
