import { useCallback, useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { AppInfo, CivView, Goal, OptimizeView, Plan, PlanView } from "./types";
import { makeLabels } from "./labels";
import { layoutMap } from "./layout";
import EconomyMap, { MapLegend, Selection } from "./EconomyMap";
import SidePanel from "./SidePanel";
import ListView from "./ListView";
import Calculator from "./Calculator";
import GoalBuilder, { emptyGoal } from "./GoalBuilder";
import Shortcuts from "./Shortcuts";
import CivPicker from "./CivPicker";
import Dashboard from "./Dashboard";
import "./App.css";

const RULES_URL = "https://www.xbox.com/en-US/developers/rules";

function stored<T>(key: string, fallback: T): T {
  try {
    const v = localStorage.getItem(key);
    return v ? (JSON.parse(v) as T) : fallback;
  } catch {
    return fallback;
  }
}

function remember(key: string, value: unknown) {
  try {
    localStorage.setItem(key, JSON.stringify(value));
  } catch {
    // Sin almacenamiento: la app funciona igual, solo no recuerda la elección.
  }
}

export default function App() {
  const [lang, setLang] = useState(() => stored("lang", "es"));
  const [info, setInfo] = useState<AppInfo | null>(null);
  const [map, setMap] = useState(() => stored("map", "arabia_neutra"));
  const [goal, setGoal] = useState<Goal>(() => stored("goal", { ...emptyGoal(), id: "feudal", age: "Feudal" }));
  const [view, setView] = useState<OptimizeView | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [tab, setTab] = useState<"map" | "dash" | "list" | "calc">("map");
  const [selected, setSelected] = useState<Selection>(null);
  const [cursor, setCursor] = useState<number | null>(null);
  const [plan, setPlan] = useState<Plan | null>(null);
  const [planView, setPlanView] = useState<PlanView | null>(null);
  const [planBusy, setPlanBusy] = useState(false);
  const [help, setHelp] = useState(false);
  const [civ, setCiv] = useState<string>(() => stored("civ", ""));
  const [civs, setCivs] = useState<CivView[]>([]);
  const [civOpen, setCivOpen] = useState(false);

  useEffect(() => {
    invoke<AppInfo>("app_info", { lang }).then(setInfo, (e) => setError(String(e)));
    invoke<CivView[]>("civs", { lang }).then(setCivs, (e) => setError(String(e)));
    remember("lang", lang);
  }, [lang]);

  const run = useCallback(() => {
    setBusy(true);
    setError(null);
    remember("map", map);
    remember("goal", goal);
    remember("civ", civ);
    invoke<OptimizeView>("optimize", { map, goal, lang, civ: civ || null })
      .then(
        (v) => {
          setView(v);
          setPlan(v.result.best);
          setPlanView(null);
          setSelected(null);
        },
        (e) => setError(String(e)),
      )
      .finally(() => setBusy(false));
  }, [map, goal, lang, civ]);

  // Primer cálculo al abrir, al cambiar de idioma (textos del motor) y al
  // cambiar de civilización (mismo mapa y objetivo, nuevos bonos).
  useEffect(() => {
    if (info) run();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [info, civ]);

  // Otro camino: se simula entero para verlo en el mapa.
  const choosePlan = (p: Plan) => {
    if (!view) return;
    setPlan(p);
    setSelected(null);
    if (p === view.result.best) {
      setPlanView(null);
      return;
    }
    setPlanBusy(true);
    invoke<PlanView>("simulate_plan", { map, goal: view.result.goal, choices: p.choices, lang, civ: civ || null })
      .then(setPlanView, (e) => setError(String(e)))
      .finally(() => setPlanBusy(false));
  };

  // Atajos globales: pestañas, calcular, ayuda.
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const tag = (e.target as HTMLElement)?.tagName;
      const typing = tag === "INPUT" || tag === "SELECT" || tag === "TEXTAREA";
      if (e.key === "Enter" && (e.ctrlKey || e.metaKey)) {
        e.preventDefault();
        run();
        return;
      }
      if (typing || e.ctrlKey || e.metaKey || e.altKey) return;
      const tabs: Record<string, "map" | "dash" | "list" | "calc"> = { "1": "map", "2": "dash", "3": "list", "4": "calc" };
      if (tabs[e.key]) setTab(tabs[e.key]);
      else if (e.key === "?") setHelp((h) => !h);
      else if (e.key === "c" || e.key === "C") setCivOpen(true);
      else if (e.key === "Escape") {
        setHelp(false);
        setCivOpen(false);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [run]);

  const civInfo = civs.find((c) => c.id === civ);
  const mapInfo = info?.maps.find((m) => m.id === map);
  const labels = useMemo(() => (info ? makeLabels(info, lang, mapInfo) : null), [info, lang, mapInfo]);
  const timeline = planView?.timeline ?? view?.result.timeline;
  const events = planView?.events ?? view?.events ?? [];
  const goalTime = planView ? planView.time : view?.result.best.time ?? null;
  const layout = useMemo(() => (timeline ? layoutMap(timeline, mapInfo) : null), [timeline, mapInfo]);

  if (!info || !labels) {
    return <div className="loading">{error ?? "IdleZero…"}</div>;
  }
  const L = labels.ui;

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
            <span className="muted">{L.tagline}</span>
          </div>
        </div>
        <div className="controls">
          <button className="civ-button" onClick={() => setCivOpen(true)} title={`${L.civ} (C)`}>
            {civInfo ? <img src={civInfo.icon} alt="" /> : <span className="civ-neutral">⚖</span>}
            <span>
              <small className="muted">{L.civ}</small>
              {civInfo?.name ?? L.neutral}
            </span>
          </button>
          <label>
            {L.map}
            <select value={map} onChange={(e) => setMap(e.target.value)}>
              {info.maps.map((m) => (
                <option key={m.id} value={m.id}>
                  {m.name}
                  {m.test ? ` (${L.testMap})` : ""}
                </option>
              ))}
            </select>
          </label>
          <label>
            {L.language}
            <select value={lang} onChange={(e) => setLang(e.target.value)}>
              {info.langs.map((l) => (
                <option key={l.id} value={l.id}>
                  {l.name}
                </option>
              ))}
            </select>
          </label>
          <button className="primary" onClick={run} disabled={busy}>
            {busy ? L.calculating : L.calculate}
          </button>
        </div>
        <div className="goal-row">
          <span className="goal-title">{L.goal}</span>
          <GoalBuilder goal={goal} onChange={setGoal} info={info} labels={labels} civ={civInfo} lang={lang} />
        </div>
      </header>

      {error && <div className="error">{error}</div>}

      <main>
        <section className="canvas">
          <div className="tabs">
            <button className={tab === "map" ? "active" : ""} onClick={() => setTab("map")}>
              {L.tabMap}
            </button>
            <button className={tab === "dash" ? "active" : ""} onClick={() => setTab("dash")}>
              {L.tabDash}
            </button>
            <button className={tab === "list" ? "active" : ""} onClick={() => setTab("list")}>
              {L.tabList}
            </button>
            <button className={tab === "calc" ? "active" : ""} onClick={() => setTab("calc")}>
              {L.tabCalc}
            </button>
            {tab === "map" && <MapLegend labels={labels} />}
            <button className="help-btn" title={`${L.shortcuts} · ${L.zoomHint}`} onClick={() => setHelp(true)}>
              ?
            </button>
          </div>
          <div className={`view${busy || planBusy ? " busy" : ""}`}>
            {tab === "map" && view && timeline && layout && (
              <EconomyMap
                layout={layout}
                timeline={timeline}
                tree={planView ? [] : view.result.tree}
                labels={labels}
                selected={selected}
                onSelect={setSelected}
                cursor={cursor}
                onCursor={setCursor}
                goalTime={goalTime}
                civIcon={civInfo?.icon}
              />
            )}
            {tab === "dash" && view && timeline && layout && (
              <Dashboard result={view.result} timeline={timeline} layout={layout} labels={labels} info={info} goalTime={goalTime} />
            )}
            {tab === "list" && view && timeline && (
              <ListView
                tree={planView ? [] : view.result.tree}
                events={events}
                timeline={timeline}
                labels={labels}
                title={`IdleZero · ${mapInfo?.name ?? map}`}
                orderToml={view.result.order_toml}
              />
            )}
            {tab === "calc" && mapInfo && <Calculator map={mapInfo} labels={labels} civ={civ} />}
          </div>
        </section>
        {view && plan && timeline && layout && tab !== "calc" && tab !== "dash" && (
          <SidePanel
            result={view.result}
            timeline={timeline}
            layout={layout}
            labels={labels}
            goalTime={goalTime}
            millis={view.millis}
            cursor={cursor}
            selected={selected}
            map={map}
            goal={view.result.goal}
            plan={plan}
            civ={civ}
            tips={info.tips}
            planBusy={planBusy}
            onPlan={choosePlan}
          />
        )}
      </main>

      {help && <Shortcuts labels={labels} onClose={() => setHelp(false)} />}
      {civOpen && (
        <CivPicker
          civs={civs}
          current={civ}
          labels={labels}
          onClose={() => setCivOpen(false)}
          onAccept={(id) => {
            setCivOpen(false);
            setCiv(id);
          }}
        />
      )}

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
