use bevy::prelude::*;
use crate::ai::goals::types::{GoalQueueSnapshot, UnifiedGoal};

#[derive(Component)]
struct GoalsDebugPanel;

#[derive(Component)]
struct GoalsDebugText;

pub struct AIGoalsDebugPlugin;

impl Plugin for AIGoalsDebugPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_goals_debug_panel)
            .add_systems(Update, update_goals_debug);
    }
}

fn setup_goals_debug_panel(mut commands: Commands) {
    commands
        .spawn((
            GoalsDebugPanel,
            Node {
                position_type: PositionType::Absolute,
                bottom: Val::Px(8.0),
                left: Val::Px(8.0),
                padding: UiRect::all(Val::Px(8.0)),
                flex_direction: FlexDirection::Column,
                max_width: Val::Px(340.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.05, 0.05, 0.05, 0.85)),
            ZIndex(900),
        ))
        .with_children(|p| {
            p.spawn((
                GoalsDebugText,
                Text::new("AI Goals\n(waiting...)"),
                TextFont {
                    font_size: 11.0,
                    ..default()
                },
                TextColor(Color::srgb(0.7, 0.9, 0.6)),
            ));
        });
}

fn update_goals_debug(
    snapshot: Res<GoalQueueSnapshot>,
    mut text_q: Query<&mut Text, With<GoalsDebugText>>,
) {
    if !snapshot.is_changed() {
        return;
    }
    let Ok(mut text) = text_q.get_single_mut() else {
        return;
    };
    if snapshot.goals.is_empty() {
        **text = "AI Goals\n(empty)".to_string();
    } else {
        let lines: String = snapshot
            .goals
            .iter()
            .map(|pg| format!("{:.2}  {}\n", pg.priority, format_goal(&pg.goal)))
            .collect();
        **text = format!(
            "AI Goals ({})\n{}",
            snapshot.goals.len(),
            lines.trim_end()
        );
    }
}

fn format_goal(goal: &UnifiedGoal) -> String {
    match goal {
        UnifiedGoal::AssignWorkerToResource { resource_type, .. } => {
            format!("Gather({resource_type:?})")
        }
        UnifiedGoal::BuildUnit {
            unit_type,
            player_id,
            ..
        } => format!("BuildUnit({unit_type:?}) p{player_id}"),
        UnifiedGoal::AttackTarget { .. } => "Attack".to_string(),
        UnifiedGoal::BuildBuilding {
            building_type,
            player_id,
            ..
        } => format!("Build({building_type:?}) p{player_id}"),
    }
}
