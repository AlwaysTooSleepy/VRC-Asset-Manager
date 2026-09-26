import { useMemo, useState } from "react";
import { api } from "../api";
import { assetCountsByModel, MODEL_CATEGORY_NAME } from "../library";
import type { Library } from "../types";
import AssetCard from "./AssetCard";
import Avatar from "./Avatar";
import CreatorForm from "./CreatorForm";
import ModelCard from "./ModelCard";

interface Props {
  root: string;
  library: Library;
  creatorId: string;
  onLibraryChange: (library: Library) => void;
  onOpenModel: (id: string) => void;
  onOpenAsset: (id: string) => void;
  onBack: () => void;
}

/** A creator's profile: their icon, links, and everything of theirs in the library. */
export default function CreatorDetail({ root, library, creatorId, onLibraryChange, onOpenModel, onOpenAsset, onBack }: Props) {
  const [editOpen, setEditOpen] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const creator = library.creators.find((c) => c.id === creatorId);

  const models = useMemo(
    () => library.models.filter((m) => m.creatorId === creatorId).sort((a, b) => b.dateUpdated.localeCompare(a.dateUpdated)),
    [library.models, creatorId]
  );
  const assets = useMemo(
    () => library.assets.filter((a) => a.creatorId === creatorId).sort((a, b) => b.dateUpdated.localeCompare(a.dateUpdated)),
    [library.assets, creatorId]
  );
  const counts = useMemo(() => assetCountsByModel(library), [library]);
  const siteNames = useMemo(() => new Map(library.sites.map((s) => [s.id, s.name])), [library.sites]);
  const categoryNames = useMemo(() => new Map(library.categories.map((c) => [c.id, c.name])), [library.categories]);

  if (!creator) {
    return (
      <div className="detail">
        <button className="btn link" onClick={onBack}>‹ Back</button>
        <p className="muted">This creator no longer exists.</p>
      </div>
    );
  }

  const openLink = (url: string) => api.openUrl(url).catch((e) => setError(String(e)));

  return (
    <div className="detail">
      <button className="btn link" onClick={onBack}>‹ Back</button>

      <div className="detail-top creator-top">
        <Avatar root={root} creator={creator} size={120} />
        <div className="detail-info">
          <h1>{creator.name}</h1>

          <div className="stats">
            <div><b>{models.length}</b><span className="muted small">model{models.length === 1 ? "" : "s"}</span></div>
            <div><b>{assets.length}</b><span className="muted small">asset{assets.length === 1 ? "" : "s"}</span></div>
          </div>

          {creator.profileUrls.length > 0 && (
            <div className="row link-row">
              {creator.profileUrls.map((url) => (
                <button key={url} className="btn" onClick={() => openLink(url)} title={url}>{url}</button>
              ))}
            </div>
          )}

          <div className="row">
            <button className="btn" onClick={() => setEditOpen(true)}>Edit</button>
          </div>

          {error && <div className="error pre">{error}</div>}
        </div>
      </div>

      <section>
        <h3>Models ({models.length})</h3>
        {models.length === 0 ? (
          <div className="empty muted small-empty">No models by this creator yet.</div>
        ) : (
          <div className="grid">
            {models.map((m) => (
              <ModelCard
                key={m.id}
                root={root}
                model={m}
                creator={creator}
                siteName={siteNames.get(m.siteId) ?? "Unknown site"}
                categoryName={MODEL_CATEGORY_NAME}
                assetCount={counts[m.id] ?? 0}
                onOpen={() => onOpenModel(m.id)}
              />
            ))}
          </div>
        )}
      </section>

      <section>
        <h3>Assets ({assets.length})</h3>
        {assets.length === 0 ? (
          <div className="empty muted small-empty">No assets by this creator yet.</div>
        ) : (
          <div className="grid">
            {assets.map((a) => (
              <AssetCard
                key={a.id}
                root={root}
                asset={a}
                creator={creator}
                categoryName={categoryNames.get(a.categoryId) ?? "Unknown category"}
                siteName={siteNames.get(a.siteId) ?? "Unknown site"}
                onOpen={() => onOpenAsset(a.id)}
              />
            ))}
          </div>
        )}
      </section>

      {editOpen && (
        <div className="overlay" onMouseDown={(e) => e.target === e.currentTarget && setEditOpen(false)}>
          <div className="modal small" role="dialog" aria-label="Edit creator">
            <div className="modal-head">
              <h2>Edit creator</h2>
              <button className="btn icon" onClick={() => setEditOpen(false)} aria-label="Close">✕</button>
            </div>
            <div className="modal-body">
              <CreatorForm
                root={root}
                creator={creator}
                onSaved={(lib) => { onLibraryChange(lib); setEditOpen(false); }}
                onCancel={() => setEditOpen(false)}
              />
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
