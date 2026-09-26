import { CreatorInfo } from "../types";
import { toDisplaySrc, openInBrowser } from "../api";

export function CreatorBadge({
  name,
  creators,
  size = 20,
  clickable = true,
}: {
  name: string;
  creators: CreatorInfo[];
  size?: number;
  clickable?: boolean;
}) {
  const info = creators.find((c) => c.name === name);
  const content = (
    <span style={{ display: "inline-flex", alignItems: "center", gap: 6 }}>
      {info?.icon ? (
        <img
          src={toDisplaySrc(info.icon)}
          alt=""
          style={{ width: size, height: size, borderRadius: "50%", objectFit: "cover", flexShrink: 0 }}
        />
      ) : (
        <span
          style={{
            width: size,
            height: size,
            borderRadius: "50%",
            background: "var(--surface-2)",
            flexShrink: 0,
            display: "inline-flex",
            alignItems: "center",
            justifyContent: "center",
            fontSize: size * 0.5,
            color: "var(--text-dim)",
          }}
        >
          {name.charAt(0).toUpperCase()}
        </span>
      )}
      {name}
    </span>
  );

  if (clickable && info?.url) {
    return (
      <button
        type="button"
        className="btn-ghost"
        style={{ padding: 0, border: "none", background: "none", color: "inherit", cursor: "pointer" }}
        onClick={(e) => {
          e.stopPropagation();
          openInBrowser(info.url);
        }}
        title={`Open ${name}'s page`}
      >
        {content}
      </button>
    );
  }
  return content;
}
