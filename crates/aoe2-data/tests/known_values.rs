//! Valida data/ contra valores conocidos de AoE2 DE.
//! Si un parche cambia alguno, este test avisa qué cambió.

use aoe2_data::{ids, Age, Cost, GameData, GatherRates, MapDef, SCHEMA_VERSION, repo_data_dir};

fn game_data() -> GameData {
    GameData::load(&repo_data_dir().join("game_data.json")).expect("cargar game_data.json")
}

#[test]
fn villager_costs_50_food_and_25_seconds() {
    let data = game_data();
    let v = data.unit(ids::VILLAGER).unwrap();
    assert_eq!(v.cost, Cost::new(50, 0, 0, 0));
    assert_eq!(v.train_time, 25);
    assert_eq!(v.tree.as_ref().unwrap().produced_at, Some(ids::TOWN_CENTER));
}

#[test]
fn archer_costs_25_wood_45_gold() {
    let data = game_data();
    let a = data.unit(ids::ARCHER).unwrap();
    assert_eq!(a.cost, Cost::new(0, 25, 45, 0));
    assert_eq!(a.tree.as_ref().unwrap().age, Age::Feudal);
}

/// (id, nombre en inglés, costo, tiempo en s)
#[test]
fn known_units() {
    let data = game_data();
    let cases = [
        (83, "Villager", Cost::new(50, 0, 0, 0), 25),
        (4, "Archer", Cost::new(0, 25, 45, 0), 35),
        (24, "Crossbowman", Cost::new(0, 25, 45, 0), 27),
        (93, "Spearman", Cost::new(35, 25, 0, 0), 22),
        (448, "Scout Cavalry", Cost::new(80, 0, 0, 0), 30),
        (38, "Knight", Cost::new(60, 0, 75, 0), 30),
        (280, "Mangonel", Cost::new(0, 160, 135, 0), 46),
        (125, "Monk", Cost::new(0, 0, 100, 0), 51),
        (128, "Trade Cart", Cost::new(0, 100, 50, 0), 51),
        (13, "Fishing Ship", Cost::new(0, 75, 0, 0), 40),
    ];
    for (id, name, cost, time) in cases {
        let u = data.unit(id).unwrap_or_else(|| panic!("falta unidad {id}"));
        assert_eq!(u.names.en, name, "nombre de {id}");
        assert_eq!(u.cost, cost, "costo de {name}");
        assert_eq!(u.train_time, time, "tiempo de {name}");
    }
}

#[test]
fn known_techs() {
    let data = game_data();
    let cases = [
        (22, "Loom", Cost::new(0, 0, 50, 0), 25, Age::Dark),
        (ids::FEUDAL_AGE, "Feudal Age", Cost::new(500, 0, 0, 0), 130, Age::Dark),
        (ids::CASTLE_AGE, "Castle Age", Cost::new(800, 0, 200, 0), 160, Age::Feudal),
        (ids::IMPERIAL_AGE, "Imperial Age", Cost::new(1000, 0, 800, 0), 190, Age::Castle),
        (199, "Fletching", Cost::new(100, 0, 50, 0), 30, Age::Feudal),
        (200, "Bodkin Arrow", Cost::new(200, 0, 100, 0), 35, Age::Castle),
        (202, "Double-Bit Axe", Cost::new(100, 50, 0, 0), 25, Age::Feudal),
        (14, "Horse Collar", Cost::new(75, 75, 0, 0), 20, Age::Feudal),
        (213, "Wheelbarrow", Cost::new(175, 50, 0, 0), 75, Age::Feudal),
        (67, "Forging", Cost::new(150, 0, 0, 0), 50, Age::Feudal),
        (211, "Padded Archer Armor", Cost::new(100, 0, 0, 0), 40, Age::Feudal),
    ];
    for (id, name, cost, time, age) in cases {
        let t = data.tech(id).unwrap_or_else(|| panic!("falta tecnología {id}"));
        assert_eq!(t.names.en, name, "nombre de {id}");
        assert_eq!(t.cost, cost, "costo de {name}");
        assert_eq!(t.research_time, time, "tiempo de {name}");
        assert_eq!(t.tree.as_ref().unwrap().age, age, "edad de {name}");
    }
}

#[test]
fn known_buildings() {
    let data = game_data();
    let cases = [
        (ids::HOUSE, "House", Cost::new(0, 25, 0, 0), 25),
        (ids::MILL, "Mill", Cost::new(0, 100, 0, 0), 35),
        (ids::LUMBER_CAMP, "Lumber Camp", Cost::new(0, 100, 0, 0), 35),
        (ids::MINING_CAMP, "Mining Camp", Cost::new(0, 100, 0, 0), 35),
        (ids::FARM, "Farm", Cost::new(0, 60, 0, 0), 15),
        (12, "Barracks", Cost::new(0, 175, 0, 0), 50),
        (ids::TOWN_CENTER, "Town Center", Cost::new(0, 275, 0, 100), 100),
        (82, "Castle", Cost::new(0, 0, 0, 650), 200),
    ];
    for (id, name, cost, time) in cases {
        let b = data.building(id).unwrap_or_else(|| panic!("falta edificio {id}"));
        assert_eq!(b.names.en, name, "nombre de {id}");
        assert_eq!(b.cost, cost, "costo de {name}");
        assert_eq!(b.build_time, time, "tiempo de {name}");
    }
}

