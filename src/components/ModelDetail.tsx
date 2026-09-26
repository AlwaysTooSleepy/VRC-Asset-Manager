import { useMemo, useState } from "react";
import { api } from "../api";
import { assetsForModel, formatDate, formatDateTime } from "../library";
import type { Library, Notice, Version } from "../types";
import AssetCard from "./AssetCard";
import CategoryFilter from "./CategoryFilter";
import CreatorLink from "./CreatorLink";
import ImageCarousel from "./ImageCarousel";
import ModelFormModal from "./ModelFormModal";
import VersionList from "./VersionList";
import VersionModal from "./VersionModal";

interface Props {
  root: string;
  library: Library;
  modelId: string;
  onLibraryChange: (library: Library) => void;
  onNotice: (notice: Notice | null) => void;
  onOpenAsset: (id: string) => void;
  onOpenCreator: (id: string) => void;
  onBack: () => void;
}

type VersionModalState = { mode: "add" } | { mode: "edit"; version: Version } | null;

export default function ModelDetail({ root, library, modelId, onLibraryChange, onNotice, onOpenAsset, onOpenCreator, onBack }: Props) {
  const [error, setError] = useState<string | null>(null);
  const [editOpen, setEditOpen] = useState(false);
  const [versionModal, setVersionModal] = useState<VersionModalState>(null);
  const [categoryFilter, setCategoryFilter] = useState<string | null>(null);
  const model = library.models.find((m) => m.id === modelId);
  const compatible = useMemo(
    () => assetsForModel(library, modelId).sort((a, b) => b.dateUpdated.localeCompare(a.dateUpdated)),
    [library, modelId]
  );
  const creators = useMemo(() => new Map(library.creators.map((c) => [c.id, c])), [library.creators]);
  const categoryNames = useMemo(() => new Map(library.categories.map((c) => [c.id, c.name])), [library.categories]);
  const usedCategories = useMemo(
    () => library.categories.filter((c) => compatible.some((a) => a.categoryId === c.id)),
    [library.categories, compatible]
  );
  const filtered = useMemo(
    () => (categoryFilter ? compatible.filter((a) => a.categoryId === categoryFilter) : compatible),
    [compatible, categoryFilter]
  );
  const siteNames = useMemo(() => new Map(library.sites.map((s) => [s.id, s.name])), [library.sites]);

  if (!model) {
    return (
      <div className="detail">
        <button className="btn link" onClick={onBack}>‹ Back</button>
        <p className="muted">This model no longer exists.</p>
      </div>
    );
  }

  const creator = library.creators.find((c) => c.id === model.creatorId);
  const site = library.sites.find((s) => s.id === model.siteId);
  const hasFolder = model.versions.length > 0 && !model.missing;
  const removable = model.versions.every((v) => v.missing);

  const act = (action: () => Promise<void>) => {
    setError(null);
    action().catch((e) => setError(String(e)));
  };

  const removeVersion = (v: Version) =>
    act(async () => {
      const ok = await api.confirmAction(`Remove the entry for version ${v.versionLabel}? Its folder is already gone, and nothing on disk is deleted.`);
      if (ok) onLibraryChange(await api.removeVersion(model.id, v.folderPath));
    });

  const removeModel = () =>
    act(async () => {
      const ok = await api.confirmAction(`Remove “${model.name}” from your library? Its folders are already gone, and nothing on disk is deleted.`);
      if (!ok) return;
      onLibraryChange(await api.removeModel(model.id));
      onBack();
    });

  return (
    <div className="detail">
      <button className="btn link" onClick={onBack}>‹ Back</button>

      {model.missing && (
        <div className="banner warn">
          <div>
            <b>This model's folders were not found on disk.</b>
            <div className="small">If you moved them, move them back and press ↻ in the top bar. If they're gone for good, remove the entry.</div>
          </div>
          {removable && <button className="btn danger" onClick={removeModel}>Remove entry</button>}
        </div>
      )}

      <div className="detail-top">
        <div className="detail-hero">
          <ImageCarousel root={root} images={model.imagePaths} name={model.name} />
        </div>

        <div className="detail-info">
          <h1>{model.name}</h1>

          <div className="row">
            <CreatorLink root={root} creator={creator} onOpen={() => onOpenCreator(model.creatorId)} />
            <span className="pill">{site?.name ?? "Unknown site"}</span>
          </div>

          <div className="stats">
            <div><b>{compatible.length}</b><span className="muted small">compatible asset{compatible.length === 1 ? "" : "s"}</span></div>
            <div><b>{formatDate(model.dateAdded)}</b><span className="muted small">added</span></div>
            <div><b title={formatDateTime(model.dateUpdated)}>{formatDate(model.dateUpdated)}</b><span className="muted small">updated</span></div>
          </div>

          <div className="row">
            <button className="btn primary" disabled={!hasFolder} onClick={() => act(() => api.openFolder(model.folderPath))}>Open folder</button>
            {model.sourceUrl && (
              <button className="btn" onClick={() => act(() => api.openUrl(model.sourceUrl as string))}>Open source page</button>
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
          versions={model.versions}
          currentPath={model.folderPath}
          onOpen={(v) => act(() => api.openFolder(v.folderPath))}
          onEdit={(v) => setVersionModal({ mode: "edit", version: v })}
          onRemove={removeVersion}
        />
      </section>

      <section>
        <div className="section-head">
          <h3>Compatible assets ({filtered.length}{categoryFilter ? ` of ${compatible.length}` : ""})</h3>
          <CategoryFilter categories={usedCategories} value={categoryFilter} onChange={setCategoryFilter} />
        </div>
        {compatible.length === 0 ? (
          <div className="empty muted small-empty">Assets you add for this model will appear here.</div>
        ) : filtered.length === 0 ? (
          <div className="empty muted small-empty">No compatible assets in that category.</div>
        ) : (
          <div className="grid">
            {filtered.map((a) => (
              <AssetCard
                key={a.id}
                root={root}
                asset={a}
                creator={creators.get(a.creatorId)}
                categoryName={categoryNames.get(a.categoryId) ?? "Unknown category"}
                siteName={siteNames.get(a.siteId) ?? "Unknown site"}
                onOpen={() => onOpenAsset(a.id)}
                onOpenCreator={() => onOpenCreator(a.creatorId)}
              />
            ))}
          </div>
        )}
      </section>

      {editOpen && (
        <ModelFormModal
          root={root}
          library={library}
          model={model}
          onLibraryChange={onLibraryChange}
          onDone={(lib) => { onLibraryChange(lib); setEditOpen(false); }}
          onClose={() => setEditOpen(false)}
        />
      )}

      {versionModal && (
        <VersionModal
          root={root}
          owner={{ kind: "model", id: model.id, name: model.name }}
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
