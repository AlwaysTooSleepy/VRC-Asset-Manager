import type { Model } from "../types";
import CreatorLink from "./CreatorLink";
import ImageCarousel from "./ImageCarousel";

interface Props {
  root: string;
  model: Model;
  creator: { name: string; iconImagePath?: string | null } | undefined;
  siteName: string;
  categoryName: string;
  assetCount: number;
  onOpen: () => void;
  onOpenCreator?: () => void;
}

export default function ModelCard({ root, model, creator, siteName, categoryName, assetCount, onOpen, onOpenCreator }: Props) {
  return (
    <article
      className="tile"
      role="button"
      tabIndex={0}
      onClick={onOpen}
      onKeyDown={(e) => e.key === "Enter" && onOpen()}
    >
      <ImageCarousel root={root} images={model.imagePaths} name={model.name} thumb />
      <div className="tile-body">
        <h3 className="tile-title" title={model.name}>{model.name}</h3>
        {onOpenCreator && (
          <div className="tile-creator" onClick={(e) => e.stopPropagation()}>
            <CreatorLink root={root} creator={creator} onOpen={onOpenCreator} size={18} />
          </div>
        )}
        <div className="tile-meta">
          <div className="meta-row">
            <span className="meta-label">Site</span>
            <span className="meta-value">{siteName}</span>
          </div>
          <div className="meta-row">
            <span className="meta-label">Context</span>
            <span className="meta-value">{categoryName}</span>
          </div>
          <div className="meta-row">
            <span className="meta-label">Uses</span>
            <span className="meta-value">{assetCount} asset{assetCount === 1 ? "" : "s"}</span>
          </div>
          {model.missing && <div className="meta-row"><span className="meta-status danger">missing</span></div>}
        </div>
      </div>
    </article>
  );
}
