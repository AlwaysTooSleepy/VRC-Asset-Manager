import { useMemo, useState } from "react";

interface Item { id: string; name: string }

interface Props {
  kind: "site" | "category";
  hint?: string;
  items: Item[];
  usage: Record<string, number>;
  onAdd: (name: string) => Promise<void>;
  onRename: (id: string, name: string) => Promise<void>;
  onDelete: (id: string) => Promise<void>;
}

/** Searchable add / rename / delete list, shared by Sites and Categories. */
export default function ReferenceListPanel({ kind, hint, items, usage, onAdd, onRename, onDelete }: Props) {
  const [query, setQuery] = useState("");
  const [newName, setNewName] = useState("");
  const [editingId, setEditingId] = useState<string | null>(null);
  const [editName, setEditName] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const visible = useMemo(() => {
    const q = query.trim().toLowerCase();
    return items
      .filter((i) => i.name.toLowerCase().includes(q))
      .sort((a, b) => a.name.localeCompare(b.name));
  }, [items, query]);

  const run = async (action: () => Promise<void>): Promise<boolean> => {
    setBusy(true);
    setError(null);
    try {
      await action();
      return true;
    } catch (e) {
      setError(String(e));
      return false;
    } finally {
      setBusy(false);
    }
  };

  const add = async () => {
    if (!newName.trim()) return;
    if (await run(() => onAdd(newName))) setNewName("");
  };

  const saveRename = async (id: string) => {
    if (await run(() => onRename(id, editName))) setEditingId(null);
  };

  return (
    <div className="panel">
      {hint && <p className="muted small">{hint}</p>}

      <div className="row">
        <input
          className="input"
          placeholder={`New ${kind} name`}
          value={newName}
          onChange={(e) => setNewName(e.target.value)}
          onKeyDown={(e) => e.key === "Enter" && add()}
          disabled={busy}
        />
        <button className="btn primary" onClick={add} disabled={busy || !newName.trim()}>Add</button>
      </div>

      <input
        className="input"
        type="search"
        placeholder={`Search ${kind}s (${items.length})`}
        value={query}
        onChange={(e) => setQuery(e.target.value)}
      />

      {error && <div className="error">{error}</div>}

      <ul className="list">
        {visible.length === 0 && <li className="muted list-empty">{items.length ? "No matches" : `No ${kind}s yet`}</li>}
        {visible.map((item) => {
          const used = usage[item.id] ?? 0;
          return (
            <li key={item.id} className="list-row">
              {editingId === item.id ? (
                <>
                  <input
                    className="input"
                    autoFocus
                    value={editName}
                    onChange={(e) => setEditName(e.target.value)}
                    onKeyDown={(e) => {
                      if (e.key === "Enter") saveRename(item.id);
                      if (e.key === "Escape") { e.stopPropagation(); setEditingId(null); }
                    }}
                    disabled={busy}
                  />
                  <button className="btn primary" onClick={() => saveRename(item.id)} disabled={busy}>Save</button>
                  <button className="btn" onClick={() => setEditingId(null)} disabled={busy}>Cancel</button>
                </>
              ) : (
                <>
                  <span className="grow">{item.name}</span>
                  <span className="muted small">{used ? `${used} item${used === 1 ? "" : "s"}` : "unused"}</span>
                  <button className="btn" disabled={busy} onClick={() => { setEditingId(item.id); setEditName(item.name); setError(null); }}>Rename</button>
                  <button
                    className="btn danger"
                    disabled={busy || used > 0}
                    title={used > 0 ? "In use: reassign or remove those items first" : "Delete"}
                    onClick={() => run(() => onDelete(item.id))}
                  >
                    Delete
                  </button>
                </>
              )}
            </li>
          );
        })}
      </ul>
    </div>
  );
}
