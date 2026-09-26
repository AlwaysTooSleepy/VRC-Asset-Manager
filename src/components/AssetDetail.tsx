import { useMemo, useState } from "react";
import { api } from "../api";
import { assetCountsByModel, formatDate, formatDateTime, modelsForAsset, MODEL_CATEGORY_NAME } from "../library";
import type { Library, Notice, Version } from "../types";
import AssetFormModal from "./AssetFormModal";
import CreatorLink from "./CreatorLink";
import ImageCarousel from "./ImageCarousel";
import ModelCard from "./ModelCard";
import VersionList from "./VersionList";
import VersionModal from "./VersionModal";

interface Props {
  root: string;
  library: Library;
  assetId: string;
  onLibraryChange: (library: Library) => void;
  onNotice: (notice: Notice | null) => void;
  onOpenModel: (id: string) => void;
  onOpenCreator: (id: string) => void;
  onBack: () => void;
}

type VersionModalState = { mode: "add" } | { mode: "edit"; version: Version } | null;

export default function AssetDetail({ root, library, assetId, onLibraryChange, onNotice, onOpenModel, onOpenCreator, onBack }: Props) {
  const [error, setError] = useState<string | null>(null);
  const [editOpen, setEditOpen] = useState(false);
  const [versionModal, setVersionModal] = useState<VersionModalState>(null);
  const asset = library.assets.find((a) => a.id === assetId);
  const models = useMemo(
    () => (asset ? modelsForAsset(library, asset).sort((a, b) => b.dateUpdated.localeCompare(a.dateUpdated)) : []),
    [library, asset]
  );
  const counts = useMemo(() => assetCountsByModel(library), [library]);
  const creators = useMemo(() => new Map(library.creators.map((c) => [c.id, c])), [library.creators]);
  const siteNames = useMemo(() => new Map(library.sites.map((s) => [s.id, s.name])), [library.sites]);

  if (!asset) {
    return (
      <div className="detail">
        <button className="btn link" onClick={onBack}>‹ Back</button>
        <p className="muted">This asset no longer exists.</p>
      </div>
    );
  }

  const creator = library.creators.find((c) => c.id === asset.creatorId);
  const site = library.sites.find((s) => s.id === asset.siteId);
  const category = library.categories.find((c) => c.id === asset.categoryId);
  const hasFolder = asset.versions.length > 0 && !asset.missing;
  const removable = asset.versions.every((v) => v.missing);
  const noCompat = !asset.compatibleWithAll && asset.compatibleModelIds.length === 0;

  const act = (action: () => Promise<void>) => {
    setError(null);
    action().catch((e) => setError(String(e)));
  };

  const removeVersion = (v: Version) =>
    act(async () => {
      const ok = await api.confirmAction(`Remove the entry for version ${v.versionLabel}? Its folder is already gone, and nothing on disk is deleted.`);
      if (ok) onLibraryChange(await api.removeAssetVersion(asset.id, v.folderPath));
    });

  const removeAsset = () =>
    act(async () => {
      const ok = await api.confirmAction(`Remove “${asset.name}” from your library? Its folders are already gone, and nothing on disk is deleted.`);
      if (!ok) return;
      onLibraryChange(await api.removeAsset(asset.id));
      onBack();
    });

  return (
    <div className="detail">
      <button className="btn link" onClick={onBack}>‹ Back</button>

      {asset.missing && (
        <div className="banner warn">
          <div>
            <b>This asset's folders were not found on disk.</b>
            <div className="small">If you moved them, move them back and press ↻ in the top bar. If they're gone for good, remove the entry.</div>
          </div>
          {removable && <button className="btn danger" onClick={removeAsset}>Remove entry</button>}
        </div>
      )}

      <div className="detail-top">
        <div className="detail-hero">
          <ImageCarousel root={root} images={asset.imagePaths} name={asset.name} />
        </div>

        <div className="detail-info">
          <h1>{asset.name}</h1>

          <div className="row">
            <CreatorLink root={root} creator={creator} onOpen={() => onOpenCreator(asset.creatorId)} />
            <span className="pill">{category?.name ?? "Unknown category"}</span>
            <span className="pill">{site?.name ?? "Unknown site"}</span>
          </div>

          <div className="stats">
            <div><b>{asset.compatibleWithAll ? "All" : models.length}</b><span className="muted small">compatible model{!asset.compatibleWithAll && models.length === 1 ? "" : "s"}</span></div>
            <div><b>{formatDate(asset.dateAdded)}</b><span className="muted small">added</span></div>
            <div><b title={formatDateTime(asset.dateUpdated)}>{formatDate(asset.dateUpdated)}</b><span className="muted small">updated</span></div>
          </div>

          <div className="row">
            <button className="btn primary" disabled={!hasFolder} onClick={() => act(() => api.openFolder(asset.folderPath))}>Open folder</button>
            {asset.sourceUrl && (
              <button className="btn" onClick={() => act(() => api.openUrl(asset.sourceUrl as string))}>Open source page</button>
            )}
            <button className="btn" onClick={() => setEditOpen(true)}>Edit</button>
          </div>

          {error && <div className="error pre">{error}</div>}
        </div>
      </div>

      <section>
        <div className="section-head">
          <h3>Version history</h3>
          <button className="btn primary" onClick={() => setVersionModal({ mode: "add" })}>+ Add version</button>
        </div>
        <VersionList
          versions={asset.versions}
          currentPath={asset.folderPath}
          onOpen={(v) => act(() => api.openFolder(v.folderPath))}
          onEdit={(v) => setVersionModal({ mode: "edit", version: v })}
          onRemove={removeVersion}
        />
      </section>

      <section>
        <h3>Compatible models ({asset.compatibleWithAll ? `all ${models.length}` : models.length})</h3>
        {noCompat ? (
          <div className="banner warn">
            <div>
              <b>No compatible models selected.</b>
              <div className="small">This asset was found on disk. Edit it to choose which models it works with.</div>
            </div>
            <button className="btn" onClick={() => setEditOpen(true)}>Edit</button>
          </div>
        ) : models.length === 0 ? (
          <div className="empty muted small-empty">No matching models in your library.</div>
        ) : (
          <>
            {asset.compatibleWithAll && <p className="muted small section-note">This asset works with all models.</p>}
            <div className="grid">
              {models.map((m) => (
                <ModelCard
                  key={m.id}
                  root={root}
                  model={m}
                  creator={creators.get(m.creatorId)}
                  siteName={siteNames.get(m.siteId) ?? "Unknown site"}
                  categoryName={MODEL_CATEGORY_NAME}
                  assetCount={counts[m.id] ?? 0}
                  onOpen={() => onOpenModel(m.id)}
                  onOpenCreator={() => onOpenCreator(m.creatorId)}
                />
              ))}
            </div>
          </>
        )}
      </section>

      {editOpen && (
        <AssetFormModal
          root={root}
          library={library}
          asset={asset}
          onLibraryChange={onLibraryChange}
          onDone={(lib) => { onLibraryChange(lib); setEditOpen(false); }}
          onClose={() => setEditOpen(false)}
        />
      )}

      {versionModal && (
        <VersionModal
          root={root}
          owner={{ kind: "asset", id: asset.id, name: asset.name }}
          targetOptions={models.map((m) => ({ id: m.id, name: m.name, label: `${m.name} — ${creators.get(m.creatorId)?.name ?? "?"}` }))}
          version={versionModal.mode === "edit" ? versionModal.version : null}
          onAdded={(result) => {
            onLibraryChange(result.library);
            setVersionModal(null);
            onNotice({ text: `Version added at ${result.destination}`, warning: result.warning });
          }}
          onSaved={(lib) => { onLibraryChange(lib); setVersionModal(null); }}
          onClose={() => setVersionModal(null)}
        />
      )}
    </div>
  );
}
