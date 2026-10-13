# IdleZero

App de escritorio gratuita (Linux y Windows) que calcula, aldeano por aldeano, la ruta más rápida para cumplir un objetivo en Age of Empires II: Definitive Edition, y la muestra como un mapa de decisiones.

Estado: **v0.3.0**: Fases 0–5 (civilizaciones con bonos simulados para Hunos, Britanos, Francos y Mayas; las demás con su árbol). Solo economía (sin combate).

![Mapa de economía](docs/img/arbol.png)

- **Mapa de economía:** el tiempo sube desde el inicio (0:00, 3 aldeanos, explorador y recursos) hacia arriba. En el centro, el CU y el nacimiento de cada aldeano; a la izquierda, lo que se construye (quién, cuándo, cuánto tarda y qué pasaría con otro número de constructores); a la derecha, la rama de cada aldeano por lo que recolecta; al lado, las reservas de cada recurso segundo a segundo.
- **Objetivo libre:** cualquier combinación de edad, aldeanos, unidades, tecnologías y edificios (p. ej. «10 ballesteros + Emplumado»).
- **Ramas alternativas:** al elegir un aldeano se ve qué pasaría si hiciera otra cosa («+25 s», «CU parado 6 s»); otros caminos completos se pueden simular y comparar.
- **Civilizaciones:** ventana tipo juego con escudos y el texto oficial de cada una; al cambiar, se recalcula con el mismo objetivo y se compara con neutro.
- **Estrategias:** Drush, Scouts, Arqueros, Fast Castle, Castillo en la cara, Boom de 3 CU… con su explicación.
- **Tablero:** indicadores, aldeanos por recurso en el tiempo, reservas, hitos y consejos de teclas para jugar la orden.
- **Calculadora:** cuánto junta de verdad un grupo de aldeanos en una tarea (carga, caminata, campamento, mejoras).
- **Libro de recursos:** cada pago queda registrado; los tests garantizan que ningún recurso baja de 0 ni aparece de la nada.

En Arabia neutra (valores aproximados), sin dejar el CU inactivo: Feudal 8:50, Castillos 15:15, Imperial 20:30. Atajos: rueda = zoom, `?` muestra todos.

Arquitectura del proyecto: [docs/arquitectura.html](docs/arquitectura.html) (generada con Archify, con enlaces al código).

## Descargas

Solo desde [GitHub Releases](../../releases). Cada versión publica `SHA256SUMS-*.txt`; verifica tu descarga con:

```bash
sha256sum -c SHA256SUMS-Linux.txt --ignore-missing
```

| Sistema | Archivos |
| --- | --- |
| Linux | `.AppImage`, `.deb`, `.rpm` |
| Windows 10/11 | `.msi`, `-setup.exe` |

## Desarrollo

Requisitos: Rust estable, Node.js LTS y las [dependencias de sistema de Tauri 2](https://tauri.app/start/prerequisites/).

```bash
git clone https://github.com/SiegeEngineers/aoe2techtree.git vendor/aoe2techtree
cargo run -p import          # regenera data/game_data.json
cargo test --workspace
cd app && npm install && npm run tauri dev

# Simular una orden de construcción, tick a tick
cargo run -p aoe2-sim --example run -- arabia_neutra fast_castle_27_2

# Optimizar un objetivo (data/goals.toml) desde la terminal
cargo run -p aoe2-opt --example optimize -- arabia_neutra castle
```

Idiomas: español por defecto e inglés (`data/i18n/`); añadir un idioma es copiar `en.toml` y traducirlo.

Las órdenes de construcción son archivos TOML en `data/build_orders/`; las reglas, mejoras económicas y mapas también viven en `data/`.

Publicar una versión: `git tag v0.x.y && git push --tags`. GitHub Actions compila en Linux y Windows y crea un borrador de Release.

## Datos

- Unidades, edificios y tecnologías: [SiegeEngineers/aoe2techtree](https://github.com/SiegeEngineers/aoe2techtree), convertidos por `tools/import`.
- Tasas de recolección y distancias de mapa: valores de la comunidad en `data/gather_rates.toml` y `data/maps/`, marcados como aproximados hasta validarlos.

## Aviso legal

Age of Empires II © Microsoft Corporation. IdleZero was created under Microsoft's "[Game Content Usage Rules](https://www.xbox.com/en-US/developers/rules)" using assets from Age of Empires II, and it is not endorsed by or affiliated with Microsoft.

El código de este repositorio se publica bajo licencia MIT.
