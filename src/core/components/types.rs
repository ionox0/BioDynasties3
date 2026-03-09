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

impl BuildingType {
    pub fn display_name(&self) -> &'static str {
        match self {
            BuildingType::Queen => "Queen Chamber",
            BuildingType::Nursery => "Nursery",
            BuildingType::WarriorChamber => "Warrior Chamber",
        }
    }
}

impl UnitType {
    pub fn display_name(&self) -> &'static str {
        match self {
            // Classic units
            UnitType::WorkerAnt => "Worker Ant",
            UnitType::BeetleKnight => "Black Ox Beetle Knight",
            UnitType::SpearMantis => "Spear Mantis",
            UnitType::ScoutAnt => "Cairns Birdwing",
            UnitType::DragonFly => "Dragonfly 2",
            UnitType::BatteringBeetle => "Battering Black Ox Beetle",
            UnitType::DefenderBug => "Defender Bug",
            UnitType::EliteSpider => "Animated Elite Spider",
            // Beetles family
            UnitType::StagBeetle => "Stag Beetle",
            UnitType::RhinoBeetle => "Rhino Beetle",
            UnitType::JewelBug => "Jewel Bug",
            // Mantids family
            UnitType::OrchidMantis => "Orchid Mantis",
            // Isopods family
            UnitType::Woodlouse => "Woodlouse",
            UnitType::SandFleas => "Sand Flea",
            // Small creatures family
            UnitType::Aphids => "Aphid",
            UnitType::Mites => "Mite",
            UnitType::Ticks => "Tick",
            UnitType::Fleas => "Flea",
            UnitType::Lice => "Louse",
            // Butterflies family
            UnitType::Moths => "Moth",
            UnitType::Caterpillars => "Caterpillar",
            UnitType::PeacockMoth => "Peacock Moth",
            // Spiders family
            UnitType::WidowSpider => "Widow Spider",
            UnitType::Tarantula => "Tarantula",
            UnitType::WolfSpider => "Wolf Spider",
            // Flies family
            UnitType::Firefly => "Firefly",
            UnitType::DragonFlies => "Dragonfly",
            UnitType::Housefly => "Common Housefly",
            // Bees family
            UnitType::Hornets => "Hornet",
            UnitType::Honeybees => "Honeybee",
            // Termites family
            UnitType::Earwigs => "Earwig",
            UnitType::TermiteWorker => "Termite Worker",
            UnitType::TermiteWarrior => "Giant Termite Warrior",
            // Individual species
            UnitType::StickBugs => "Stick Bug",
            UnitType::Scorpion => "Scorpion",
            UnitType::Stinkbug => "Stink Bug",
            UnitType::Cicadas => "Cicada",
        }
    }

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
