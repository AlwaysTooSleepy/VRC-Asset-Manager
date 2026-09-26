import { useState } from "react";
import { api } from "../api";
import type { SourceInput } from "../types";

/** A model an asset's files can be aimed at. */
export interface TargetOption {
  id: string;
  name: string;
  label: string;
}

/** A file or folder queued for filing. `target` is "none", "shared" or "model:<id>". */
export interface SourceItem {
  path: string;
  kind: "file" | "folder";
  target: string;
}

export const toSourceInputs = (items: SourceItem[]): SourceInput[] =>
  items.map((i) =>
    i.target === "none" || i.target === "shared"
      ? { path: i.path, target: i.target, modelId: null }
      : { path: i.path, target: "model", modelId: i.target.slice("model:".length) }
  );

const norm = (s: string) => s.toLowerCase().replace(/[^a-z0-9]+/g, " ").trim();
const baseName = (p: string) => p.split(/[\\/]/).filter(Boolean).pop() ?? p;

/** Best guess of a source's target from its file or folder name (longest model name wins). */
function guessTarget(path: string, options: TargetOption[]): string | null {
  const name = ` ${norm(baseName(path))} `;
  const hit = [...options]
    .sort((a, b) => b.name.length - a.name.length)
    .find((o) => norm(o.name) && name.includes(` ${norm(o.name)} `));
  if (hit) return `model:${hit.id}`;
  if (/\b(material|materials|psd|shared|common)\b/.test(name)) return "shared";
  return null;
}

interface Props {
  sources: SourceItem[];
  onChange: (sources: SourceItem[]) => void;
  targetOptions: TargetOption[];
  disabled?: boolean;
}

/** Attach several files/folders to one asset version, each aimed at a model or Shared. */
export default function SourcePicker({ sources, onChange, targetOptions, disabled }: Props) {
  const [error, setError] = useState<string | null>(null);
  const sorted = [...targetOptions].sort((a, b) => a.label.localeCompare(b.label));

  const add = async (kind: "file" | "folder") => {
    setError(null);
    try {
      const picked = kind === "folder" ? await api.pickFolders() : await api.pickFiles();
      const fresh = picked.filter((p) => !sources.some((s) => s.path === p));
      // Follow the layout already in use: once sources have targets, new ones start as Shared.
      const target = sources.length > 0 && sources.every((s) => s.target !== "none") ? "shared" : "none";
      onChange([...sources, ...fresh.map((path) => ({ path, kind, target }))]);
    } catch (e) {
      setError(String(e));
    }
  };

  const setTarget = (i: number, target: string) =>
    onChange(sources.map((s, idx) => (idx === i ? { ...s, target } : s)));

  const matchByName = () =>
    onChange(sources.map((s) => (s.target === "none" ? { ...s, target: guessTarget(s.path, targetOptions) ?? s.target } : s)));

  return (
    <div className="panel">
      {sources.length > 0 && (
        <ul className="list">
          {sources.map((s, i) => (
            <li key={s.path} className="list-row source-row">
              <span className="pill small-pill">{s.kind}</span>
              <span className="grow" title={s.path}>{baseName(s.path)}</span>
              <select className="input select-native" value={s.target} disabled={disabled} onChange={(e) => setTarget(i, e.target.value)}>
                <option value="none">None (straight into the version folder)</option>
                <option value="shared">Shared (not model-specific)</option>
                {sorted.length > 0 && (
                  <optgroup label="For a specific model">
                    {sorted.map((o) => <option key={o.id} value={`model:${o.id}`}>{o.label}</option>)}
                  </optgroup>
                )}
              </select>
              <button className="btn icon" aria-label="Remove" disabled={disabled} onClick={() => onChange(sources.filter((_, idx) => idx !== i))}>✕</button>
            </li>
          ))}
        </ul>
      )}

      <div className="row">
        <button className="btn" onClick={() => add("folder")} disabled={disabled}>+ Add folders…</button>
        <button className="btn" onClick={() => add("file")} disabled={disabled}>+ Add files…</button>
        {sources.length >= 2 && sources.some((s) => s.target === "none") && targetOptions.length > 0 && (
          <button className="btn" onClick={matchByName} disabled={disabled} title="Aim each source at the model named in its file name">Match models by name</button>
        )}
      </div>

      <p className="muted small">
        Files and folders are moved (not copied) into your library. Give every source a target to keep each model's files
        in its own <code>For_&lt;Model&gt;</code> subfolder, or leave them all as “None” to file everything flat.
      </p>
      {error && <div className="error pre">{error}</div>}
    </div>
  );
}
