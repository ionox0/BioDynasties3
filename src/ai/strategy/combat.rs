//! AI combat goal generation.
//!
//! Future home for attack-order, patrol, and engage goals for AI military units.

use bevy::prelude::*;
use crate::ai::goals::types::GlobalGoalManager;

/// Generates attack-order goals for AI military units.
/// TODO: implement attack/patrol/engage logic here.
pub fn combat_goal_system(
    _goals: ResMut<GlobalGoalManager>,
) {
}
