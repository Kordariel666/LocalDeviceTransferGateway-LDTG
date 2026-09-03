import { useMemo } from "react";
import type { DirectoryResponse } from "@ldtg/shared";
import { text } from "./i18n";
import { formatBytes } from "./presentation";

type DirectoryBrowserProps = {
  path: string;
  search: string;
  directory: DirectoryResponse | null;
  error: string;
  onSearchChange: (value: string) => void;
  onLoad: (path: string, cursor?: string, page?: number, query?: string) => Promise<void>;
};

export function DirectoryBrowser({
  path,
  search,
  directory,
  error,
  onSearchChange,
  onLoad,
}: DirectoryBrowserProps) {
  const breadcrumbs = useMemo(() => {
    const parts = path.split("/").filter(Boolean);
    return [
      { name: text.shareRoot, path: "" },
      ...parts.map((name, index) => ({
        name,
        path: parts.slice(0, index + 1).join("/"),
      })),
    ];
  }, [path]);

  return (
    <section>
      <p className="eyebrow">{text.readOnly}</p>
      <h1>{text.filesFromPc}</h1>
      <p className="intro">{text.downloadIntro}</p>
      <form className="search-row" onSubmit={(event) => { event.preventDefault(); void onLoad(path); }}>
        <input
          value={search}
          onChange={(event) => onSearchChange(event.target.value)}
          placeholder={text.searchPlaceholder}
          aria-label={text.searchPlaceholder}
        />
        <button type="submit">{text.search}</button>
      </form>
      <div className="breadcrumbs" aria-label={text.currentPath}>
        {breadcrumbs.map((item) => (
          <button type="button" key={item.path} onClick={() => void onLoad(item.path)}>
            <bdi className="untrusted-name">{item.name}</bdi>
          </button>
        ))}
      </div>
      {error && <div className="error-box">{error}</div>}
      <div className="file-list">
        {directory?.entries.map((entry) => entry.kind === "directory" ? (
          <button className="file-row folder-row" type="button" key={entry.path} onClick={() => void onLoad(entry.path)}>
            <span><strong><bdi className="untrusted-name">{entry.name}</bdi></strong><small>{text.folder}</small></span>
            <b>{text.openFolder}</b>
          </button>
        ) : (
          <a className="file-row file-download-row" key={entry.path} href={`/api/v1/download?path=${encodeURIComponent(entry.path)}`} download>
            <span>
              <strong><bdi className="untrusted-name">{entry.name}</bdi></strong>
              <small>
                {formatBytes(entry.size)}
                {entry.modifiedAt ? ` · ${new Intl.DateTimeFormat("de-DE", { dateStyle: "short", timeStyle: "short" }).format(new Date(entry.modifiedAt))}` : ""}
              </small>
            </span>
            <b>{text.load}</b>
          </a>
        ))}
        {directory && !directory.entries.length && <p className="empty">{text.emptyFolder}</p>}
        {directory?.nextCursor && directory.nextPage !== null && (
          <button
            className="more-button"
            onClick={() => void onLoad(path, directory.nextCursor ?? undefined, directory.nextPage ?? undefined, directory.query)}
          >
            {text.moreFiles}
          </button>
        )}
      </div>
    </section>
  );
}
