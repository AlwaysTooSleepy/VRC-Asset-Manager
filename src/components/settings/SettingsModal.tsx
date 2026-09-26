import { useEffect, useMemo, useState } from "react";
import type { Library } from "../../types";
import LibraryPanel from "./LibraryPanel";
import ReferenceListPanel from "./ReferenceListPanel";
import CreatorsPanel from "./CreatorsPanel";
import { api } from "../../api";

type Tab = "library" | "sites" | "creators" | "categories";

const TABS: { id: Tab; label: string }[] = [
  { id: "library", label: "Library" },
  { id: "sites", label: "Sites" },
  { id: "creators", label: "Creators" },
  { id: "categories", label: "Categories" },
];

interface Props {
  root: string;
  library: Library;
  onLibraryChange: (library: Library) => void;
  onChangeRoot: () => void;
  onOpenCreatorProfile: (id: string) => void;
  onClose: () => void;
}

type Counts = Record<string, number>;
const bump = (map: Counts, key: string) => {
  map[key] = (map[key] ?? 0) + 1;
};

export default function SettingsModal({ root, library, onLibraryChange, onChangeRoot, onOpenCreatorProfile, onClose }: Props) {
  const [tab, setTab] = useState<Tab>("library");

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => e.key === "Escape" && onClose();
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onClose]);

  // How many items use each Site / Creator / Category (blocks deleting in-use ones).
  const usage = useMemo(() => {
    const sites: Counts = {};
    const creators: Counts = {};
    const categories: Counts = {};
    for (const m of library.models) {
      bump(sites, m.siteId);
      bump(creators, m.creatorId);
    }
    for (const a of library.assets) {
      bump(sites, a.siteId);
      bump(creators, a.creatorId);
      bump(categories, a.categoryId);
    }
    return { sites, creators, categories };
  }, [library]);

  return (
    <div
      className="overlay"
      onMouseDown={(e) => e.target === e.currentTarget && onClose()}
    >
      <div className="modal" role="dialog" aria-label="Settings">
        <div className="modal-head">
          <h2>Settings</h2>
          <button className="btn icon" onClick={onClose} aria-label="Close">✕</button>
        </div>

        <nav className="tabs">
          {TABS.map((t) => (
            <button
              key={t.id}
              className={`tab ${tab === t.id ? "active" : ""}`}
              onClick={() => setTab(t.id)}
            >
              {t.label}
            </button>
          ))}
        </nav>

        <div className="modal-body">
          {tab === "library" && (
            <LibraryPanel
              root={root}
              library={library}
              onLibraryChange={onLibraryChange}
              onChangeRoot={onChangeRoot}
            />
          )}

          {tab === "sites" && (
            <ReferenceListPanel
              kind="site"
              items={library.sites}
              usage={usage.sites}
              onAdd={async (name) => onLibraryChange(await api.addSite(name))}
              onRename={async (id, name) => onLibraryChange(await api.renameSite(id, name))}
              onDelete={async (id) => onLibraryChange(await api.deleteSite(id))}
            />
          )}

          {tab === "categories" && (
            <ReferenceListPanel
              kind="category"
              hint="Category names are folder names. Renaming one also renames its folder under Assets."
              items={library.categories}
              usage={usage.categories}
              onAdd={async (name) => onLibraryChange(await api.addCategory(name))}
              onRename={async (id, name) => onLibraryChange(await api.renameCategory(id, name))}
              onDelete={async (id) => onLibraryChange(await api.deleteCategory(id))}
            />
          )}

          {tab === "creators" && (
            <CreatorsPanel
              root={root}
              library={library}
              usage={usage.creators}
              onLibraryChange={onLibraryChange}
              onOpenProfile={onOpenCreatorProfile}
            />
          )}
        </div>
      </div>
    </div>
  );
}
