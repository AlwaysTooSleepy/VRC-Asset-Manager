import type { Library } from "../../types";

interface Props {
  root: string;
  library: Library;
  onLibraryChange: (library: Library) => void;
  onChangeRoot: () => void;
}

export default function LibraryPanel({ root, library, onChangeRoot }: Props) {
  return (
    <div className="panel">
      <h3>Library folder</h3>
      <div className="path-box">{root}</div>
      <button className="btn" onClick={onChangeRoot}>Change folder…</button>
      <p className="muted small">
        Choosing a different folder switches to the library stored there (a new one is created if
        the folder is empty). Your existing files are not moved.
      </p>

      <h3>Contents</h3>
      <p className="muted">
        {library.models.length} models · {library.assets.length} assets · {library.creators.length}{" "}
        creators · {library.sites.length} sites · {library.categories.length} categories
      </p>
    </div>
  );
}
