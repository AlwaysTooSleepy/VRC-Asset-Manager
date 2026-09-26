import { useState } from "react";

export interface Suggestion {
  id: string;
  name: string;
  detail: string;
}

interface Props {
  value: string;
  onChange: (value: string) => void;
  /** Existing entries that match what's typed. Picking one calls onPick. */
  suggestions: Suggestion[];
  onPick: (id: string) => void;
  heading?: string;
  disabled?: boolean;
  autoFocus?: boolean;
}

/** Free-text input that offers matching existing entries while you type. */
export default function SuggestInput({ value, onChange, suggestions, onPick, heading = "Already in your library", disabled, autoFocus }: Props) {
  const [open, setOpen] = useState(false);

  return (
    <div className="select">
      <input
        className="input"
        value={value}
        disabled={disabled}
        autoFocus={autoFocus}
        onChange={(e) => onChange(e.target.value)}
        onFocus={() => setOpen(true)}
        onBlur={() => setOpen(false)}
        onKeyDown={(e) => e.key === "Escape" && open && suggestions.length > 0 && (e.stopPropagation(), setOpen(false))}
      />
      {open && suggestions.length > 0 && (
        <ul className="select-menu">
          <li className="muted small menu-head">{heading} (pick one to edit it):</li>
          {suggestions.map((s) => (
            <li
              key={s.id}
              className="suggest"
              onMouseDown={(e) => { e.preventDefault(); setOpen(false); onPick(s.id); }}
            >
              <span>{s.name}</span>
              <span className="muted small">{s.detail}</span>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
