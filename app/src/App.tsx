import { useCallback, useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { AppInfo, OptimizeView, Plan } from "./types";
import { makeLabels } from "./labels";
import Tree from "./Tree";
import SidePanel from "./SidePanel";
import ListView from "./ListView";
import "./App.css";

const RULES_URL = "https://www.xbox.com/en-US/developers/rules";

function stored(key: string, fallback: string) {
  try {
    return localStorage.getItem(key) ?? fallback;
  } catch {
    return fallback;
  }
}

function remember(key: string, value: string) {
  try {
    localStorage.setItem(key, value);
  } catch {
    // Sin almacenamiento: la app funciona igual, solo no recuerda la elección.
  }
}

export default function App() {
  const [lang, setLang] = useState(() => stored("lang", "es"));
  const [info, setInfo] = useState<AppInfo | null>(null);
  const [map, setMap] = useState(() => stored("map", "arabia_neutra"));
  const [goal, setGoal] = useState(() => stored("goal", "feudal"));
  const [view, setView] = useState<OptimizeView | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [tab, setTab] = useState<"tree" | "list">("tree");
  const [selected, setSelected] = useState<number | null>(null);
  const [plan, setPlan] = useState<Plan | null>(null);

  useEffect(() => {
    invoke<AppInfo>("app_info", { lang }).then(setInfo, (e) => setError(String(e)));
    remember("lang", lang);
  }, [lang]);

  const run = useCallback(() => {
    setBusy(true);
    setError(null);
    remember("map", map);
    remember("goal", goal);
    invoke<OptimizeView>("optimize", { map, goal, lang })
      .then((v) => {
        setView(v);
        setPlan(v.result.best);
        setSelected(null);
      }, (e) => setError(String(e)))
      .finally(() => setBusy(false));
  }, [map, goal, lang]);

  // Primer cálculo al abrir, y de nuevo al cambiar de idioma (textos del motor).
  useEffect(() => {
    if (info) run();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [info]);

  const mapInfo = info?.maps.find((m) => m.id === map);
  const labels = useMemo(() => (info ? makeLabels(info, lang, mapInfo) : null), [info, lang, mapInfo]);
  const goalName = info?.goals.find((g) => g.id === goal)?.name ?? goal;

  if (!info || !labels) {
    return <div className="loading">{error ?? "IdleZero…"}</div>;
  }

  return (
    <div className="app">
      <header>
        <div className="brand">
          <svg viewBox="0 0 36 36" aria-hidden="true">
            <circle cx="18" cy="18" r="15" fill="none" stroke="var(--accent)" strokeWidth="3" />
            <path d="M18 9v9l6 4" fill="none" stroke="var(--text)" strokeWidth="2.5" strokeLinecap="round" />
            <circle cx="18" cy="18" r="2.2" fill="var(--accent)" />
          </svg>
          <div>
            <h1>IdleZero</h1>
            <span className="muted">{labels.ui.tagline}</span>
          </div>
        </div>
        <div className="controls">
          <label>
            {labels.ui.map}
            <select value={map} onChange={(e) => setMap(e.target.value)}>
              {info.maps.map((m) => (
                <option key={m.id} value={m.id}>
                  {m.name}
                </option>
              ))}
            </select>
          </label>
          <label>
            {labels.ui.goal}
            <select value={goal} onChange={(e) => setGoal(e.target.value)}>
              {info.goals.map((g) => (
                <option key={g.id} value={g.id}>
                  {g.name}
                </option>
              ))}
            </select>
          </label>
          <label>
            {labels.ui.language}
            <select value={lang} onChange={(e) => setLang(e.target.value)}>
              {info.langs.map((l) => (
                <option key={l.id} value={l.id}>
                  {l.name}
                </option>
              ))}
            </select>
          </label>
          <button className="primary" onClick={run} disabled={busy}>
            {busy ? labels.ui.calculating : labels.ui.calculate}
          </button>
        </div>
      </header>

      {error && <div className="error">{error}</div>}

      <main>
        <section className="canvas">
          <div className="tabs">
            <button className={tab === "tree" ? "active" : ""} onClick={() => setTab("tree")}>
              {labels.ui.tabTree}
            </button>
            <button className={tab === "list" ? "active" : ""} onClick={() => setTab("list")}>
              {labels.ui.tabList}
            </button>
            <span className="muted legend">{labels.ui.legend}</span>
          </div>
          <div className={`view${busy ? " busy" : ""}`}>
            {view && tab === "tree" && (
              <Tree result={view.result} labels={labels} selected={selected} onSelect={setSelected} plan={plan} />
            )}
            {view && tab === "list" && <ListView view={view} labels={labels} title={`IdleZero · ${goalName} · ${mapInfo?.name ?? map}`} />}
          </div>
        </section>
        {view && plan && (
          <SidePanel view={view} labels={labels} goalName={goalName} selected={selected} plan={plan} onPlan={setPlan} />
        )}
      </main>

      <footer className="legal">
        Age of Empires II © Microsoft Corporation. IdleZero was created under Microsoft's "
        <a href={RULES_URL} target="_blank" rel="noreferrer">
          Game Content Usage Rules
        </a>
        " using assets from Age of Empires II, and it is not endorsed by or affiliated with Microsoft. · v{info.version}
      </footer>
    </div>
  );
}
