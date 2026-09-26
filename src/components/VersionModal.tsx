import { useEffect, useState } from "react";
import { api, displayPath } from "../api";
import type { AddVersionResult, Library, Version } from "../types";
import SourcePicker, { toSourceInputs } from "./SourcePicker";
import type { SourceItem, TargetOption } from "./SourcePicker";

/** What the version belongs to: a model or an asset. */
export interface VersionOwner {
  kind: "model" | "asset";
  id: string;
  name: string;
}

interface Props {
  root: string;
  owner: VersionOwner;
  version: Version | null; // null = add a version, otherwise edit it
  /** Assets only: the models a source can be aimed at. */
  targetOptions?: TargetOption[];
  onAdded: (result: AddVersionResult) => void;
  onSaved: (library: Library) => void;
  onClose: () => void;
}

type Preview = { path: string; subfolders: string[] } | { error: string } | null;

export default function VersionModal({ root, owner, version, targetOptions = [], onAdded, onSaved, onClose }: Props) {
  const [label, setLabel] = useState(version?.versionLabel ?? "");
  const [notes, setNotes] = useState(version?.notes ?? "");
  const [folder, setFolder] = useState<string | null>(null); // models: one folder
  const [sources, setSources] = useState<SourceItem[]>([]); // assets: several sources
  const [preview, setPreview] = useState<Preview>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => e.key === "Escape" && !busy && onClose();
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onClose, busy]);

  // Live dry run (adding only), validated by the backend: destination, subfolders, and any problem.
  useEffect(() => {
    if (version || !label.trim()) { setPreview(null); return; }
    let cancelled = false;
    const t = setTimeout(async () => {
      try {
        const result =
          owner.kind === "model"
            ? { path: await api.previewVersionPath(owner.id, label), subfolders: [] as string[] }
            : await api.previewAssetVersion(owner.id, label, toSourceInputs(sources));
        if (!cancelled) setPreview(result);
      } catch (e) {
        if (!cancelled) setPreview({ error: String(e) });
      }
    }, 250);
    return () => { cancelled = true; clearTimeout(t); };
  }, [label, owner.id, owner.kind, version, sources]);

  const chooseFolder = async () => {
    try {
      const picked = await api.pickFolder("Choose the folder holding this version's files");
      if (picked) setFolder(picked);
    } catch (e) { setError(String(e)); }
  };

  const submit = async () => {
    setBusy(true);
    setError(null);
    const notesValue = notes.trim() || null;
    try {
      if (version) {
        onSaved(
          owner.kind === "model"
            ? await api.updateVersion({ modelId: owner.id, folderPath: version.folderPath, version: label, notes: notesValue })
            : await api.updateAssetVersion({ assetId: owner.id, folderPath: version.folderPath, version: label, notes: notesValue })
        );
      } else {
        onAdded(
          owner.kind === "model"
            ? await api.addVersion({ modelId: owner.id, version: label, notes: notesValue, sourceFolder: folder })
            : await api.addAssetVersion({ assetId: owner.id, version: label, notes: notesValue, sources: toSourceInputs(sources) })
        );
      }
    } catch (e) {
      setError(String(e));
      setBusy(false);
    }
  };

  const canSubmit = !busy && !!label.trim() && (version ? true : !!preview && "path" in preview);
  const wide = owner.kind === "asset" && !version;

  return (
    <div className="overlay" onMouseDown={(e) => e.target === e.currentTarget && !busy && onClose()}>
      <div className={`modal ${wide ? "" : "small"}`} role="dialog" aria-label={version ? "Edit version" : "Add version"}>
        <div className="modal-head">
          <h2>{version ? `Edit version ${version.versionLabel}` : `Add version of ${owner.name}`}</h2>
          <button className="btn icon" onClick={onClose} disabled={busy} aria-label="Close">✕</button>
        </div>

        <div className="modal-body">
          <div className="panel">
            <label className="field">
              <span>Version</span>
              <input
                className="input"
                placeholder="e.g. 1.0.0"
                value={label}
                onChange={(e) => setLabel(e.target.value)}
                disabled={busy || !!version?.missing}
                autoFocus
              />
            </label>
            {version && !version.missing && label.trim() !== version.versionLabel && (
              <p className="muted small">Changing the version also renames its folder on disk.</p>
            )}
            {version?.missing && (
              <p className="muted small">This version's folder is missing, so only its notes can be changed.</p>
            )}

            <label className="field">
              <span>Notes (optional)</span>
              <textarea className="input textarea" rows={2} value={notes} onChange={(e) => setNotes(e.target.value)} disabled={busy} />
            </label>

            {!version && (
              <>
                {owner.kind === "asset" ? (
                  <div className="field">
                    <span>Files (optional)</span>
                    <SourcePicker sources={sources} onChange={setSources} targetOptions={targetOptions} disabled={busy} />
                  </div>
                ) : (
                  <div className="field">
                    <span>Files (optional)</span>
                    <div className="row">
                      <div className="path-box grow">{folder ?? "No folder chosen: an empty folder will be created"}</div>
                      <button className="btn" onClick={chooseFolder} disabled={busy}>Choose folder…</button>
                      {folder && <button className="btn" onClick={() => setFolder(null)} disabled={busy}>Clear</button>}
                    </div>
                    <p className="muted small">A chosen folder is moved (not copied) into your library, with its files directly inside the version folder.</p>
                  </div>
                )}

                <div className="field">
                  <span>Destination</span>
                  {preview && "path" in preview ? (
                    <>
                      <div className="path-box ok">{displayPath(root, preview.path)}</div>
                      {preview.subfolders.length > 0 && (
                        <p className="muted small">Subfolders that will be created inside it: {preview.subfolders.join(", ")}</p>
                      )}
                    </>
                  ) : preview && "error" in preview ? (
                    <div className="path-box bad">{preview.error}</div>
                  ) : (
                    <div className="path-box muted">Enter a version to see where it will be filed.</div>
                  )}
                </div>
              </>
            )}

            {error && <div className="error pre">{error}</div>}

            <div className="row">
              <button className="btn primary" onClick={submit} disabled={!canSubmit}>
                {busy ? "Working…" : version ? "Save changes" : "Add version"}
              </button>
              <button className="btn" onClick={onClose} disabled={busy}>Cancel</button>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
