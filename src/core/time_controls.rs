use bevy::prelude::*;

/// Plugin for time control systems (fast-forward, pause, etc.)
pub struct TimeControlPlugin;

impl Plugin for TimeControlPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TimeControlSettings>()
            .add_systems(Update, time_control_input_system);
    }
}

/// Resource to track current time control settings
#[derive(Resource, Debug)]
pub struct TimeControlSettings {
    pub current_speed: f32,
    pub available_speeds: Vec<f32>,
    pub speed_index: usize,
}

impl Default for TimeControlSettings {
    fn default() -> Self {
        Self {
            current_speed: 1.0,
            available_speeds: vec![0.5, 1.0, 2.0, 4.0, 8.0, 16.0], // 0.5x to 16x speed
            speed_index: 1,                                        // Start at 1.0x (normal speed)
        }
    }
}

impl TimeControlSettings {
    /// Increase time speed to next level
    pub fn speed_up(&mut self) -> f32 {
        if self.speed_index < self.available_speeds.len() - 1 {
            self.speed_index += 1;
            self.current_speed = self.available_speeds[self.speed_index];
        }
        self.current_speed
    }

    /// Decrease time speed to previous level
    pub fn slow_down(&mut self) -> f32 {
        if self.speed_index > 0 {
            self.speed_index -= 1;
            self.current_speed = self.available_speeds[self.speed_index];
        }
        self.current_speed
    }

    /// Reset to normal speed (1.0x)
    pub fn reset_speed(&mut self) -> f32 {
        self.speed_index = 1;
        self.current_speed = self.available_speeds[self.speed_index];
        self.current_speed
    }

    /// Get current speed as a formatted string
    pub fn speed_display(&self) -> String {
        if self.current_speed == 1.0 {
            "Normal".to_string()
        } else if self.current_speed < 1.0 {
            format!("{:.1}x Slow", self.current_speed)
        } else {
            format!("{}x Fast", self.current_speed as u32)
        }
    }
}

/// System to handle time control input
pub fn time_control_input_system(
    mut time_controls: ResMut<TimeControlSettings>,
    mut time: ResMut<Time<Virtual>>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
) {
    let mut speed_changed = false;

    // Plus/Equal key: Speed up
    if keyboard_input.just_pressed(KeyCode::Equal)
        || keyboard_input.just_pressed(KeyCode::NumpadAdd)
    {
        time_controls.speed_up();
        speed_changed = true;
        info!(
            "⚡ Time speed increased to {}",
            time_controls.speed_display()
        );
    }

    // Minus key: Slow down
    if keyboard_input.just_pressed(KeyCode::Minus)
        || keyboard_input.just_pressed(KeyCode::NumpadSubtract)
    {
        time_controls.slow_down();
        speed_changed = true;
        info!(
            "🐌 Time speed decreased to {}",
            time_controls.speed_display()
        );
    }

    // Backspace: Reset to normal speed
    if keyboard_input.just_pressed(KeyCode::Backspace) {
        time_controls.reset_speed();
        speed_changed = true;

    }

    // Apply speed change to Bevy's virtual time
    if speed_changed {
        time.set_relative_speed(time_controls.current_speed);

        // Debug info about what systems are affected

    }
}
