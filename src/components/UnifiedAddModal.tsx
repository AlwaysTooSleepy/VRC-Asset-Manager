import { useEffect, useMemo, useRef, useState } from "react";
import { api } from "../api";
import type { Asset, Library } from "../types";
import CreatorDialog from "./CreatorDialog";
import ImagePicker from "./ImagePicker";
import ModelPicker from "./ModelPicker";
import SearchSelect from "./SearchSelect";
import SuggestInput from "./SuggestInput";

type AddKind = "model" | "asset";

interface Props {
  root: string;
  library: Library;
  onLibraryChange: (library: Library) => void;
  onDone: (library: Library, itemId: string, kind: AddKind) => void;
  onClose: () => void;
}

export default function UnifiedAddModal({ root, library, onLibraryChange, onDone, onClose }: Props) {
  const [kind, setKind] = useState<AddKind>("model");
  const [name, setName] = useState("");
  const [editingAsset, setEditingAsset] = useState<Asset | null>(null);
  const [creatorId, setCreatorId] = useState<string | null>(null);
  const [siteId, setSiteId] = useState<string | null>(null);
  const [categoryId, setCategoryId] = useState<string | null>(null);
  const [sourceUrl, setSourceUrl] = useState("");
  const [images, setImages] = useState<string[]>([]);
  const [modelIds, setModelIds] = useState<string[]>([]);
  const [allModels, setAllModels] = useState(false);
  const [creatorDialog, setCreatorDialog] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [detectedSite, setDetectedSite] = useState<string | null>(null);
  const [fetching, setFetching] = useState(false);
  const [fetchNotice, setFetchNotice] = useState<string | null>(null);
  const detectTimeout = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => e.key === "Escape" && !busy && creatorDialog === null && onClose();
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onClose, busy, creatorDialog]);

  const suggestions = useMemo(() => {
    const q = name.trim().toLowerCase();
    if (!q) return [];

    const creators = new Map(library.creators.map((c) => [c.id, c.name]));
    const categories = new Map(library.categories.map((c) => [c.id, c.name]));

    if (kind === "model") {
      return library.models
        .filter((item) => item.name.toLowerCase().includes(q))
        .sort((a, b) =>
          Number(b.name.toLowerCase().startsWith(q)) - Number(a.name.toLowerCase().startsWith(q)) ||
          a.name.localeCompare(b.name)
        )
        .slice(0, 8)
        .map((item) => ({
          id: item.id,
          name: item.name,
          detail: `${creators.get(item.creatorId) ?? "?"} · ${library.sites.find((site) => site.id === item.siteId)?.name ?? "?"}`,
        }));
    }

    return library.assets
      .filter((item) => item.name.toLowerCase().includes(q))
      .sort((a, b) =>
        Number(b.name.toLowerCase().startsWith(q)) - Number(a.name.toLowerCase().startsWith(q)) ||
        a.name.localeCompare(b.name)
      )
      .slice(0, 8)
      .map((item) => ({
        id: item.id,
        name: item.name,
        detail: `${creators.get(item.creatorId) ?? "?"} · ${categories.get(item.categoryId) ?? "?"}`,
      }));
  }, [kind, name, library.assets, library.categories, library.creators, library.models, library.sites]);

  const maybeLoadExistingAsset = async (id: string) => {
    if (kind !== "asset") return;
    const asset = library.assets.find((item) => item.id === id);
    if (!asset) return;

    const hasOtherInput = !!creatorId || !!siteId || !!categoryId || !!sourceUrl.trim() || images.length > 0 || modelIds.length > 0 || allModels;
    if (hasOtherInput) {
      const ok = await api.confirmAction(`“${asset.name}” is already in your library. Switch to editing it? The details you've entered in this form will be replaced.`);
      if (!ok) return;
    }

    setEditingAsset(asset);
    setName(asset.name);
    setCreatorId(asset.creatorId);
    setSiteId(asset.siteId);
    setCategoryId(asset.categoryId);
    setSourceUrl(asset.sourceUrl ?? "");
    setImages(asset.imagePaths);
    setModelIds(asset.compatibleModelIds);
    setAllModels(asset.compatibleWithAll);
    setError(null);
  };

  const startAssetOver = () => {
    setEditingAsset(null);
    setCreatorId(null);
    setSiteId(null);
    setCategoryId(null);
    setSourceUrl("");
    setImages([]);
    setModelIds([]);
    setAllModels(false);
    setError(null);
  };

  const onSourceUrlChange = (url: string) => {
    setSourceUrl(url);
    setFetchNotice(null);
    if (detectTimeout.current) clearTimeout(detectTimeout.current);
    const trimmed = url.trim();
    if (!trimmed) { setDetectedSite(null); return; }
    // Debounce detection by 300 ms so we don't fire on every keystroke.
    detectTimeout.current = setTimeout(async () => {
      try {
        const site = await api.detectSupportedSite(trimmed);
        setDetectedSite(site);
      } catch {
        setDetectedSite(null);
      }
    }, 300);
  };

  const fetchMeta = async () => {
    const trimmed = sourceUrl.trim();
    if (!trimmed) return;
    setFetching(true);
    setFetchNotice(null);
    try {
      const meta = await api.fetchPageMeta(trimmed);
      let filled: string[] = [];
      // Local running copy of the library so a site we just created is
      // visible to the creator-matching step right after, without waiting
      // on a state re-render.
      let lib = library;

      if (meta.name && !name.trim()) { setName(meta.name); filled.push("name"); }

      if (meta.siteName && !siteId) {
        const match = lib.sites.find(
          (s) => s.name.toLowerCase() === meta.siteName!.toLowerCase()
        );
        if (match) { setSiteId(match.id); filled.push("site"); }
        else {
          try {
            lib = await api.addSite(meta.siteName);
            onLibraryChange(lib);
            const newSite = lib.sites.find(
              (s) => s.name.toLowerCase() === meta.siteName!.toLowerCase()
            );
            if (newSite) { setSiteId(newSite.id); filled.push("site"); }
          } catch { /* non-fatal */ }
        }
      }

      if (meta.creatorName && !creatorId) {
        const match = lib.creators.find(
          (c) => c.name.toLowerCase() === meta.creatorName!.toLowerCase()
        );
        if (match) {
          setCreatorId(match.id);
          filled.push("creator");
        } else {
          // Creator doesn't exist yet — create it automatically, pulling in
          // a shop/creator icon from the page when we found one.
          try {
            let iconPath: string | null = null;
            if (meta.creatorIconUrl) {
              try {
                iconPath = await api.importImageFromUrl(meta.creatorIconUrl);
              } catch { /* icon is a nice-to-have, not worth failing over */ }
            }
            lib = await api.addCreator({
              name: meta.creatorName,
              iconImagePath: iconPath,
              profileUrls: meta.creatorProfileUrl ? [meta.creatorProfileUrl] : [],
            });
            onLibraryChange(lib);
            const created = lib.creators.find(
              (c) => c.name.toLowerCase() === meta.creatorName!.toLowerCase()
            );
            if (created) { setCreatorId(created.id); filled.push("creator (new)"); }
          } catch { /* non-fatal — the creator field is still editable manually */ }
        }
      }

      if (meta.imageUrls.length > 0 && images.length === 0) {
        const results = await Promise.allSettled(meta.imageUrls.map((url) => api.importImageFromUrl(url)));
        const rels = results
          .filter((r): r is PromiseFulfilledResult<string> => r.status === "fulfilled")
          .map((r) => r.value);
        if (rels.length > 0) {
          setImages(rels);
          filled.push(rels.length === 1 ? "image" : `images (${rels.length})`);
        }
      }

      setFetchNotice(
        filled.length > 0
          ? `Filled in: ${filled.join(", ")}.`
          : "No new fields could be filled in automatically."
      );
    } catch (e) {
      setFetchNotice(`Fetch failed: ${String(e)}`);
    } finally {
      setFetching(false);
    }
  };

  const createSite = async (newName: string) => {
    try {
      const lib = await api.addSite(newName);
      onLibraryChange(lib);
      return lib.sites.find((site) => site.name.toLowerCase() === newName.toLowerCase())?.id ?? null;
    } catch (e) {
      setError(String(e));
      return null;
    }
  };

  const createCategory = async (newName: string) => {
    try {
      const lib = await api.addCategory(newName);
      onLibraryChange(lib);
      return lib.categories.find((category) => category.name.toLowerCase() === newName.toLowerCase())?.id ?? null;
    } catch (e) {
      setError(String(e));
      return null;
    }
  };

  const canSubmit = !busy && !!name.trim() && !!creatorId && !!siteId && (kind === "model" || !!categoryId) && (!allModels || kind === "asset") && (kind === "model" || allModels || modelIds.length > 0);

  const submit = async () => {
    if (!creatorId || !siteId) return;
    if (kind === "asset" && !categoryId) return;

    const nextCreatorId = creatorId;
    const nextSiteId = siteId;
    const nextCategoryId = categoryId;

    setBusy(true);
    setError(null);

    try {
      if (kind === "model") {
        const result = await api.addModel({
          name,
          creatorId: nextCreatorId,
          siteId: nextSiteId,
          sourceUrl: sourceUrl.trim() || null,
          imagePaths: images,
        });
        onDone(result.library, result.modelId, "model");
        return;
      }

      if (!nextCategoryId) return;

      const result = await api.addAsset({
        name,
        creatorId: nextCreatorId,
        siteId: nextSiteId,
        categoryId: nextCategoryId,
        sourceUrl: sourceUrl.trim() || null,
        imagePaths: images,
        compatibleModelIds: allModels ? [] : modelIds,
        compatibleWithAll: allModels,
      });
      onDone(result.library, result.assetId, "asset");
    } catch (e) {
      setError(String(e));
      setBusy(false);
    }
  };

  return (
    <div className="overlay" onMouseDown={(e) => e.target === e.currentTarget && !busy && onClose()}>
      <div className="modal" role="dialog" aria-label="Add item" style={{ width: "min(760px, 92vw)", maxHeight: "90vh" }}>
        <div className="modal-head">
          <h2>Add item</h2>
          <button className="btn icon" onClick={onClose} disabled={busy} aria-label="Close">✕</button>
        </div>

        <div className="modal-body">
          <div className="panel">
            <div className="field">
              <span>Type</span>
              <div className="segmented" style={{ display: "flex", gap: 8 }}>
                <button className={`btn ${kind === "model" ? "primary" : ""}`} onClick={() => setKind("model")} disabled={busy}>Model</button>
                <button className={`btn ${kind === "asset" ? "primary" : ""}`} onClick={() => setKind("asset")} disabled={busy}>Asset</button>
              </div>
            </div>

            {kind === "asset" && editingAsset && !library.assets.some((item) => item.id === editingAsset.id) && (
              <div className="banner info">
                <div>
                  <b>“{editingAsset.name}” is already in your library.</b>
                  <div className="small">You're editing the existing entry instead of adding a new one.</div>
                </div>
                <button className="btn" onClick={startAssetOver} disabled={busy}>Add as new instead</button>
              </div>
            )}

            <div className="field">
              <span>{kind === "model" ? "Model name" : "Asset name"}</span>
              <SuggestInput value={name} onChange={setName} suggestions={suggestions} onPick={maybeLoadExistingAsset} disabled={busy} autoFocus />
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

            {kind === "asset" && (
              <div className="field">
                <span>Category</span>
                <SearchSelect value={categoryId} options={library.categories} placeholder="Search or create a category (e.g. Clothing)" onChange={setCategoryId} onCreate={createCategory} disabled={busy} />
              </div>
            )}

            {kind === "asset" && (
              <ModelPicker
                root={root}
                library={library}
                selectedIds={modelIds}
                allModels={allModels}
                onChange={(ids, all) => { setModelIds(ids); setAllModels(all); }}
                disabled={busy}
              />
            )}

            <div className="field">
              <span>Source URL (optional)</span>
              <div style={{ display: "flex", gap: 8 }}>
                <input
                  className="input"
                  placeholder="https://…"
                  value={sourceUrl}
                  onChange={(e) => onSourceUrlChange(e.target.value)}
                  disabled={busy || fetching}
                  style={{ flex: 1 }}
                />
                {detectedSite && (
                  <button
                    className="btn"
                    onClick={fetchMeta}
                    disabled={busy || fetching}
                    title={`Fetch metadata from ${detectedSite}`}
                  >
                    {fetching ? "Fetching…" : `Fetch from ${detectedSite}`}
                  </button>
                )}
              </div>
              {fetchNotice && (
                <div className={`small ${fetchNotice.startsWith("Fetch failed") ? "muted" : "muted"}`} style={{ marginTop: 4 }}>
                  {fetchNotice}
                </div>
              )}
            </div>

            <ImagePicker root={root} images={images} onChange={setImages} disabled={busy} />

            {kind === "model" && (
              <p className="muted small">After adding, open the model and use “Add version” to create the version folder and file the files.</p>
            )}
            {kind === "asset" && !editingAsset && (
              <p className="muted small">After adding, open the asset and use “Add version” to create the version folder and file the files.</p>
            )}

            {error && <div className="error pre">{error}</div>}

            <div className="row">
              <button className="btn primary" onClick={submit} disabled={!canSubmit}>
                {busy ? "Working…" : kind === "model" ? "Add model" : "Add asset"}
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
