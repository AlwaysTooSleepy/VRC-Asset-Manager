import { useState } from "react";
import type { MouseEvent } from "react";
import { imageUrl } from "../api";
import { useThumbSrc } from "../hooks/useThumbSrc";

interface Props {
  root: string;
  images: string[];
  name: string;
  /**
   * Card/grid tiles pass true to load the small cached thumbnail instead of
   * decoding the full-resolution original (falls back to the original
   * automatically if a thumbnail isn't available for that image yet).
   * Detail views leave this off, since they show the image large enough
   * that the original is worth the extra decode.
   */
  thumb?: boolean;
}

/** Square image area. With several images, round arrows appear on hover. */
export default function ImageCarousel({ root, images, name, thumb = false }: Props) {
  const [index, setIndex] = useState(0);
  const count = images.length;
  const current = Math.min(index, Math.max(count - 1, 0));

  const thumbnail = useThumbSrc(root, thumb ? images[current] : undefined);
  const src = thumb ? thumbnail.src : imageUrl(root, images[current]);

  if (count === 0) {
    return <div className="carousel placeholder">{name.charAt(0).toUpperCase()}</div>;
  }

  const step = (delta: number) => (e: MouseEvent) => {
    e.stopPropagation(); // don't trigger the card's own click
    setIndex((current + delta + count) % count);
  };

  return (
    <div className="carousel">
      <img src={src} alt={name} draggable={false} onError={thumb ? thumbnail.onError : undefined} />
      {count > 1 && (
        <>
          <button className="carousel-arrow left" onClick={step(-1)} aria-label="Previous image">‹</button>
          <button className="carousel-arrow right" onClick={step(1)} aria-label="Next image">›</button>
          <div className="carousel-dots">
            {images.map((_, i) => (
              <span key={i} className={i === current ? "on" : ""} />
            ))}
          </div>
        </>
      )}
    </div>
  );
}
