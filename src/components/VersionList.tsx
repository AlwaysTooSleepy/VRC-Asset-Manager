import { useMemo } from "react";
import { compareVersionsDesc, formatDate, formatDateTime } from "../library";
import type { Version } from "../types";

interface Props {
  versions: Version[];
  /** Folder of the current version (highest present). */
  currentPath: string;
  onOpen: (v: Version) => void;
  onEdit: (v: Version) => void;
  onRemove: (v: Version) => void;
}

/** Version history shared by the model and asset pages. */
export default function VersionList({ versions, currentPath, onOpen, onEdit, onRemove }: Props) {
  const sorted = useMemo(() => [...versions].sort(compareVersionsDesc), [versions]);

  if (sorted.length === 0) {
    return (
      <div className="empty muted small-empty">
        No versions yet. Add a version to create its folder and file its files.
      </div>
    );
  }

  return (
    <ul className="list">
      {sorted.map((v) => (
        <li key={v.folderPath} className="list-row version-row">
          <div className="grow">
            <div>
              <b>{v.versionLabel}</b>
              {v.folderPath === currentPath && !v.missing && <span className="pill small-pill">current</span>}
              {v.missing && <span className="pill danger small-pill">folder missing</span>}
            </div>
            {v.notes && <div className="muted small">{v.notes}</div>}
          </div>
          <span className="muted small" title={formatDateTime(v.dateAdded)}>{formatDate(v.dateAdded)}</span>
          <button className="btn" disabled={v.missing} onClick={() => onOpen(v)}>Open folder</button>
          <button className="btn" onClick={() => onEdit(v)}>Edit</button>
          {v.missing && <button className="btn danger" onClick={() => onRemove(v)}>Remove entry</button>}
        </li>
      ))}
    </ul>
  );
}
