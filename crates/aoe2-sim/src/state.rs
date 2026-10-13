//! Estado del juego que avanza el simulador.

use aoe2_data::rates::Resource;
use aoe2_data::{Age, Cost};

pub(crate) const EPS: f64 = 1e-9;

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Resources {
    pub food: f64,
    pub wood: f64,
    pub gold: f64,
    pub stone: f64,
}

impl Resources {
    pub fn from_cost(c: &Cost) -> Resources {
        Resources { food: c.food as f64, wood: c.wood as f64, gold: c.gold as f64, stone: c.stone as f64 }
    }

    pub fn can_afford(&self, c: &Cost) -> bool {
        self.food + EPS >= c.food as f64
            && self.wood + EPS >= c.wood as f64
            && self.gold + EPS >= c.gold as f64
            && self.stone + EPS >= c.stone as f64
    }

    pub fn pay(&mut self, c: &Cost) {
        debug_assert!(self.can_afford(c));
        self.food -= c.food as f64;
        self.wood -= c.wood as f64;
        self.gold -= c.gold as f64;
        self.stone -= c.stone as f64;
    }

    pub fn get_mut(&mut self, r: Resource) -> &mut f64 {
        match r {
            Resource::Food => &mut self.food,
            Resource::Wood => &mut self.wood,
            Resource::Gold => &mut self.gold,
            Resource::Stone => &mut self.stone,
        }
    }
}

/// Una fuente de recurso del mapa (puede agrupar varias: 4 ovejas, 7 montones de oro…).
#[derive(Debug, Clone)]
pub struct SourceState {
    pub id: String,
    pub pool: usize,
    /// Índice en `GatherRates::tasks`.
    pub task: usize,
    /// `f64::INFINITY` = inagotable.
    pub remaining: f64,
    /// Casillas actuales hasta el punto de entrega.
    pub distance: f64,
    pub distance_with_drop_site: Option<f64>,
    pub drop_site: Option<u32>,
    pub has_drop_site: bool,
    /// Ya hay un punto de entrega encargado para esta fuente.
    pub drop_site_planned: bool,
    pub arrives_at: u32,
    pub lure_seconds: u32,
    pub lured: bool,
    pub luring: bool,
}

#[derive(Debug, Clone)]
pub struct Farm {
    pub remaining: f64,
}

/// Qué recolecta un aldeano.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Target {
    Source(usize),
    Farm(usize),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Activity {
    /// Sin tarea: en el siguiente segundo busca trabajo en su pool.
    Idle,
    Gathering(Target),
    /// Camina al punto de entrega con lo que carga.
    ToDrop { target: Target, left: f64 },
    /// Vuelve a la fuente después de depositar.
    Back { target: Target, left: f64 },
    /// Atrae un jabalí al CU.
    Luring { source: usize, left: f64 },
    /// Espera recursos para colocar un edificio que le asignó la orden.
    WaitBuild { building: u32, at: Option<usize> },
    ToSite { site: usize, left: f64 },
    Building { site: usize },
}

impl Activity {
    pub fn is_building(&self) -> bool {
        matches!(self, Activity::WaitBuild { .. } | Activity::ToSite { .. } | Activity::Building { .. })
    }
}

#[derive(Debug, Clone)]
pub struct Villager {
    pub id: u32,
    /// Pool al que vuelve cuando no tiene otra cosa que hacer.
    pub pool: usize,
    pub activity: Activity,
    pub carry: f64,
    pub carry_resource: Option<Resource>,
    pub farm: Option<usize>,
    /// Si en el último segundo no tuvo nada que hacer.
    pub idle: bool,
}

/// Edificio en construcción (o terminado).
#[derive(Debug, Clone)]
pub struct Site {
    pub building: u32,
    pub progress: f64,
    pub done: bool,
    /// Granja de este aldeano (las granjas se reasignan a su dueño).
    pub farm_for: Option<u32>,
    /// Pool cuyo punto de entrega acorta este edificio.
    pub at: Option<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Job {
    Villager,
    Research(u32),
    Unit(u32),
}

/// Producción o investigación en curso en un edificio.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Running {
    pub building: u32,
    pub job: Job,
    pub left: u32,
}

/// Multiplicadores activos por las tecnologías investigadas.
#[derive(Debug, Clone)]
pub struct Effects {
    /// Por tarea (índice en `GatherRates::tasks`).
    pub rate_mult: Vec<f64>,
    pub carry_add: Vec<f64>,
    pub carry_mult: f64,
    pub speed_mult: f64,
    pub farm_food_add: f64,
}

#[derive(Debug, Clone)]
pub struct GameState {
    pub time: u32,
    pub age: Age,
    pub res: Resources,
    pub scouts: u32,
    pub town_centers: u32,
    pub villagers: Vec<Villager>,
    pub sources: Vec<SourceState>,
    pub farms: Vec<Farm>,
    pub sites: Vec<Site>,
    /// (edificio, segundo terminado)
    pub built: Vec<(u32, u32)>,
    pub techs: Vec<u32>,
    pub effects: Effects,
    /// Lo que hace el CU (uno solo en esta fase).
    pub tc: Option<Running>,
    /// Investigaciones en otros edificios.
    pub research: Vec<Running>,
    /// Investigaciones pedidas por la orden que aún no empiezan.
    pub pending_research: Vec<u32>,
    /// Investigaciones del CU pedidas en modo suave (no paran los aldeanos).
    pub soft_research: Vec<u32>,
    /// Unidades en entrenamiento (una por edificio productor).
    pub training: Vec<Running>,
    /// Unidades pedidas: (unidad, cuántas faltan por empezar).
    pub train_orders: Vec<(u32, u32)>,
    /// Unidades militares terminadas, por id.
    pub units: std::collections::BTreeMap<u32, u32>,
    pub next_villager: u32,
}

impl GameState {
    pub fn pop(&self) -> u32 {
        self.villagers.len() as u32 + self.scouts + self.units.values().sum::<u32>()
    }

    /// Edificios de este tipo terminados o en obra.
    pub fn count_planned(&self, building: u32) -> usize {
        self.count_built(building) + self.sites.iter().filter(|s| s.building == building && !s.done).count()
    }

    pub fn units_of(&self, unit: u32) -> u32 {
        self.units.get(&unit).copied().unwrap_or(0)
    }

    pub fn count_built(&self, building: u32) -> usize {
        self.built.iter().filter(|(b, _)| *b == building).count()
    }

    pub fn has_tech(&self, tech: u32) -> bool {
        self.techs.contains(&tech)
    }
}
