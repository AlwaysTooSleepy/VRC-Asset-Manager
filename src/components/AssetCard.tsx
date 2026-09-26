import type { Asset } from "../types";
import CreatorLink from "./CreatorLink";
import ImageCarousel from "./ImageCarousel";

interface Props {
  root: string;
  asset: Asset;
  creator: { name: string; iconImagePath?: string | null } | undefined;
  categoryName: string;
  siteName: string;
  onOpen: () => void;
  onOpenCreator?: () => void;
}

export default function AssetCard({ root, asset, creator, categoryName, siteName, onOpen, onOpenCreator }: Props) {
  return (
    <article
      className="tile"
      role="button"
      tabIndex={0}
      onClick={onOpen}
      onKeyDown={(e) => e.key === "Enter" && onOpen()}
    >
      <ImageCarousel root={root} images={asset.imagePaths} name={asset.name} thumb />
      <div className="tile-body">
        <h3 className="tile-title" title={asset.name}>{asset.name}</h3>
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
          {asset.missing && <div className="meta-row"><span className="meta-status danger">missing</span></div>}
        </div>
      </div>
    </article>
  );
}
