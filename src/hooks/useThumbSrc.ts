import { useEffect, useState } from "react";
import { imageUrl, thumbUrl } from "../api";

/**
 * <img> src for a library image: prefers the cached thumbnail, and falls
 * back to the full original if the thumbnail 404s — the only way the
 * webview can tell us "that file isn't actually there" is by failing to
 * load it. That happens for images imported before thumbnails existed, or
 * if generation failed for that particular file (see images.rs's
 * `generate_thumbnail`). Resets automatically whenever `rel` changes, so
 * switching to a different image gives its own thumbnail a fresh try.
 */
export function useThumbSrc(root: string, rel: string | null | undefined) {
  const [failed, setFailed] = useState(false);
  useEffect(() => setFailed(false), [rel]);

  if (!rel) return { src: undefined, onError: undefined };
  return {
    src: failed ? imageUrl(root, rel) : thumbUrl(root, rel),
    onError: failed ? undefined : () => setFailed(true),
  };
}
