import { useEffect } from "react";
import type { Library } from "../types";
import CreatorForm from "./CreatorForm";

interface Props {
  root: string;
  initialName: string;
  onSaved: (library: Library, creatorId: string) => void;
  onCancel: () => void;
}

/** Small "new creator" dialog shown on top of another modal. */
export default function CreatorDialog({ root, initialName, onSaved, onCancel }: Props) {
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => e.key === "Escape" && onCancel();
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onCancel]);

  return (
    <div className="overlay nested" onMouseDown={(e) => e.target === e.currentTarget && onCancel()}>
      <div className="modal small" role="dialog" aria-label="New creator">
        <div className="modal-head"><h2>New creator</h2></div>
        <div className="modal-body">
          <CreatorForm root={root} creator={null} initialName={initialName} onSaved={onSaved} onCancel={onCancel} />
        </div>
      </div>
    </div>
  );
}
