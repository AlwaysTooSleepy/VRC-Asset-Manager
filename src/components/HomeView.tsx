import { useMemo } from "react";
import { assetCountsByModel, MODEL_CATEGORY_NAME } from "../library";
import type { Library } from "../types";
import AssetCard from "./AssetCard";
import Avatar from "./Avatar";
import ModelCard from "./ModelCard";

type ViewMode = "models" | "assets" | "creators";
type SortMode = "creator" | "name";

interface Props {
  root: string;
  library: Library;
  search: string;
  viewMode: ViewMode;
  sortMode: SortMode;
  onOpenModel: (id: string) => void;
  onOpenAsset: (id: string) => void;
  onOpenCreator: (id: string) => void;
}

const compareStrings = (a: string, b: string) => a.localeCompare(b, undefined, { sensitivity: "base" });

export default function HomeView({ root, library, search, viewMode, sortMode, onOpenModel, onOpenAsset, onOpenCreator }: Props) {

  const counts = useMemo(() => assetCountsByModel(library), [library]);
  const creators = useMemo(() => new Map(library.creators.map((c) => [c.id, c])), [library.creators]);
  const siteNames = useMemo(() => new Map(library.sites.map((s) => [s.id, s.name])), [library.sites]);
  const categoryNames = useMemo(() => new Map(library.categories.map((c) => [c.id, c.name])), [library.categories]);

  const q = search.trim().toLowerCase();

  const modelEntries = useMemo(
    () =>
      library.models
        .map((m) => ({
          id: m.id,
          name: m.name,
          creatorName: creators.get(m.creatorId)?.name ?? "Unknown creator",
          creatorId: m.creatorId,
          siteName: siteNames.get(m.siteId) ?? "Unknown site",
          categoryName: MODEL_CATEGORY_NAME,
          model: m,
          assetCount: counts[m.id] ?? 0,
        }))
        .filter((entry) => {
          if (!q) return true;
          return [entry.name, entry.creatorName, entry.siteName].some((value) => value.toLowerCase().includes(q));
        }),
    [counts, creators, library.models, q, siteNames]
  );

  const assetEntries = useMemo(
    () =>
      library.assets
        .map((a) => ({
          id: a.id,
          name: a.name,
          creatorName: creators.get(a.creatorId)?.name ?? "Unknown creator",
          creatorId: a.creatorId,
          siteName: siteNames.get(a.siteId) ?? "Unknown site",
          categoryName: categoryNames.get(a.categoryId) ?? "Unknown category",
          asset: a,
        }))
        .filter((entry) => {
          if (!q) return true;
          return [entry.name, entry.creatorName, entry.siteName, entry.categoryName].some((value) => value.toLowerCase().includes(q));
        }),
    [categoryNames, creators, library.assets, q, siteNames]
  );

  const creatorEntries = useMemo(
    () =>
      library.creators
        .map((creator) => ({
          id: creator.id,
          name: creator.name,
          modelCount: library.models.filter((m) => m.creatorId === creator.id).length,
          assetCount: library.assets.filter((a) => a.creatorId === creator.id).length,
          creator,
        }))
        .filter((entry) => {
          if (!q) return true;
          return [entry.name].some((value) => value.toLowerCase().includes(q));
        })
        .sort((a, b) => compareStrings(a.name, b.name) || a.id.localeCompare(b.id)),
    [library.assets, library.creators, library.models, q]
  );

  const sortedModels = useMemo(() => {
    const result = [...modelEntries];
    return result.sort((a, b) => {
      if (sortMode === "name") {
        return compareStrings(a.name, b.name) || compareStrings(a.creatorName, b.creatorName) || a.id.localeCompare(b.id);
      }
      return compareStrings(a.creatorName, b.creatorName) || compareStrings(a.name, b.name) || a.id.localeCompare(b.id);
    });
  }, [modelEntries, sortMode]);

  const sortedAssets = useMemo(() => {
    const result = [...assetEntries];
    return result.sort((a, b) => {
      if (sortMode === "name") {
        return compareStrings(a.name, b.name) || compareStrings(a.creatorName, b.creatorName) || a.id.localeCompare(b.id);
      }
      return compareStrings(a.creatorName, b.creatorName) || compareStrings(a.name, b.name) || a.id.localeCompare(b.id);
    });
  }, [assetEntries, sortMode]);

  const sortedCreators = useMemo(
    () => [...creatorEntries].sort((a, b) => compareStrings(a.name, b.name) || a.id.localeCompare(b.id)),
    [creatorEntries]
  );

  const emptyState =
    viewMode === "models"
      ? "No models match your search."
      : viewMode === "assets"
        ? "No assets match your search."
        : "No creators match your search.";

  return (
    <div className="panel">
      {viewMode === "models" && (
        sortedModels.length === 0 ? (
          <div className="empty muted">{emptyState}</div>
        ) : (
          <div className="grid">
            {sortedModels.map((entry) => (
              <ModelCard
                key={entry.id}
                root={root}
                model={entry.model}
                creator={creators.get(entry.creatorId)}
                siteName={entry.siteName}
                categoryName={MODEL_CATEGORY_NAME}
                assetCount={entry.assetCount}
                onOpen={() => onOpenModel(entry.id)}
                onOpenCreator={() => onOpenCreator(entry.creatorId)}
              />
            ))}
          </div>
        )
      )}

      {viewMode === "assets" && (
        sortedAssets.length === 0 ? (
          <div className="empty muted">{emptyState}</div>
        ) : (
          <div className="grid">
            {sortedAssets.map((entry) => (
              <AssetCard
                key={entry.id}
                root={root}
                asset={entry.asset}
                creator={creators.get(entry.creatorId)}
                categoryName={entry.categoryName}
                siteName={entry.siteName}
                onOpen={() => onOpenAsset(entry.id)}
                onOpenCreator={() => onOpenCreator(entry.creatorId)}
              />
            ))}
          </div>
        )
      )}

      {viewMode === "creators" && (
        sortedCreators.length === 0 ? (
          <div className="empty muted">{emptyState}</div>
        ) : (
          <div className="grid">
            {sortedCreators.map(({ id, name, modelCount, assetCount, creator }) => (
              <article key={id} className="tile" role="button" tabIndex={0} onClick={() => onOpenCreator(id)} onKeyDown={(e) => e.key === "Enter" && onOpenCreator(id)}>
                <div className="tile-body">
                  <div className="creator-card-head">
                    <Avatar root={root} creator={creator} size={42} />
                    <h3 className="tile-title" title={name}>{name}</h3>
                  </div>
                  <div className="tile-meta">
                    <div className="meta-row">
                      <span className="meta-label">Models</span>
                      <span className="meta-value">{modelCount}</span>
                    </div>
                    <div className="meta-row">
                      <span className="meta-label">Assets</span>
                      <span className="meta-value">{assetCount}</span>
                    </div>
                  </div>
                </div>
              </article>
            ))}
          </div>
        )
      )}
    </div>
  );
}
