use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum BuildingType {
    Queen,
    Nursery,
    WarriorChamber,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum UnitType {
    WorkerAnt,
    BeetleKnight,
    SpearMantis,
    ScoutAnt,
    BatteringBeetle,

    DragonFly,   // Flying reconnaissance unit
    DefenderBug, // Defensive unit
    EliteSpider, // Elite predator unit

    Scorpion,     // Heavy melee unit with armor
    WolfSpider,   // Heavy predator unit

    Housefly,       // Fast flying harassment unit
    TermiteWorker,  // Builder/gatherer specialist
    TermiteWarrior, // Heavy siege unit (giant_termite.glb)
    Stinkbug,       // Area denial/support unit

    // Beetles family
    StagBeetle,
    RhinoBeetle,
    JewelBug,

    // Mantids family
    OrchidMantis,

    // Cephalopoda family (Isopods/Crustaceans)
    Woodlouse,
    SandFleas,

    // Small creatures family
    Aphids,
    Mites,
    Ticks,
    Fleas,
    Lice,

    // Butterflies family
    Moths,
    Caterpillars,
    PeacockMoth,

    // Spiders family
    WidowSpider,
    Tarantula,

    // Flies family
    Firefly,
    DragonFlies,

    // Bees family
    Hornets,
    Honeybees,

    // Termites family
    Earwigs,

    // Individual species
    StickBugs,
    Cicadas,
}

impl UnitType {
    pub fn is_worker(&self) -> bool {
        matches!(self, UnitType::WorkerAnt | UnitType::TermiteWorker)
    }

    /// The building type required to produce this unit.
    pub fn required_building(&self) -> BuildingType {
        match self {
            UnitType::WorkerAnt | UnitType::TermiteWorker => BuildingType::Queen,
            UnitType::BeetleKnight
            | UnitType::SpearMantis
            | UnitType::ScoutAnt
            | UnitType::BatteringBeetle => BuildingType::WarriorChamber,
            _ => BuildingType::Nursery,
        }
    }

    /// Nectar cost to produce this unit.
    pub fn build_cost_nectar(&self) -> f32 {
        if self.is_worker() { 20.0 } else { 30.0 }
    }
}
