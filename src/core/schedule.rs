use bevy::prelude::*;

/// Update schedule phases. RtsUpdate runs first and flushes before AiGoals reads state.
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub enum GameSet {
    RtsUpdate,
    AiGoals,
}
