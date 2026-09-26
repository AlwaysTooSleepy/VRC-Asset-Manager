import { useEffect, useRef, useState } from "react";
import { api } from "../api";
import type { Creator, Library } from "../types";
import Avatar from "./Avatar";

interface Props {
  root: string;
  creator: Creator | null; // null = new creator
  initialName?: string;
  usage?: number; // how many items use this creator (blocks deleting)
  onSaved: (library: Library, creatorId: string) => void;
  onDeleted?: (library: Library) => void;
  onCancel: () => void;
}

/** Create or edit a creator: name, icon (file or web address) and profile links. */
export default function CreatorForm({ root, creator, initialName = "", usage = 0, onSaved, onDeleted, onCancel }: Props) {
  const [name, setName] = useState(creator?.name ?? initialName);
  const [iconPath, setIconPath] = useState<string | null>(creator?.iconImagePath ?? null);
  const [urls, setUrls] = useState<string[]>(creator ? [...creator.profileUrls] : []);
  const [iconUrlOpen, setIconUrlOpen] = useState(false);
  const [iconUrl, setIconUrl] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Icons imported while editing that never get saved are cleaned up on exit.
  const imported = useRef<string[]>([]);
  useEffect(() => () => { imported.current.forEach((p) => api.removeImageIfUnused(p).catch(() => {})); }, []);

  const run = async (action: () => Promise<void>) => {
    setBusy(true);
    setError(null);
    try { await action(); } catch (e) { setError(String(e)); } finally { setBusy(false); }
  };

  const useIcon = (rel: string) => {
    imported.current.push(rel);
    setIconPath(rel);
  };

  const chooseIcon = () =>
    run(async () => {
      const file = await api.pickImage();
      if (file) useIcon(await api.importImage(file));
    });

  const downloadIcon = () =>
    run(async () => {
      useIcon(await api.importImageFromUrl(iconUrl));
      setIconUrl("");
      setIconUrlOpen(false);
    });

  const save = () =>
    run(async () => {
      const input = { name, iconImagePath: iconPath, profileUrls: urls.filter((u) => u.trim()) };
      if (creator) {
        onSaved(await api.updateCreator(creator.id, input), creator.id);
      } else {
        const lib = await api.addCreator(input);
        const created = lib.creators.find((c) => c.name.toLowerCase() === name.trim().toLowerCase());
        onSaved(lib, created?.id ?? "");
      }
    });

  const remove = () =>
    run(async () => {
      if (creator && onDeleted) onDeleted(await api.deleteCreator(creator.id));
    });

  const setUrl = (i: number, value: string) => setUrls(urls.map((u, idx) => (idx === i ? value : u)));

  return (
    <div className="panel">
      {creator && (
        <p className="muted small">
          Renaming also renames this creator's folders on disk (under Models and Assets) and updates every item.
        </p>
      )}

      <label className="field">
        <span>Name</span>
        <input className="input" value={name} onChange={(e) => setName(e.target.value)} disabled={busy} autoFocus />
      </label>

      <div className="field">
        <span>Icon</span>
        <div className="row">
          <Avatar root={root} creator={{ name: name || "?", iconImagePath: iconPath }} size={56} />
          <button className="btn" onClick={chooseIcon} disabled={busy}>From file…</button>
          <button className="btn" onClick={() => setIconUrlOpen((o) => !o)} disabled={busy}>From URL…</button>
          {iconPath && <button className="btn" onClick={() => setIconPath(null)} disabled={busy}>Remove</button>}
        </div>
        {iconUrlOpen && (
          <div className="row">
            <input className="input" placeholder="https://…/icon.png" value={iconUrl} onChange={(e) => setIconUrl(e.target.value)} onKeyDown={(e) => e.key === "Enter" && iconUrl.trim() && downloadIcon()} disabled={busy} />
            <button className="btn primary" onClick={downloadIcon} disabled={busy || !iconUrl.trim()}>{busy ? "…" : "Download"}</button>
          </div>
        )}
      </div>

      <div className="field">
        <span>Profile links</span>
        {urls.map((url, i) => (
          <div className="row" key={i}>
            <input className="input" placeholder="https://…" value={url} onChange={(e) => setUrl(i, e.target.value)} disabled={busy} />
            <button className="btn icon" aria-label="Remove link" onClick={() => setUrls(urls.filter((_, idx) => idx !== i))}>✕</button>
          </div>
        ))}
        <button className="btn" onClick={() => setUrls([...urls, ""])} disabled={busy}>+ Add link</button>
      </div>

      {error && <div className="error pre">{error}</div>}

      <div className="row">
        <button className="btn primary" onClick={save} disabled={busy || !name.trim()}>Save</button>
        <button className="btn" onClick={onCancel} disabled={busy}>Cancel</button>
        {creator && onDeleted && (
          <button
            className="btn danger push"
            onClick={remove}
            disabled={busy || usage > 0}
            title={usage > 0 ? `Used by ${usage} item(s): reassign or remove them first` : "Delete creator"}
          >
            Delete
          </button>
        )}
      </div>
    </div>
  );
}
