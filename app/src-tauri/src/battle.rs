//! Pestaña Batalla: combate entre dos ejércitos y duelo de estrategias.

use crate::bundle::{self, bundle};
use aoe2_battle::{Army, Side, Squad, fight};
use aoe2_data::{Age, Cost};
use aoe2_opt::{Goal, OptConfig};
use aoe2_sim::{Event, Scenario, Timeline};
use serde::{Deserialize, Serialize};

/// Un bando tal como lo arma la interfaz (unidades y mejoras por clave).
#[derive(Debug, Clone, Deserialize)]
pub struct ArmyInput {
    pub civ: Option<String>,
    pub age: Age,
    #[serde(default)]
    pub techs: Vec<String>,
    pub squads: Vec<SquadInput>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SquadInput {
    pub unit: String,
    pub count: u32,
}

#[derive(Serialize)]
pub struct UnitCount {
    id: u32,
    name: String,
    count: u32,
    /// Vida que les queda (supervivientes) o vida de cada una (ejército).
    hp: f64,
}

#[derive(Serialize)]
pub struct SideView {
    cost: Cost,
    destroyed: u32,
    survivors: Vec<UnitCount>,
}

#[derive(Serialize)]
pub struct BattleView {
    /// "a", "b" o null (empate).
    winner: Option<Side>,
    seconds: f64,
    /// Vida total de cada bando cada `sample` segundos.
    hp: [Vec<f64>; 2],
    sample: f64,
    sides: [SideView; 2],
}

fn scenario() -> Result<Scenario, String> {
    bundle::scenario("arabia_neutra", None)
}

fn to_army(sc: &Scenario, a: &ArmyInput) -> Result<Army, String> {
    let squads = a
        .squads
        .iter()
        .filter(|s| s.count > 0)
        .map(|s| sc.unit_ref(&s.unit).map(|unit| Squad { unit, count: s.count.min(200) }).ok_or(format!("unidad desconocida: {}", s.unit)))
        .collect::<Result<Vec<_>, _>>()?;
    let techs = a.techs.iter().filter_map(|t| sc.tech_ref(t)).collect();
    Ok(Army { civ: a.civ.clone().filter(|c| !c.is_empty()), age: a.age, techs, squads })
}

fn run(a: &Army, b: &Army, lang: &str) -> Result<BattleView, String> {
    let bd = bundle();
    let r = fight(&bd.data, &bd.combat, a, b)?;
    let name = |id: u32| bd.data.unit(id).map_or(id.to_string(), |u| u.names.get(lang).to_owned());
    let side = |i: usize| SideView {
        cost: r.cost[i],
        destroyed: r.destroyed[i],
        survivors: r.survivors[i].iter().map(|s| UnitCount { id: s.unit, name: name(s.unit), count: s.count, hp: s.hp.round() }).collect(),
    };
    Ok(BattleView { winner: r.winner, seconds: r.seconds, hp: r.hp.clone(), sample: aoe2_battle::SAMPLE_SECONDS, sides: [side(0), side(1)] })
}

/// Combate entre dos ejércitos.
#[tauri::command]
pub async fn battle(a: ArmyInput, b: ArmyInput, lang: String) -> Result<BattleView, String> {
    crate::blocking(move || {
        let sc = scenario()?;
        run(&to_army(&sc, &a)?, &to_army(&sc, &b)?, &lang)
    })
    .await
}

// ---------- duelo de estrategias ----------

#[derive(Debug, Clone, Deserialize)]
pub struct DuelSide {
    pub goal: Goal,
    pub civ: Option<String>,
}

#[derive(Serialize)]
pub struct DuelMinute {
    minute: u32,
    units: [Vec<UnitCount>; 2],
    /// "a", "b" o null (empate o nadie tiene ejército).
    winner: Option<Side>,
    /// Vida que le queda al ganador (0..1); 0 si nadie gana.
    margin: f64,
}

#[derive(Serialize)]
pub struct DuelView {
    /// Segundo en que cada plan cumple su objetivo.
    goal_time: [Option<u32>; 2],
    minutes: Vec<DuelMinute>,
}

/// Ejército, mejoras y edad de un plan en el segundo `t`.
fn army_at(sc: &Scenario, tl: &Timeline, civ: &Option<String>, t: u32) -> Army {
    let mut squads: Vec<Squad> = Vec::new();
    let mut techs = Vec::new();
    let mut age = Age::Dark;
    for e in tl.events.iter().take_while(|e| e.time <= t) {
        match &e.event {
            Event::UnitTrained { unit, count } => {
                if let Some(id) = sc.unit_ref(unit) {
                    match squads.iter_mut().find(|s| s.unit == id) {
                        Some(s) => s.count = s.count.max(*count),
                        None => squads.push(Squad { unit: id, count: *count }),
                    }
                }
            }
            Event::ResearchFinished { tech } => techs.extend(sc.tech_ref(tech)),
            Event::AgeReached { age: a } => age = *a,
            _ => {}
        }
    }
    Army { civ: civ.clone(), age, techs, squads }
}

/// Optimiza las dos economías y, minuto a minuto, enfrenta los ejércitos que
/// cada una tiene en ese momento.
#[tauri::command]
pub async fn strategy_duel(map: String, a: DuelSide, b: DuelSide, lang: String) -> Result<DuelView, String> {
    aoe2_opt::reset_cancel();
    crate::blocking(move || {
        let sides = [a, b];
        // Las dos optimizaciones a la vez (cada una ya reparte su trabajo).
        let results: Vec<Result<(Scenario, aoe2_opt::OptResult), String>> = std::thread::scope(|s| {
            let handles: Vec<_> = sides
                .iter()
                .map(|side| {
                    let map = map.clone();
                    s.spawn(move || {
                        let sc = bundle::scenario(&map, side.civ.as_deref())?;
                        let r = aoe2_opt::optimize(&sc, &side.goal, OptConfig::default())?;
                        Ok((sc, r))
                    })
                })
                .collect();
            handles.into_iter().map(|h| h.join().unwrap_or_else(|_| Err("error interno".into()))).collect()
        });
        let mut runs = Vec::new();
        for r in results {
            runs.push(r?);
        }
        let bd = bundle();
        let name = |id: u32| bd.data.unit(id).map_or(id.to_string(), |u| u.names.get(&lang).to_owned());
        let end = runs.iter().map(|(_, r)| r.timeline.summary.end_time).max().unwrap_or(0);
        let last_minute = end.div_ceil(60) + 1;
        let mut minutes = Vec::new();
        for m in 1..=last_minute {
            let t = m * 60;
            let armies: Vec<Army> = runs.iter().zip(&sides).map(|((sc, r), side)| army_at(sc, &r.timeline, &side.civ, t)).collect();
            let units = |i: usize| -> Vec<UnitCount> {
                armies[i]
                    .squads
                    .iter()
                    .map(|s| UnitCount { id: s.unit, name: name(s.unit), count: s.count, hp: bd.data.unit(s.unit).map_or(0.0, |u| u.hp as f64) })
                    .collect()
            };
            let has = |i: usize| armies[i].squads.iter().any(|s| s.count > 0);
            let (winner, margin) = if !has(0) && !has(1) {
                (None, 0.0)
            } else {
                let r = fight(&bd.data, &bd.combat, &armies[0], &armies[1])?;
                let left = |i: usize| {
                    let start = r.hp[i].first().copied().unwrap_or(0.0);
                    if start > 0.0 { r.hp[i].last().copied().unwrap_or(0.0).max(0.0) / start } else { 0.0 }
                };
                match r.winner {
                    Some(Side::A) => (Some(Side::A), left(0)),
                    Some(Side::B) => (Some(Side::B), left(1)),
                    None => (None, 0.0),
                }
            };
            minutes.push(DuelMinute { minute: m, units: [units(0), units(1)], winner, margin });
        }
        Ok(DuelView { goal_time: [runs[0].1.best.time, runs[1].1.best.time], minutes })
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn battle_command_resolves_keys_and_reports_a_winner() {
        let sc = scenario().unwrap();
        let a = ArmyInput { civ: None, age: Age::Castle, techs: vec![], squads: vec![SquadInput { unit: "38".into(), count: 10 }] };
        let b = ArmyInput { civ: None, age: Age::Castle, techs: vec![], squads: vec![SquadInput { unit: "4".into(), count: 10 }] };
        let v = run(&to_army(&sc, &a).unwrap(), &to_army(&sc, &b).unwrap(), "es").unwrap();
        assert_eq!(v.winner, Some(Side::A));
        assert!(v.sides[0].survivors.iter().any(|s| s.name == "Caballero"));
    }

    #[test]
    fn strategy_duel_fights_minute_by_minute() {
        let presets = aoe2_opt::goal::load_presets(&aoe2_data::repo_data_dir()).unwrap();
        let goal = |id: &str| presets.iter().find(|g| g.id == id).unwrap().clone();
        let a = DuelSide { goal: goal("scouts"), civ: None };
        let b = DuelSide { goal: goal("drush"), civ: Some("Franks".into()) };
        let v = tauri::async_runtime::block_on(strategy_duel("arabia_neutra".into(), a, b, "es".into())).unwrap();
        assert!(v.goal_time.iter().all(|t| t.is_some()));
        // La milicia del drush llega antes que los exploradores: algún minuto gana B.
        assert!(v.minutes.iter().any(|m| m.winner == Some(Side::B)), "{:?}", v.minutes.iter().map(|m| (m.minute, m.winner)).collect::<Vec<_>>());
        // Al final los dos tienen su ejército completo.
        let last = v.minutes.last().unwrap();
        assert!(last.units[0].iter().map(|u| u.count).sum::<u32>() >= 3);
    }
}
