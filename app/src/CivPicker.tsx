import { useMemo, useState } from "react";
import { Labels } from "./labels";
import { CivView } from "./types";

/** Ventana de selección de civilización, al estilo del juego. */
export default function CivPicker({
  civs,
  current,
  labels,
  onAccept,
  onClose,
}: {
  civs: CivView[];
  current: string;
  labels: Labels;
  onAccept: (id: string) => void;
  onClose: () => void;
}) {
  const L = labels.ui;
  const [pick, setPick] = useState(current);
  const [query, setQuery] = useState("");
  const shown = useMemo(() => {
    const q = query.trim().toLowerCase();
    return civs.filter((c) => !q || c.name.toLowerCase().includes(q));
  }, [civs, query]);
  const sel = civs.find((c) => c.id === pick);

  return (
    <div className="modal-backdrop" onClick={onClose}>
      <div className="modal civ-picker" onClick={(e) => e.stopPropagation()} role="dialog" aria-label={L.civTitle}>
        <div className="civ-head">
          <h2>{L.civTitle}</h2>
          <input autoFocus placeholder={L.search} value={query} onChange={(e) => setQuery(e.target.value)} />
        </div>
        <div className="civ-body">
          <div className="civ-grid">
            <button className={`civ-cell${pick === "" ? " active" : ""}`} onClick={() => setPick("")} onDoubleClick={() => onAccept("")}>
              <span className="civ-neutral">⚖</span>
              <span>{L.neutral}</span>
            </button>
            {shown.map((c) => (
              <button
                key={c.id}
                className={`civ-cell${pick === c.id ? " active" : ""}${c.simulated.length ? " simulated" : ""}`}
                onClick={() => setPick(c.id)}
                onDoubleClick={() => onAccept(c.id)}
                title={c.name}
              >
                <img src={c.icon} alt="" />
                <span>{c.name}</span>
                {c.simulated.length > 0 && <i className="dot" title={L.civSimulated} />}
              </button>
            ))}
          </div>
          <div className="civ-detail">
            {sel ? (
              <>
                <div className="civ-title">
                  <img src={sel.icon} alt="" />
                  <h3>{sel.name}</h3>
                </div>
                {sel.simulated.length > 0 ? (
                  <div className="civ-badge ok">
                    <strong>{L.civSimulated}</strong>
                    <ul>
                      {sel.simulated.map((t) => (
                        <li key={t}>{t}</li>
                      ))}
                    </ul>
                  </div>
                ) : (
                  <div className="civ-badge pending">{L.civTreeOnly}</div>
                )}
                <pre className="civ-desc">{sel.description}</pre>
              </>
            ) : (
              <>
                <div className="civ-title">
                  <span className="civ-neutral big">⚖</span>
                  <h3>{L.neutral}</h3>
                </div>
                <p className="muted">{L.neutralDesc}</p>
              </>
            )}
          </div>
        </div>
        <div className="modal-actions">
          <span className="muted tiny">{L.civHint}</span>
          <button onClick={onClose}>{L.cancel}</button>
          <button className="primary" onClick={() => onAccept(pick)}>
            {L.accept}
          </button>
        </div>
      </div>
    </div>
  );
}
