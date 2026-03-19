use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum BuildingType {
    Queen,
    Nursery,
    WarriorChamber,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum UnitType {
    Fourmi,          // Worker gatherer ant
    Bee,             // Gathering bee
    CairnsBirdwing,  // Flying scout butterfly
    Dragonfly,       // Flying elite combat unit
    RolyPoly,        // Defensive tank
    Scorpion,        // Heavy melee DPS
    GoliathBirdeater, // Heavy predator spider
    RhinoBeetle,     // Armored tank beetle
}

impl BuildingType {
    pub fn display_name(&self) -> &'static str {
        match self {
            BuildingType::Queen => "Queen Chamber",
            BuildingType::Nursery => "Nursery",
            BuildingType::WarriorChamber => "Warrior Chamber",
        }
    }

    /// Radius (world units) that must be clear of other buildings and passable terrain
    /// before a unit can spawn or a new building can be placed nearby.
    pub fn exclusion_radius(&self) -> f32 {
        match self {
            BuildingType::Queen          => 42.0,  // collision 12 + 30 buffer
            BuildingType::Nursery        => 38.0,  // collision  8 + 30 buffer
            BuildingType::WarriorChamber => 40.0,  // collision 10 + 30 buffer
        }
    }

    /// Nectar cost to place this building.
    pub fn build_cost_nectar(&self) -> f32 {
        match self {
            BuildingType::Queen          => 200.0,
            BuildingType::Nursery        => 75.0,
            BuildingType::WarriorChamber => 120.0,
        }
    }
}

impl UnitType {
    pub fn display_name(&self) -> &'static str {
        match self {
            UnitType::Fourmi => "Fourmi",
            UnitType::Bee => "Bee",
            UnitType::CairnsBirdwing => "Cairns Birdwing",
            UnitType::Dragonfly => "Dragonfly",
            UnitType::RolyPoly => "Roly Poly",
            UnitType::Scorpion => "Scorpion",
            UnitType::GoliathBirdeater => "Goliath Birdeater",
            UnitType::RhinoBeetle => "Rhino Beetle",
        }
    }

    pub fn is_worker(&self) -> bool {
        matches!(self, UnitType::Fourmi | UnitType::Bee)
    }

    /// The building type required to produce this unit.
    pub fn required_building(&self) -> BuildingType {
        match self {
            UnitType::Fourmi | UnitType::Bee => BuildingType::Queen,
            UnitType::CairnsBirdwing | UnitType::Dragonfly => BuildingType::Nursery,
            UnitType::RolyPoly | UnitType::Scorpion | UnitType::GoliathBirdeater | UnitType::RhinoBeetle => BuildingType::WarriorChamber,
        }
    }

    /// Nectar cost to produce this unit.
    pub fn build_cost_nectar(&self) -> f32 {
        if self.is_worker() { 20.0 } else { 30.0 }
    }
}