#[test]
fn archery_range_requires_barracks_and_farm_requires_mill() {
    let data = game_data();
    let range = data.building(87).unwrap().tree.clone().unwrap();
    assert_eq!(range.requires_building, Some(12));
    assert_eq!(range.age, Age::Feudal);
    let farm = data.building(ids::FARM).unwrap().tree.clone().unwrap();
    assert_eq!(farm.requires_building, Some(ids::MILL));
}

#[test]
fn spanish_names_are_imported_without_html() {
    let data = game_data();
    assert_eq!(data.unit(ids::VILLAGER).unwrap().names.es, "Aldeano");
    assert_eq!(data.building(ids::LUMBER_CAMP).unwrap().names.es, "Campamento maderero");
    for name in data.units.values().map(|u| &u.names.es) {
        assert!(!name.contains('<') && !name.contains('\n'), "nombre sucio: {name:?}");
    }
}

#[test]
fn schema_version_matches() {
    assert_eq!(game_data().schema_version, SCHEMA_VERSION);
}

#[test]
fn gather_rates_parse_and_match_spec() {
    let rates = GatherRates::load(&repo_data_dir().join("gather_rates.toml")).unwrap();
    assert_eq!(rates.carry.default, 10);
    assert_eq!(rates.carry.hunting, 35);
    assert_eq!(rates.tasks.len(), 8);
    assert_eq!(rates.task("lumberjack").unwrap().rate, 0.39);
    assert_eq!(rates.carry_for(rates.task("hunter").unwrap()), 35);
    for t in &rates.tasks {
        assert!(!t.source.is_empty(), "{} sin fuente", t.id);
    }
    // Casa (25 s) con 1 constructor = 25 s; con 4 = 3·25/6 = 12.5 s.
    assert_eq!(rates.build_time(25, 1), 25.0);
    assert_eq!(rates.build_time(25, 4), 12.5);
}

#[test]
fn effective_rate_accounts_for_walking() {
    use aoe2_data::rates::effective_rate;
    // Sin caminata la tasa no cambia.
    assert!((effective_rate(10, 0.39, 0.0) - 0.39).abs() < 1e-12);
    // Con caminata siempre baja.
    assert!(effective_rate(10, 0.39, 3.0) < 0.39);
}

#[test]
fn maps_parse_and_reference_known_tasks() {
    let rates = GatherRates::load(&repo_data_dir().join("gather_rates.toml")).unwrap();
    let data = game_data();
    for file in ["laboratorio.toml", "arabia_neutra.toml"] {
        let map = MapDef::load(&repo_data_dir().join("maps").join(file)).unwrap();
        assert_eq!(map.start.resources, Cost::new(200, 200, 100, 200), "{file}");
        assert_eq!(map.start.villagers, 3);
        for s in &map.sources {
            assert!(rates.task(&s.task).is_some(), "{file}: tarea desconocida {}", s.task);
            if let Some(b) = s.drop_site {
                assert!(data.building(b).is_some(), "{file}: edificio desconocido {b}");
            }
        }
    }
    let lab = MapDef::load(&repo_data_dir().join("maps/laboratorio.toml")).unwrap();
    assert!(lab.sources.iter().all(|s| !["shepherd", "hunter", "forager"].contains(&s.task.as_str())));
}

#[test]
fn rules_reference_existing_entities() {
    let data = game_data();
    let rules = aoe2_data::Rules::load(&repo_data_dir().join("rules.toml")).unwrap();
    for (name, id) in &rules.names.buildings {
        assert!(data.building(*id).is_some(), "edificio {name} = {id} no existe");
    }
    for (name, id) in &rules.names.techs {
        assert!(data.tech(*id).is_some(), "tecnología {name} = {id} no existe");
    }
    for req in &rules.age_requirements {
        assert!(data.tech(req.tech).is_some());
        assert!(req.buildings.iter().all(|b| data.building(*b).is_some()));
    }
    assert_eq!(rules.age_of_tech(ids::FEUDAL_AGE), Some(Age::Feudal));
    assert_eq!(rules.age_of_tech(ids::CASTLE_AGE), Some(Age::Castle));
    assert_eq!(rules.population.per_house, 5);
}

#[test]
fn modifiers_reference_existing_techs_and_tasks() {
    let data = game_data();
    let rates = GatherRates::load(&repo_data_dir().join("gather_rates.toml")).unwrap();
    let mods = aoe2_data::Modifiers::load(&repo_data_dir().join("modifiers.toml")).unwrap();
    for m in &mods.modifiers {
        assert!(data.tech(m.tech).is_some(), "tecnología {} no existe", m.tech);
        for t in &m.tasks {
            assert!(rates.task(t).is_some(), "tarea desconocida {t}");
        }
    }
    // Hacha de doble filo: +20 % madera.
    let dba: Vec<_> = mods.for_tech(202).collect();
    assert_eq!(dba.len(), 1);
    assert_eq!(dba[0].value, 1.20);
}
