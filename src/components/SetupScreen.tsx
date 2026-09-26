interface Props {
  onChoose: () => void;
  error: string | null;
}

export default function SetupScreen({ onChoose, error }: Props) {
  return (
    <div className="center">
      <div className="card setup">
        <h1>VRChat Asset Manager</h1>
        <p className="muted">
          Choose a folder to hold your library. The app will create <code>Models</code>,{" "}
          <code>Assets</code> and <code>AppData</code> inside it and organize everything you add.
        </p>
        <button className="btn primary" onClick={onChoose}>
          Choose library folder
        </button>
        {error && <div className="error">{error}</div>}
      </div>
    </div>
  );
}
