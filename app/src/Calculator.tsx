import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Labels } from "./labels";
import { CalcResult, MapInfo } from "./types";

// Mejoras económicas que cambian la recolección (claves de data/rules.toml).
const ECO_TECHS = ["double_bit_axe", "bow_saw", "horse_collar", "heavy_plow", "wheelbarrow", "hand_cart", "gold_mining", "stone_mining"];

/** "¿Cuánto junta de verdad?" con el mismo motor de la simulación. */
export default function Calculator({ map, labels, civ }: { map: MapInfo; labels: Labels; civ: string }) {
  const L = labels.ui;
  const [pool, setPool] = useState(map.pools.find((p) => p.id === "forest")?.id ?? map.pools[0].id);
  const [villagers, setVillagers] = useState(1);
  const [seconds, setSeconds] = useState(60);
  const [drop, setDrop] = useState(false);
  const [techs, setTechs] = useState<string[]>([]);
  const [result, setResult] = useState<CalcResult | null>(null);
  const [error, setError] = useState<string | null>(null);

  const run = () => {
    setError(null);
    invoke<CalcResult>("calculate", { map: map.id, input: { pool, villagers, seconds, drop_site: drop, techs }, civ: civ || null }).then(setResult, (e) =>
      setError(String(e)),
    );
  };
  const r = result;
  const res = r ? labels.t(`resources.${r.resource}`) : "";
  return (
    <div className="calc">
      <h2>{L.calcTitle}</h2>
      <div className="calc-form">
        <label>
          {L.calcTask}
          <select value={pool} onChange={(e) => setPool(e.target.value)}>
            {map.pools.map((p) => (
              <option key={p.id} value={p.id}>
                {labels.pool(p.id)}
                {p.amount !== null ? ` (${p.amount})` : ""}
              </option>
            ))}
          </select>
        </label>
        <label>
          {L.calcVillagers}
          <input type="number" min={1} max={100} value={villagers} onChange={(e) => setVillagers(Number(e.target.value))} />
        </label>
        <label>
          {L.calcSeconds}
          <input type="number" min={1} max={3600} value={seconds} onChange={(e) => setSeconds(Number(e.target.value))} />
        </label>
        <label className="check">
          <input type="checkbox" checked={drop} onChange={(e) => setDrop(e.target.checked)} />
          {L.calcDrop}
        </label>
        <fieldset>
          <legend>{L.calcTechs}</legend>
          {ECO_TECHS.map((t) => (
            <label key={t} className="check">
              <input
                type="checkbox"
                checked={techs.includes(t)}
                onChange={(e) => setTechs(e.target.checked ? [...techs, t] : techs.filter((x) => x !== t))}
              />
              <img src={labels.icon("tech", t)} alt="" className="icon small" />
              {labels.name("tech", t)}
            </label>
          ))}
        </fieldset>
        <button className="primary" onClick={run}>
          {L.calcRun}
        </button>
      </div>
      {error && <div className="error">{error}</div>}
      {r && (
        <div className="calc-result">
          <div className="big-number">
            <img src={`/img/resource/${r.resource}.png`} alt="" />
            <strong>{Math.round(r.gathered)}</strong> {res}
            <span className="muted">
              {" "}
              + {r.carrying} {L.calcCarrying}
            </span>
          </div>
          <p className="muted">
            {villagers} × {labels.pool(pool)} · {seconds} s
          </p>
          <table className="whatif">
            <tbody>
              <tr>
                <td>{L.calcRate}</td>
                <td>
                  {r.rate_per_villager.toFixed(3)} {L.perSecond}
                </td>
              </tr>
              <tr>
                <td>{L.calcRaw}</td>
                <td>
                  {r.gather_rate.toFixed(3)} {L.perSecond}
                </td>
              </tr>
              <tr>
                <td>{L.calcCarry}</td>
                <td>{r.carry_capacity}</td>
              </tr>
              <tr>
                <td>{L.calcDistance}</td>
                <td>
                  {r.distance_tiles} {L.tiles}
                </td>
              </tr>
              <tr>
                <td>{L.calcTrip}</td>
                <td>
                  {r.trip_seconds.toFixed(1)} s ({r.walk_seconds_per_trip.toFixed(1)} s {L.calcWalk})
                </td>
              </tr>
              <tr>
                <td>{L.calcTrips}</td>
                <td>{r.trips}</td>
              </tr>
            </tbody>
          </table>
          {r.depleted && <p className="notice">{L.calcDepleted}</p>}
        </div>
      )}
    </div>
  );
}
