import { useThumbSrc } from "../hooks/useThumbSrc";

interface Props {
  root: string;
  creator: { name: string; iconImagePath?: string | null };
  size?: number;
}

/**
 * Creator icon, or their initial when they have none. Uses the cached
 * thumbnail — every place this is used shows it at 120px or smaller, well
 * within what a 384px-max thumbnail covers even at high DPI.
 */
export default function Avatar({ root, creator, size = 36 }: Props) {
  const { src, onError } = useThumbSrc(root, creator.iconImagePath);

  if (!src) {
    return (
      <div className="avatar placeholder" style={{ width: size, height: size }}>
        {creator.name.charAt(0).toUpperCase()}
      </div>
    );
  }
  return <img className="avatar" style={{ width: size, height: size }} src={src} alt="" onError={onError} />;
}
