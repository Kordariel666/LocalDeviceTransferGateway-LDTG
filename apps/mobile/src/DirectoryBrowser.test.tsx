import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { DirectoryBrowser } from "./DirectoryBrowser";

afterEach(cleanup);

describe("Ordnerdownloads", () => {
  it("bietet ZIP für den aktuellen und jeden gelisteten Ordner ohne die Dateilinks zu ändern", () => {
    const onLoad = vi.fn(async () => {});
    render(<DirectoryBrowser path="Fotos & mehr" search="Filter" error="" onLoad={onLoad} onSearchChange={vi.fn()} directory={{
      path: "Fotos & mehr", query: "Filter", nextCursor: "next", nextPage: 1,
      entries: [
        { name: "Grüße 🐈", path: "Fotos & mehr/Grüße 🐈", kind: "directory", size: 0, modifiedAt: null },
        { name: "Bild.jpg", path: "Fotos & mehr/Bild.jpg", kind: "file", size: 12, modifiedAt: null },
      ],
    }} />);
    const current = screen.getByRole("link", { name: "Ordner als ZIP herunterladen" });
    expect(current.getAttribute("href")).toBe(`/api/v1/download-folder?path=${encodeURIComponent("Fotos & mehr")}`);
    const child = screen.getByRole("link", { name: "Grüße 🐈: Ordner als ZIP herunterladen" });
    expect(child.getAttribute("href")).toBe(`/api/v1/download-folder?path=${encodeURIComponent("Fotos & mehr/Grüße 🐈")}`);
    expect(child.hasAttribute("download")).toBe(true);
    expect(child.closest("button")).toBeNull();
    fireEvent.click(screen.getByRole("button", { name: /Grüße 🐈.*Öffnen/ }));
    expect(onLoad).toHaveBeenCalledWith("Fotos & mehr/Grüße 🐈");
    expect(screen.getByRole("link", { name: /Bild.jpg/ }).getAttribute("href")).toBe(`/api/v1/download?path=${encodeURIComponent("Fotos & mehr/Bild.jpg")}`);
  });

  it("lässt die leere Freigabe als ZIP laden", () => {
    render(<DirectoryBrowser path="" search="" error="" onLoad={vi.fn()} onSearchChange={vi.fn()} directory={{ path: "", query: "", entries: [], nextCursor: null, nextPage: null }} />);
    expect(screen.getByRole("link", { name: "Ordner als ZIP herunterladen" }).getAttribute("href")).toBe("/api/v1/download-folder?path=");
    expect(screen.getByText("Dieser Ordner ist leer.")).toBeTruthy();
  });
});
