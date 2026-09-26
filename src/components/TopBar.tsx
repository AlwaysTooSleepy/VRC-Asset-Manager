import { useState } from "react";

type BrowseMode = "models" | "assets" | "creators";
type SortMode = "creator" | "name";

interface Props {
  search: string;
  onSearch: (value: string) => void;
  searchPlaceholder: string;
  searchDisabled?: boolean;
  onHome: () => void;
  onRefresh: () => void;
  onOpenSettings: () => void;
  onAddItem: () => void;
  onClearSearch: () => void;
  filterOpen: boolean;
  onFilterOpenChange: (open: boolean) => void;
  browseMode: BrowseMode;
  browseSort: SortMode;
  onBrowseModeChange: (mode: BrowseMode) => void;
  onBrowseSortChange: (sort: SortMode) => void;
}

const modeLabels: Record<BrowseMode, string> = {
  models: "Models",
  assets: "Assets",
  creators: "Creators",
};

const sortLabels: Record<SortMode, string> = {
  creator: "Creator name",
  name: "Product/item name",
};

// Top bar only, no sidebar.
export default function TopBar({
  search,
  onSearch,
  searchPlaceholder,
  searchDisabled,
  onHome,
  onRefresh,
  onOpenSettings,
  onAddItem,
  onClearSearch,
  filterOpen,
  onFilterOpenChange,
  browseMode,
  browseSort,
  onBrowseModeChange,
  onBrowseSortChange,
}: Props) {
  const [draftMode, setDraftMode] = useState<BrowseMode>(browseMode);
  const [draftSort, setDraftSort] = useState<SortMode>(browseSort);
  const [resetRequested, setResetRequested] = useState(false);

  const openFilterModal = () => {
    setDraftMode(browseMode);
    setDraftSort(browseSort);
    setResetRequested(false);
    onFilterOpenChange(true);
  };

  const applyFilter = () => {
    if (resetRequested) {
      onClearSearch();
      onBrowseModeChange("models");
      onBrowseSortChange("creator");
    } else {
      onBrowseModeChange(draftMode);
      onBrowseSortChange(draftSort);
    }
    onFilterOpenChange(false);
  };

  const cancelFilter = () => {
    setDraftMode(browseMode);
    setDraftSort(browseSort);
    setResetRequested(false);
    onFilterOpenChange(false);
  };

  const resetFilterDraft = () => {
    setDraftMode("models");
    setDraftSort("creator");
    setResetRequested(true);
  };

  const filterSummary = browseMode === "creators"
    ? `Filter: ${modeLabels.creators}`
    : `Filter: ${modeLabels[browseMode]} / ${sortLabels[browseSort]}`;

  return (
    <>
      <header className="topbar">
        <button className="brand" onClick={onHome} title="Back to all items">VRChat Asset Manager</button>
        <input
          className="search"
          type="search"
          placeholder={searchPlaceholder}
          value={search}
          disabled={searchDisabled}
          onChange={(e) => onSearch(e.target.value)}
        />

        <button className="btn primary" onClick={onAddItem}>+ Add</button>
        <button className="btn" onClick={openFilterModal} title="Browse filters">{filterSummary}</button>
        <button className="btn icon" onClick={onRefresh} title="Rescan the library folder" aria-label="Refresh">↻</button>
        <button className="btn icon" onClick={onOpenSettings} title="Settings" aria-label="Settings">⚙</button>
      </header>

      {filterOpen && (
        <div className="overlay" onClick={cancelFilter}>
          <div className="modal small" role="dialog" aria-modal="true" aria-labelledby="filter-title" onClick={(e) => e.stopPropagation()}>
            <div className="modal-head">
              <h2 id="filter-title">Browse filters</h2>
              <button className="btn icon" onClick={cancelFilter} aria-label="Close filters">✕</button>
            </div>
            <div className="modal-body">
              <div className="panel">
                <div className="field">
                  <span>View</span>
                  <div className="list" aria-label="Browse view mode">
                    {(Object.keys(modeLabels) as BrowseMode[]).map((mode) => (
                      <button
                        key={mode}
                        type="button"
                        className={`list-row clickable ${draftMode === mode ? "selected" : ""}`}
                        onClick={() => {
                          setDraftMode(mode);
                          setResetRequested(false);
                        }}
                        style={{ justifyContent: "space-between", width: "100%", border: "none", background: draftMode === mode ? "rgba(124, 140, 255, 0.12)" : "transparent", font: "inherit", color: "var(--text)" }}
                      >
                        <span>{modeLabels[mode]}</span>
                        {draftMode === mode && <span aria-hidden="true">✓</span>}
                      </button>
                    ))}
                  </div>
                </div>

                {draftMode !== "creators" && (
                  <div className="field">
                    <span>Sort</span>
                    <div className="list" aria-label="Browse sort mode">
                      {(Object.keys(sortLabels) as SortMode[]).map((mode) => (
                        <button
                          key={mode}
                          type="button"
                          className={`list-row clickable ${draftSort === mode ? "selected" : ""}`}
                          onClick={() => {
                            setDraftSort(mode);
                            setResetRequested(false);
                          }}
                          style={{ justifyContent: "space-between", width: "100%", border: "none", background: draftSort === mode ? "rgba(124, 140, 255, 0.12)" : "transparent", font: "inherit", color: "var(--text)" }}
                        >
                          <span>{sortLabels[mode]}</span>
                          {draftSort === mode && <span aria-hidden="true">✓</span>}
                        </button>
                      ))}
                    </div>
                  </div>
                )}
              </div>
            </div>
            <div className="modal-footer" style={{ display: "flex", justifyContent: "space-between", alignItems: "center", gap: 8, padding: "0 24px 20px" }}>
              <button className="btn link" onClick={resetFilterDraft}>Reset/Clear filters</button>
              <div style={{ display: "flex", gap: 8 }}>
                <button className="btn" onClick={cancelFilter}>Cancel</button>
                <button className="btn primary" onClick={applyFilter}>Apply</button>
              </div>
            </div>
          </div>
        </div>
      )}
    </>
  );
}
