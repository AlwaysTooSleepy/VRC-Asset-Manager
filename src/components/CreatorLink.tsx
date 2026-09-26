import Avatar from "./Avatar";

interface Props {
  root: string;
  creator: { name: string; iconImagePath?: string | null } | undefined;
  onOpen: () => void;
  size?: number;
  showIcon?: boolean;
}

/** Clickable creator name (with icon), used on cards and detail pages alike. */
export default function CreatorLink({ root, creator, onOpen, size = 40, showIcon = true }: Props) {
  if (!creator) return <span className="muted">Unknown creator</span>;
  return (
    <button className="creator-link" onClick={onOpen} title={`Open ${creator.name}`}>
      {showIcon && <Avatar root={root} creator={creator} size={size} />}
      <span>{creator.name}</span>
    </button>
  );
}
