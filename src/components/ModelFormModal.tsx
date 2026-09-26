import { useEffect, useState } from "react";
import { api } from "../api";
import type { Library, Model } from "../types";
import CreatorDialog from "./CreatorDialog";
import ImagePicker from "./ImagePicker";
import SearchSelect from "./SearchSelect";

interface Props {
  root: string;
  library: Library;
  model: Model | null; // null = add, otherwise edit
  onLibraryChange: (library: Library) => void;
  onDone: (library: Library, modelId: string) => void;
  onClose: () => void;
}

/** Add or edit a model's details. Files are filed per version on the model page. */
export default function ModelFormModal({ root, library, model, onLibraryChange, onDone, onClose }: Props) {
  const [name, setName] = useState(model?.name ?? "");
  const [creatorId, setCreatorId] = useState<string | null>(model?.creatorId ?? null);
  const [siteId, setSiteId] = useState<string | null>(model?.siteId ?? null);
  const [sourceUrl, setSourceUrl] = useState(model?.sourceUrl ?? "");
  const [images, setImages] = useState<string[]>(model?.imagePaths ?? []);
  const [creatorDialog, setCreatorDialog] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => e.key === "Escape" && !busy && creatorDialog === null && onClose();
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onClose, busy, creatorDialog]);

  const createSite = async (newName: string) => {
    try {
      const lib = await api.addSite(newName);
      onLibraryChange(lib);
      return lib.sites.find((s) => s.name.toLowerCase() === newName.toLowerCase())?.id ?? null;
    } catch (e) { setError(String(e)); return null; }
  };

  const movesFolders =
    !!model && model.versions.length > 0 && (name.trim() !== model.name || creatorId !== model.creatorId);

  const submit = async () => {
    if (!creatorId || !siteId) return;
    setBusy(true);
    setError(null);
    try {
      const input = { name, creatorId, siteId, sourceUrl: sourceUrl.trim() || null, imagePaths: images };
      if (model) {
        onDone(await api.updateModel(model.id, input), model.id);
      } else {
        const result = await api.addModel(input);
        onDone(result.library, result.modelId);
      }
    } catch (e) {
      setError(String(e));
      setBusy(false);
    }
  };

  return (
    <div className="overlay" onMouseDown={(e) => e.target === e.currentTarget && !busy && onClose()}>
      <div className="modal" role="dialog" aria-label={model ? "Edit model" : "Add model"}>
        <div className="modal-head">
          <h2>{model ? "Edit model" : "Add model"}</h2>
          <button className="btn icon" onClick={onClose} disabled={busy} aria-label="Close">✕</button>
        </div>

        <div className="modal-body">
          <div className="panel">
            <label className="field">
              <span>Name</span>
              <input className="input" value={name} onChange={(e) => setName(e.target.value)} disabled={busy} autoFocus />
            </label>

            <div className="grid2">
              <div className="field">
                <span>Creator</span>
                <SearchSelect
                  value={creatorId}
                  options={library.creators}
                  placeholder="Search or create a creator"
                  onChange={setCreatorId}
                  onCreate={async (n) => { setCreatorDialog(n); return null; }}
                  disabled={busy}
                />
              </div>
              <div className="field">
                <span>Site</span>
                <SearchSelect value={siteId} options={library.sites} placeholder="Search or create a site" onChange={setSiteId} onCreate={createSite} disabled={busy} />
              </div>
            </div>

            <label className="field">
              <span>Source URL (optional)</span>
              <input className="input" placeholder="https://…" value={sourceUrl} onChange={(e) => setSourceUrl(e.target.value)} disabled={busy} />
            </label>

            <ImagePicker root={root} images={images} onChange={setImages} disabled={busy} />

            {movesFolders && (
              <p className="muted small">Changing the name or creator also renames or moves this model's version folders on disk.</p>
            )}
            {!model && (
              <p className="muted small">After adding, open the model and use “Add version” to create its folder and file its files.</p>
            )}

            {error && <div className="error pre">{error}</div>}

            <div className="row">
              <button className="btn primary" onClick={submit} disabled={busy || !name.trim() || !creatorId || !siteId}>
                {busy ? "Working…" : model ? "Save changes" : "Add model"}
              </button>
              <button className="btn" onClick={onClose} disabled={busy}>Cancel</button>
            </div>
          </div>
        </div>

        {creatorDialog !== null && (
          <CreatorDialog
            root={root}
            initialName={creatorDialog}
            onSaved={(lib, id) => { onLibraryChange(lib); if (id) setCreatorId(id); setCreatorDialog(null); }}
            onCancel={() => setCreatorDialog(null)}
          />
        )}
      </div>
    </div>
  );
}
