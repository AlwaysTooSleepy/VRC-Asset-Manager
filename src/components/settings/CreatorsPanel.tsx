import { useMemo, useState } from "react";
import type { Creator, Library } from "../../types";
import Avatar from "../Avatar";
import CreatorForm from "../CreatorForm";

interface Props {
  root: string;
  library: Library;
  usage: Record<string, number>;
  onLibraryChange: (library: Library) => void;
  /** Closes Settings and opens this creator's full profile page. */
  onOpenProfile: (id: string) => void;
}

export default function CreatorsPanel({ root, library, usage, onLibraryChange, onOpenProfile }: Props) {
  const [query, setQuery] = useState("");
  const [editing, setEditing] = useState<Creator | "new" | null>(null);

  const visible = useMemo(() => {
    const q = query.trim().toLowerCase();
    return library.creators
      .filter((c) => c.name.toLowerCase().includes(q))
      .sort((a, b) => a.name.localeCompare(b.name));
  }, [library.creators, query]);

  if (editing) {
    const creator = editing === "new" ? null : editing;
    return (
      <div className="panel">
        <div className="row">
          <button className="btn link" onClick={() => setEditing(null)}>‹ Back to creators</button>
          {creator && (
            <button className="btn push" onClick={() => onOpenProfile(creator.id)}>Open profile page</button>
          )}
        </div>
        <h3>{creator ? "Edit creator" : "New creator"}</h3>
        <CreatorForm
          root={root}
          creator={creator}
          usage={creator ? usage[creator.id] ?? 0 : 0}
          onSaved={(lib) => { onLibraryChange(lib); setEditing(null); }}
          onDeleted={(lib) => { onLibraryChange(lib); setEditing(null); }}
          onCancel={() => setEditing(null)}
        />
      </div>
    );
  }

  return (
    <div className="panel">
      <div className="row">
        <input
          className="input"
          type="search"
          placeholder={`Search creators (${library.creators.length})`}
          value={query}
          onChange={(e) => setQuery(e.target.value)}
        />
        <button className="btn primary" onClick={() => setEditing("new")}>+ New creator</button>
      </div>

      <ul className="list">
        {visible.length === 0 && <li className="muted list-empty">{library.creators.length ? "No matches" : "No creators yet"}</li>}
        {visible.map((c) => {
          const used = usage[c.id] ?? 0;
          return (
            <li key={c.id} className="list-row clickable" onClick={() => setEditing(c)}>
              <Avatar root={root} creator={c} />
              <span className="grow">{c.name}</span>
              <span className="muted small">
                {c.profileUrls.length} link{c.profileUrls.length === 1 ? "" : "s"} · {used ? `${used} item${used === 1 ? "" : "s"}` : "unused"}
              </span>
            </li>
          );
        })}
      </ul>
    </div>
  );
}
