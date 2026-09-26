import { useMemo, useState } from "react";

interface Option { id: string; name: string }

interface Props {
  value: string | null;
  options: Option[];
  placeholder: string;
  onChange: (id: string) => void;
  /** If given, typing a name that doesn't exist offers to create it. Returns the new id. */
  onCreate?: (name: string) => Promise<string | null>;
  disabled?: boolean;
}

/** Searchable dropdown that stays usable with hundreds of entries. */
export default function SearchSelect({ value, options, placeholder, onChange, onCreate, disabled }: Props) {
  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState("");
  const selected = options.find((o) => o.id === value);
  const q = query.trim().toLowerCase();

  const filtered = useMemo(
    () =>
      options
        .filter((o) => o.name.toLowerCase().includes(q))
        .sort((a, b) => a.name.localeCompare(b.name))
        .slice(0, 100),
    [options, q]
  );
  const canCreate = !!onCreate && q !== "" && !options.some((o) => o.name.toLowerCase() === q);

  const close = () => { setOpen(false); setQuery(""); };
  const pick = (id: string) => { onChange(id); close(); };
  const create = async () => {
    const id = await onCreate!(query.trim());
    if (id) pick(id);
  };

  return (
    <div className="select">
      <input
        className="input"
        placeholder={placeholder}
        disabled={disabled}
        value={open ? query : selected?.name ?? ""}
        onFocus={() => setOpen(true)}
        onBlur={close}
        onChange={(e) => setQuery(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === "Enter") {
            e.preventDefault();
            if (q === "") return;
            if (filtered.length) pick(filtered[0].id);
            else if (canCreate) create();
          }
          if (e.key === "Escape" && open) {
            e.stopPropagation();
            close();
            (e.target as HTMLInputElement).blur();
          }
        }}
      />
      {open && (
        <ul className="select-menu">
          {filtered.map((o) => (
            <li
              key={o.id}
              className={o.id === value ? "selected" : ""}
              onMouseDown={(e) => { e.preventDefault(); pick(o.id); }}
            >
              {o.name}
            </li>
          ))}
          {canCreate && (
            <li className="create" onMouseDown={(e) => { e.preventDefault(); create(); }}>
              + Create “{query.trim()}”
            </li>
          )}
          {!filtered.length && !canCreate && <li className="muted">No matches</li>}
        </ul>
      )}
    </div>
  );
}
