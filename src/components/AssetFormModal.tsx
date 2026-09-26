import { useEffect, useMemo, useState } from "react";
import { api } from "../api";
import type { Asset, Library } from "../types";
import CreatorDialog from "./CreatorDialog";
import ImagePicker from "./ImagePicker";
import ModelPicker from "./ModelPicker";
import SearchSelect from "./SearchSelect";
import SuggestInput from "./SuggestInput";

interface Props {
  root: string;
  library: Library;
  asset: Asset | null; // null = add, otherwise edit
  onLibraryChange: (library: Library) => void;
  /** `created` is true when a brand-new asset was added, false when an existing one was edited. */
  onDone: (library: Library, assetId: string, created: boolean) => void;
  onClose: () => void;
}

/**
 * Add or edit an asset's details. Files are filed per version on the asset page.
 * While adding, typing a name that already exists offers that asset; picking it
 * switches the form to editing it.
 */
export default function AssetFormModal({ root, library, asset, onLibraryChange, onDone, onClose }: Props) {
  const [editing, setEditing] = useState<Asset | null>(asset);
  const [name, setName] = useState(asset?.name ?? "");
  const [creatorId, setCreatorId] = useState<string | null>(asset?.creatorId ?? null);
  const [siteId, setSiteId] = useState<string | null>(asset?.siteId ?? null);
  const [categoryId, setCategoryId] = useState<string | null>(asset?.categoryId ?? null);
  const [sourceUrl, setSourceUrl] = useState(asset?.sourceUrl ?? "");
  const [images, setImages] = useState<string[]>(asset?.imagePaths ?? []);
  const [modelIds, setModelIds] = useState<string[]>(asset?.compatibleModelIds ?? []);
  const [allModels, setAllModels] = useState(asset?.compatibleWithAll ?? false);
  const [creatorDialog, setCreatorDialog] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => e.key === "Escape" && !busy && creatorDialog === null && onClose();
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onClose, busy, creatorDialog]);

  // Existing assets matching the typed name (only while adding a new one).
  const suggestions = useMemo(() => {
    const q = name.trim().toLowerCase();
    if (editing || !q) return [];
    const creators = new Map(library.creators.map((c) => [c.id, c.name]));
    const categories = new Map(library.categories.map((c) => [c.id, c.name]));
    return library.assets
      .filter((a) => a.name.toLowerCase().includes(q))
      .sort((a, b) =>
        Number(b.name.toLowerCase().startsWith(q)) - Number(a.name.toLowerCase().startsWith(q)) ||
        a.name.localeCompare(b.name)
      )
      .slice(0, 8)
      .map((a) => ({
        id: a.id,
        name: a.name,
        detail: `${creators.get(a.creatorId) ?? "?"} · ${categories.get(a.categoryId) ?? "?"}`,
      }));
  }, [name, editing, library.assets, library.creators, library.categories]);

  // Swap the form over to editing an existing asset.
  const loadAsset = async (id: string) => {
    const a = library.assets.find((x) => x.id === id);
    if (!a) return;
    const hasOtherInput =
      !!creatorId || !!siteId || !!categoryId || !!sourceUrl.trim() || images.length > 0 || modelIds.length > 0 || allModels;
    if (hasOtherInput) {
      const ok = await api.confirmAction(
        `“${a.name}” is already in your library. Switch to editing it? The details you've entered in this form will be replaced.`
      );
      if (!ok) return;
    }
    setEditing(a);
    setName(a.name);
    setCreatorId(a.creatorId);
    setSiteId(a.siteId);
    setCategoryId(a.categoryId);
    setSourceUrl(a.sourceUrl ?? "");
    setImages(a.imagePaths);
    setModelIds(a.compatibleModelIds);
    setAllModels(a.compatibleWithAll);
    setError(null);
  };

  // Back to adding a brand-new asset (keeps the typed name).
  const startOver = () => {
    setEditing(null);
    setCreatorId(null);
    setSiteId(null);
    setCategoryId(null);
    setSourceUrl("");
    setImages([]);
    setModelIds([]);
    setAllModels(false);
    setError(null);
  };

  // Sites and categories are just names, so they're created on the spot.
  const createNamed = (add: (name: string) => Promise<Library>, pick: (lib: Library, name: string) => string | undefined) =>
    async (newName: string) => {
      try {
        const lib = await add(newName);
        onLibraryChange(lib);
        return pick(lib, newName.toLowerCase()) ?? null;
      } catch (e) { setError(String(e)); return null; }
    };
  const createSite = createNamed(api.addSite, (lib, n) => lib.sites.find((s) => s.name.toLowerCase() === n)?.id);
  const createCategory = createNamed(api.addCategory, (lib, n) => lib.categories.find((c) => c.name.toLowerCase() === n)?.id);

  const movesFolders =
    !!editing && editing.versions.length > 0 &&
    (name.trim() !== editing.name || creatorId !== editing.creatorId || categoryId !== editing.categoryId);

  const canSubmit = !busy && !!name.trim() && !!creatorId && !!siteId && !!categoryId && (allModels || modelIds.length > 0);

  const submit = async () => {
    if (!creatorId || !siteId || !categoryId) return;
    setBusy(true);
    setError(null);
    try {
      const input = {
        name, creatorId, siteId, categoryId,
        sourceUrl: sourceUrl.trim() || null,
        imagePaths: images,
        compatibleModelIds: allModels ? [] : modelIds,
        compatibleWithAll: allModels,
      };
      if (editing) {
        onDone(await api.updateAsset(editing.id, input), editing.id, false);
      } else {
        const result = await api.addAsset(input);
        onDone(result.library, result.assetId, true);
      }
    } catch (e) {
      setError(String(e));
      setBusy(false);
    }
  };

  return (
    <div className="overlay" onMouseDown={(e) => e.target === e.currentTarget && !busy && onClose()}>
      <div className="modal" role="dialog" aria-label={editing ? "Edit asset" : "Add asset"}>
        <div className="modal-head">
          <h2>{editing ? "Edit asset" : "Add asset"}</h2>
          <button className="btn icon" onClick={onClose} disabled={busy} aria-label="Close">✕</button>
        </div>

        <div className="modal-body">
          <div className="panel">
            {editing && !asset && (
              <div className="banner info">
                <div>
                  <b>“{editing.name}” is already in your library.</b>
                  <div className="small">You're editing the existing entry instead of adding a new one.</div>
                </div>
                <button className="btn" onClick={startOver} disabled={busy}>Add as new instead</button>
              </div>
            )}

            <div className="field">
              <span>Name</span>
              <SuggestInput value={name} onChange={setName} suggestions={suggestions} onPick={loadAsset} disabled={busy} autoFocus />
            </div>

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

            <div className="field">
              <span>Category</span>
              <SearchSelect value={categoryId} options={library.categories} placeholder="Search or create a category (e.g. Clothing)" onChange={setCategoryId} onCreate={createCategory} disabled={busy} />
            </div>

            <ModelPicker
              root={root}
              library={library}
              selectedIds={modelIds}
              allModels={allModels}
              onChange={(ids, all) => { setModelIds(ids); setAllModels(all); }}
              disabled={busy}
            />

            <label className="field">
              <span>Source URL (optional)</span>
              <input className="input" placeholder="https://…" value={sourceUrl} onChange={(e) => setSourceUrl(e.target.value)} disabled={busy} />
            </label>

            <ImagePicker root={root} images={images} onChange={setImages} disabled={busy} />

            {movesFolders && (
              <p className="muted small">Changing the name, creator or category also moves this asset's version folders on disk.</p>
            )}
            {!editing && (
              <p className="muted small">After adding, open the asset and use “Add version” to create its folder and file its files.</p>
            )}

            {error && <div className="error pre">{error}</div>}

            <div className="row">
              <button className="btn primary" onClick={submit} disabled={!canSubmit}>
                {busy ? "Working…" : editing ? "Save changes" : "Add asset"}
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
