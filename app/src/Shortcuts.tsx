import { Labels } from "./labels";

/** Ventana de ayuda con todos los atajos (tecla ?). */
export default function Shortcuts({ labels, onClose }: { labels: Labels; onClose: () => void }) {
  const L = labels.ui;
  const rows: [string, string][] = [
    [L.keyWheel, L.keyWheelDesc],
    [L.keyShiftWheel, L.keyShiftWheelDesc],
    [L.keyDrag, L.keyDragDesc],
    [L.keyDouble, L.keyDoubleDesc],
    ["+ / −", L.keyZoom],
    ["← ↑ → ↓", L.keyArrows],
    [L.keyHome, L.keyHomeDesc],
    [L.keyEnd, L.keyEndDesc],
    ["Esc", L.keyEsc],
    ["1 · 2 · 3 · 4", L.keyTabs],
    ["Ctrl + Enter", L.keyCalc],
    ["C", L.keyCiv],
    ["?", L.keyHelp],
  ];
  return (
    <div className="modal-backdrop" onClick={onClose}>
      <div className="modal shortcuts" onClick={(e) => e.stopPropagation()} role="dialog" aria-label={L.shortcuts}>
        <h2>{L.shortcuts}</h2>
        <table>
          <tbody>
            {rows.map(([k, d]) => (
              <tr key={k}>
                <td>
                  <kbd>{k}</kbd>
                </td>
                <td>{d}</td>
              </tr>
            ))}
          </tbody>
        </table>
        <div className="modal-actions">
          <button className="primary" onClick={onClose}>
            OK
          </button>
        </div>
      </div>
    </div>
  );
}
