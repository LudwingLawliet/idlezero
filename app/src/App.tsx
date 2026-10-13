import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import "./App.css";

type EngineInfo = {
  version: string;
  source_commit: string;
  units: number;
  buildings: number;
  techs: number;
};

const RULES_URL = "https://www.xbox.com/en-US/developers/rules";

function App() {
  const [info, setInfo] = useState<EngineInfo | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    invoke<EngineInfo>("engine_info").then(setInfo, (e) => setError(String(e)));
  }, []);

  return (
    <main className="container">
      <h1>IdleZero</h1>
      <p>Cero segundos de CU inactivo: órdenes de construcción óptimas para Age of Empires II, aldeano por aldeano.</p>

      {error && <p className="error">No se pudieron cargar los datos: {error}</p>}
      {info && (
        <p className="data-info">
          Motor v{info.version} · {info.units} unidades, {info.buildings} edificios,{" "}
          {info.techs} tecnologías · datos aoe2techtree @{info.source_commit.slice(0, 7)}
        </p>
      )}

      <footer className="legal">
        Age of Empires II © Microsoft Corporation. IdleZero was created under
        Microsoft's "<a href={RULES_URL} target="_blank" rel="noreferrer">Game Content Usage Rules</a>"
        using assets from Age of Empires II, and it is not endorsed by or affiliated with Microsoft.
      </footer>
    </main>
  );
}

export default App;
