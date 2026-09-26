import { useCallback, useEffect, useState } from "react";
import { api } from "./api";
import type { Library, Notice } from "./types";
import TopBar from "./components/TopBar";
import SetupScreen from "./components/SetupScreen";
import SettingsModal from "./components/settings/SettingsModal";
import HomeView from "./components/HomeView";
import UnifiedAddModal from "./components/UnifiedAddModal";
import ModelDetail from "./components/ModelDetail";
import AssetDetail from "./components/AssetDetail";
import CreatorDetail from "./components/CreatorDetail";

const plural = (n: number, word: string) => `${n} ${word}${n === 1 ? "" : "s"}`;

type View = { kind: "home" } | { kind: "model"; id: string } | { kind: "asset"; id: string } | { kind: "creator"; id: string };
const HOME: View = { kind: "home" };

export default function App() {
  const [loading, setLoading] = useState(true);
  const [root, setRoot] = useState<string | null>(null);
  const [library, setLibrary] = useState<Library | null>(null);
  const [search, setSearch] = useState("");
  const [browseMode, setBrowseMode] = useState<"models" | "assets" | "creators">("models");
  const [browseSort, setBrowseSort] = useState<"creator" | "name">("creator");
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [filterOpen, setFilterOpen] = useState(false);
  const [addUnifiedOpen, setAddUnifiedOpen] = useState(false);
  // Pages you've drilled through, so Back returns to where you came from.
  const [stack, setStack] = useState<View[]>([HOME]);
  const [notice, setNotice] = useState<Notice | null>(null);
  const [error, setError] = useState<string | null>(null);
  const view = stack[stack.length - 1];
  const anyModalOpen = settingsOpen || filterOpen || addUnifiedOpen;

  useEffect(() => {
    if (!anyModalOpen) {
      return;
    }

    const content = document.querySelector<HTMLElement>(".content");
    const previousContentOverflow = content?.style.overflow ?? "";
    if (content) content.style.overflow = "hidden";

    const onKeyDown = (event: KeyboardEvent) => {
      const target = event.target as HTMLElement | null;
      if (target?.closest(".modal, .modal-body, .select-menu, .settings-list")) {
        return;
      }

      if (["ArrowDown", "ArrowUp", "PageDown", "PageUp", "Space"].includes(event.key)) {
        event.preventDefault();
      }
    };

    document.addEventListener("keydown", onKeyDown);

    return () => {
      document.removeEventListener("keydown", onKeyDown);
      if (content) content.style.overflow = previousContentOverflow;
    };
  }, [anyModalOpen]);

  // Compare the library with the folders on disk and adopt the result.
  const refresh = useCallback(async (showErrors = false) => {
    try {
      setLibrary(await api.syncLibrary());
    } catch (e) {
      if (showErrors) setError(String(e));
    }
  }, []);

  useEffect(() => {
    (async () => {
      try {
        const savedRoot = await api.getLibraryRoot();
        if (savedRoot) {
          setRoot(savedRoot);
          setLibrary(await api.syncLibrary());
        }
      } catch (e) {
        setError(String(e));
      } finally {
        setLoading(false);
      }
    })();
  }, []);

  // Coming back to the window after moving or renaming folders picks the changes up.
  useEffect(() => {
    if (!root) return;
    const onFocus = () => { refresh(); };
    window.addEventListener("focus", onFocus);
    return () => window.removeEventListener("focus", onFocus);
  }, [root, refresh]);

  const go = (next: View) => { setStack((s) => [...s, next]); refresh(); };
  const back = () => setStack((s) => (s.length > 1 ? s.slice(0, -1) : s));
  const home = () => setStack([HOME]);

  const chooseRoot = async () => {
    setError(null);
    try {
      const folder = await api.pickFolder("Choose your library folder");
      if (!folder) return;
      const lib = await api.setLibraryRoot(folder);
      setLibrary(lib);
      setRoot(folder);
      home();
      const found = lib.models.length + lib.assets.length;
      setNotice({
        text: found
          ? `Library loaded: found ${plural(lib.models.length, "model")}, ${plural(lib.assets.length, "asset")} and ${plural(lib.creators.length, "creator")}.`
          : "New library created.",
        warning: null,
      });
    } catch (e) {
      setError(String(e));
    }
  };

  if (loading) return <div className="center muted">Loading…</div>;

  if (!root || !library) {
    return <SetupScreen onChoose={chooseRoot} error={error} />;
  }

  return (
    <div className="app">
      <TopBar
        search={search}
        onSearch={setSearch}
        searchPlaceholder={view.kind === "home" ? "Search items, creators, sites…" : "Search is available on the home page"}
        searchDisabled={view.kind !== "home"}
        onHome={home}
        onRefresh={() => refresh(true)}
        onOpenSettings={() => setSettingsOpen(true)}
        onAddItem={() => { setNotice(null); setAddUnifiedOpen(true); }}
        onClearSearch={() => setSearch("")}
        filterOpen={filterOpen}
        onFilterOpenChange={setFilterOpen}
        browseMode={browseMode}
        browseSort={browseSort}
        onBrowseModeChange={setBrowseMode}
        onBrowseSortChange={setBrowseSort}
      />
      <main className="content">
        {error && <div className="error">{error}</div>}
        {notice && (
          <div className="notice" onClick={() => setNotice(null)} title="Click to dismiss">
            <div>{notice.text}</div>
            {notice.warning && <div className="warn">{notice.warning}</div>}
          </div>
        )}
        {view.kind === "home" && (
          <HomeView
            root={root}
            library={library}
            search={search}
            viewMode={browseMode}
            sortMode={browseSort}
            onOpenModel={(id) => go({ kind: "model", id })}
            onOpenAsset={(id) => go({ kind: "asset", id })}
            onOpenCreator={(id) => go({ kind: "creator", id })}
          />
        )}
        {view.kind === "model" && (
          <ModelDetail
            root={root}
            library={library}
            modelId={view.id}
            onLibraryChange={setLibrary}
            onNotice={setNotice}
            onOpenAsset={(id) => go({ kind: "asset", id })}
            onOpenCreator={(id) => go({ kind: "creator", id })}
            onBack={back}
          />
        )}
        {view.kind === "asset" && (
          <AssetDetail
            root={root}
            library={library}
            assetId={view.id}
            onLibraryChange={setLibrary}
            onNotice={setNotice}
            onOpenModel={(id) => go({ kind: "model", id })}
            onOpenCreator={(id) => go({ kind: "creator", id })}
            onBack={back}
          />
        )}
        {view.kind === "creator" && (
          <CreatorDetail
            root={root}
            library={library}
            creatorId={view.id}
            onLibraryChange={setLibrary}
            onOpenModel={(id) => go({ kind: "model", id })}
            onOpenAsset={(id) => go({ kind: "asset", id })}
            onBack={back}
          />
        )}
      </main>

      {settingsOpen && (
        <SettingsModal
          root={root}
          library={library}
          onLibraryChange={setLibrary}
          onChangeRoot={chooseRoot}
          onOpenCreatorProfile={(id) => { setSettingsOpen(false); go({ kind: "creator", id }); }}
          onClose={() => setSettingsOpen(false)}
        />
      )}

      {addUnifiedOpen && (
        <UnifiedAddModal
          root={root}
          library={library}
          onLibraryChange={setLibrary}
          onDone={(lib, itemId, kind) => {
            setLibrary(lib);
            setAddUnifiedOpen(false);
            go({ kind: kind === "model" ? "model" : "asset", id: itemId });
            setNotice({
              text: kind === "model"
                ? "Model added. Use “Add version” below to create its folder and file its files."
                : "Asset added. Use “Add version” below to create its folder and file its files.",
              warning: null,
            });
          }}
          onClose={() => setAddUnifiedOpen(false)}
        />
      )}
    </div>
  );
}
