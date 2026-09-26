import { useEffect, useRef, useState } from "react";
import { api } from "../api";
import { useThumbSrc } from "../hooks/useThumbSrc";

interface Props {
  root: string;
  images: string[];
  onChange: (images: string[]) => void;
  disabled?: boolean;
}

/** One thumbnail tile. Its own component so `useThumbSrc` (a hook) can be
 * called once per image rather than inside the list's `.map()`. */
function Thumb({ root, rel, onRemove, disabled }: { root: string; rel: string; onRemove: () => void; disabled?: boolean }) {
  const { src, onError } = useThumbSrc(root, rel);
  return (
    <div className="thumb">
      <img src={src} alt="" onError={onError} />
      <button className="thumb-x" aria-label="Remove image" onClick={onRemove} disabled={disabled}>✕</button>
    </div>
  );
}

/** Image list with thumbnails. Add from files on disk or from web addresses. */
export default function ImagePicker({ root, images, onChange, disabled }: Props) {
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [urlOpen, setUrlOpen] = useState(false);
  const [urlText, setUrlText] = useState("");

  const latest = useRef(images);
  latest.current = images;

  // Images imported during this session. Any that never get saved are removed on exit
  // (the backend refuses to delete an image that something still references).
  const imported = useRef<string[]>([]);
  useEffect(() => () => { imported.current.forEach((p) => api.removeImageIfUnused(p).catch(() => {})); }, []);

  const track = (rel: string) => {
    imported.current.push(rel);
  };

  const addFiles = async () => {
    setBusy(true);
    setError(null);
    try {
      let next = [...latest.current];
      for (const file of await api.pickImages()) {
        const rel = await api.importImage(file);
        track(rel);
        next = [...next, rel];
        onChange(next);
      }
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  const addUrls = async () => {
    const urls = urlText.split(/\s+/).map((u) => u.trim()).filter(Boolean);
    if (urls.length === 0) return;
    setBusy(true);
    setError(null);
    let next = [...latest.current];
    const failed: { url: string; reason: string }[] = [];
    for (const url of urls) {
      try {
        const rel = await api.importImageFromUrl(url);
        track(rel);
        next = [...next, rel];
        onChange(next);
      } catch (e) {
        failed.push({ url, reason: String(e) });
      }
    }
    setUrlText(failed.map((f) => f.url).join("\n")); // keep only the ones that failed
    if (failed.length) setError(failed.map((f) => f.reason).join("\n"));
    else setUrlOpen(false);
    setBusy(false);
  };

  const remove = (rel: string) => {
    onChange(images.filter((p) => p !== rel));
    api.removeImageIfUnused(rel).catch(() => {}); // no-op while a saved item still uses it
  };

  return (
    <div className="field">
      <span>Images (the first one is the card cover)</span>
      <div className="thumbs">
        {images.map((rel) => (
          <Thumb key={rel} root={root} rel={rel} onRemove={() => remove(rel)} disabled={disabled || busy} />
        ))}
        <button className="thumb add" onClick={addFiles} disabled={disabled || busy}>+ From files</button>
        <button className="thumb add" onClick={() => setUrlOpen((o) => !o)} disabled={disabled || busy}>+ From URL</button>
      </div>

      {urlOpen && (
        <div className="panel">
          <textarea
            className="input textarea"
            rows={3}
            placeholder={"https://…/image.png\n(one address per line)"}
            value={urlText}
            onChange={(e) => setUrlText(e.target.value)}
            disabled={disabled || busy}
          />
          <div className="row">
            <button className="btn primary" onClick={addUrls} disabled={disabled || busy || !urlText.trim()}>
              {busy ? "Downloading…" : "Download"}
            </button>
          </div>
        </div>
      )}

      {error && <div className="error pre">{error}</div>}
    </div>
  );
}
