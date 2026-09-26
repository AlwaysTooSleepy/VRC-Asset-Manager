import { useEffect, useMemo, useState } from "react";
import type { Library } from "../types";
import ImageCarousel from "./ImageCarousel";

type SortMode = "creator" | "name";

interface Props {
  root: string;
  library: Library;
  selectedIds: string[];
  allModels: boolean;
  onChange: (selectedIds: string[], allModels: boolean) => void;
  disabled?: boolean;
}

const compareStrings = (a: string, b: string) => a.localeCompare(b, undefined, { sensitivity: "base" });

/** Model compatibility picker: a dedicated modal with draft state and apply/cancel semantics. */
export default function ModelPicker({ root, library, selectedIds, allModels, onChange, disabled }: Props) {
  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState("");
  const [creatorFilter, setCreatorFilter] = useState<string>("all");
  const [sortMode, setSortMode] = useState<SortMode>("creator");
  const [draftSelectedIds, setDraftSelectedIds] = useState<string[]>(selectedIds);
  const [draftAllModels, setDraftAllModels] = useState(allModels);

  const creators = useMemo(() => new Map(library.creators.map((c) => [c.id, c.name])), [library.creators]);
  const q = query.trim().toLowerCase();

  useEffect(() => {
    if (!open) return;
    setDraftSelectedIds(selectedIds);
    setDraftAllModels(allModels);
  }, [open, selectedIds, allModels]);

  const matches = useMemo(() => {
    return [...library.models]
      .filter((m) => {
        const creatorName = creators.get(m.creatorId) ?? "Unknown creator";
        const matchesSearch = !q || m.name.toLowerCase().includes(q) || creatorName.toLowerCase().includes(q);
        const matchesCreator = creatorFilter === "all" || m.creatorId === creatorFilter;
        return matchesSearch && matchesCreator;
      })
      .sort((a, b) => {
        const aCreator = creators.get(a.creatorId) ?? "Unknown creator";
        const bCreator = creators.get(b.creatorId) ?? "Unknown creator";
        if (sortMode === "name") {
          return compareStrings(a.name, b.name) || compareStrings(aCreator, bCreator) || a.id.localeCompare(b.id);
        }
        return compareStrings(aCreator, bCreator) || compareStrings(a.name, b.name) || a.id.localeCompare(b.id);
      });
  }, [creatorFilter, creators, library.models, q, sortMode]);

  const openPicker = () => {
    setDraftSelectedIds(selectedIds);
    setDraftAllModels(allModels);
    setOpen(true);
  };

  const closePicker = () => {
    setOpen(false);
    setDraftSelectedIds(selectedIds);
    setDraftAllModels(allModels);
  };

  const applyPicker = () => {
    onChange(draftSelectedIds, draftAllModels);
    setOpen(false);
  };

  const toggleAll = (checked: boolean) => {
    setDraftAllModels(checked);
    if (checked) setDraftSelectedIds([]);
  };

  const toggleOne = (id: string) => {
    if (draftAllModels) {
      setDraftAllModels(false);
      setDraftSelectedIds([id]);
      return;
    }
    setDraftSelectedIds((prev) =>
      prev.includes(id) ? prev.filter((x) => x !== id) : [...prev, id]
    );
  };

  return (
    <div className="field">
      <span>Choose supported models</span>

      <button type="button" className="btn" onClick={() => !disabled && openPicker()} disabled={disabled}>
        Choose supported models
      </button>

      {open && (
        <div className="overlay" onMouseDown={(e) => e.target === e.currentTarget && closePicker()}>
          <div className="modal" role="dialog" aria-modal="true" aria-label="Choose supported models" style={{ width: "min(1120px, 96vw)", maxHeight: "88vh" }}>
            <div className="modal-head">
              <h2>Choose supported models</h2>
              <button className="btn icon" onClick={closePicker} aria-label="Close model picker">✕</button>
            </div>

            <div className="modal-body">
              <div className="panel">
                <label className="check" style={{ display: "flex", alignItems: "center", gap: 8 }}>
                  <input
                    type="checkbox"
                    checked={draftAllModels}
                    onChange={(e) => toggleAll(e.target.checked)}
                    disabled={disabled}
                  />
                  <span>Select all models</span>
                </label>

                {!draftAllModels && (
                  <div className="row" style={{ alignItems: "end", gap: 12, flexWrap: "wrap" }}>
                    <label className="field" style={{ flex: 1, minWidth: 180 }}>
                      <span>Search</span>
                      <input className="input" type="search" placeholder="Search models or creators" value={query} onChange={(e) => setQuery(e.target.value)} disabled={disabled} />
                    </label>

                    <label className="field" style={{ minWidth: 180 }}>
                      <span>Creator</span>
                      <select className="input" value={creatorFilter} onChange={(e) => setCreatorFilter(e.target.value)} disabled={disabled}>
                        <option value="all">All creators</option>
                        {library.creators.map((creator) => (
                          <option key={creator.id} value={creator.id}>{creator.name}</option>
                        ))}
                      </select>
                    </label>

                    <label className="field" style={{ minWidth: 180 }}>
                      <span>Sort</span>
                      <select className="input" value={sortMode} onChange={(e) => setSortMode(e.target.value as SortMode)} disabled={disabled}>
                        <option value="creator">Creator name</option>
                        <option value="name">Model name</option>
                      </select>
                    </label>
                  </div>
                )}

                {draftAllModels ? (
                  <div className="card" style={{ padding: 16 }}>
                    <div className="row" style={{ justifyContent: "space-between", gap: 12, alignItems: "center" }}>
                      <div>
                        <strong>All models selected</strong>
                        <div className="muted small">This asset supports every model in the library.</div>
                      </div>
                    </div>
                  </div>
                ) : library.models.length === 0 ? (
                  <p className="muted small">You haven't added any models yet.</p>
                ) : matches.length === 0 ? (
                  <div className="muted small">No models match the current search and filters.</div>
                ) : (
                  <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fill, minmax(190px, 1fr))", gap: 14 }}>
                    {matches.map((m) => {
                      const selected = draftSelectedIds.includes(m.id);
                      return (
                        <button
                          type="button"
                          key={m.id}
                          className="tile"
                          style={{
                            padding: 0,
                            overflow: "hidden",
                            textAlign: "left",
                            cursor: disabled ? "default" : "pointer",
                            borderColor: selected ? "var(--accent)" : undefined,
                            boxShadow: selected ? "0 0 0 1px rgba(124, 140, 255, 0.45)" : undefined,
                          }}
                          onClick={() => !disabled && toggleOne(m.id)}
                        >
                          <div style={{ position: "relative" }}>
                            <ImageCarousel root={root} images={m.imagePaths} name={m.name} thumb />
                            {selected && (
                              <span
                                aria-label={selected ? "Selected model" : "Not selected model"}
                                style={{
                                  position: "absolute",
                                  right: 10,
                                  top: 10,
                                  width: 28,
                                  height: 28,
                                  borderRadius: "50%",
                                  background: "var(--accent)",
                                  color: "#0b0d14",
                                  display: "grid",
                                  placeItems: "center",
                                  fontWeight: 700,
                                  boxShadow: "0 6px 18px rgba(0,0,0,0.35)",
                                }}
                              >
                                ✓
                              </span>
                            )}
                          </div>
                          <div className="tile-body">
                            <h3 className="tile-title" title={m.name}>{m.name}</h3>
                            <div className="tile-creator">{creators.get(m.creatorId) ?? "Unknown creator"}</div>
                            <div className="tile-meta">
                              <div className="meta-row">
                                <span className="meta-label">Status</span>
                                <span className="meta-value">{selected ? "Selected" : "Not selected"}</span>
                              </div>
                            </div>
                          </div>
                        </button>
                      );
                    })}
                  </div>
                )}
              </div>
            </div>

            <div className="modal-footer" style={{ display: "flex", justifyContent: "space-between", gap: 8, padding: "0 24px 20px" }}>
              <div className="muted small">{draftAllModels ? "All models" : `${draftSelectedIds.length} selected`}</div>
              <div style={{ display: "flex", gap: 8 }}>
                <button className="btn" onClick={closePicker} disabled={disabled}>Cancel</button>
                <button className="btn primary" onClick={applyPicker} disabled={disabled || (!draftAllModels && draftSelectedIds.length === 0)}>
                  Apply
                </button>
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
