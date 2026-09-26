// Pure helpers for reading the library. No backend calls here.

import type { Asset, Library, Model } from "./types";

/** Assets that work with a model: listed for it explicitly, or marked compatible with all. */
export const assetsForModel = (library: Library, modelId: string): Asset[] =>
  library.assets.filter((a) => a.compatibleWithAll || a.compatibleModelIds.includes(modelId));

/** Models an asset works with (all of them when it is marked compatible with all). */
export const modelsForAsset = (library: Library, asset: Asset): Model[] =>
  asset.compatibleWithAll
    ? [...library.models]
    : library.models.filter((m) => asset.compatibleModelIds.includes(m.id));

/** Compatible-asset count per model id, computed in one pass over the assets. */
export function assetCountsByModel(library: Library): Record<string, number> {
  const counts: Record<string, number> = {};
  for (const m of library.models) counts[m.id] = 0;
  for (const a of library.assets) {
    if (a.compatibleWithAll) {
      for (const m of library.models) counts[m.id] += 1;
    } else {
      for (const id of a.compatibleModelIds) if (id in counts) counts[id] += 1;
    }
  }
  return counts;
}

export const formatDate = (iso: string) =>
  new Date(iso).toLocaleDateString(undefined, { year: "numeric", month: "short", day: "numeric" });

export const formatDateTime = (iso: string) =>
  new Date(iso).toLocaleString(undefined, { dateStyle: "medium", timeStyle: "short" });

export const MODEL_CATEGORY_NAME = "Models";

export const itemCategoryName = (library: Library, kind: "model" | "asset", categoryId?: string) => {
  if (kind === "model") return MODEL_CATEGORY_NAME;
  return library.categories.find((c) => c.id === categoryId)?.name ?? "Unknown category";
};

/** "1.10.0" > "1.9.0": compares the digit runs numerically (mirrors sync.rs). */
const versionKey = (label: string) =>
  label.split(/[^0-9]+/).filter(Boolean).map((n) => Number(n) || 0);

/** Sort order for version lists: highest version first. */
export function compareVersionsDesc(a: { versionLabel: string; dateAdded: string }, b: { versionLabel: string; dateAdded: string }) {
  const ka = versionKey(a.versionLabel);
  const kb = versionKey(b.versionLabel);
  for (let i = 0; i < Math.max(ka.length, kb.length); i++) {
    const diff = (kb[i] ?? -1) - (ka[i] ?? -1);
    if (diff !== 0) return diff;
  }
  return b.dateAdded.localeCompare(a.dateAdded);
}
