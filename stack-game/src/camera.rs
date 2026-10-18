use stack_core::Game;

pub const INITIAL_TARGET_Y: f32 = 112.0;
pub const TOWER_BASE_Y: f32 = 174.0;
pub const LAYER_STEP: f32 = 18.0;

const ACTIVE_TOP_SCREEN_Y: f32 = 72.0;
const MIN_ACTIVE_SCREEN_Y: f32 = 28.0;
const FOLLOW_RESPONSE: f32 = 8.0;

/// Return the world-space top edge of the next active layer.
pub fn active_world_y(game: &Game) -> f32 {
    TOWER_BASE_Y - game.layers().len() as f32 * LAYER_STEP
}

/// Follow only once the active play area enters the upper safety margin.
/// Keeping the initial target as an upper bound means the base tower never
/// drifts downward while the first few slabs are being placed.
pub fn desired_target_y(game: &Game) -> f32 {
    desired_target_y_for_world(active_world_y(game))
}

/// Convert a logical world Y coordinate into the top-left logical screen
/// coordinate used by the game's Camera2D setup.
#[cfg(test)]
pub fn world_to_screen_y(world_y: f32, target_y: f32) -> f32 {
    world_y - target_y + INITIAL_TARGET_Y
}

fn desired_target_y_for_world(active_y: f32) -> f32 {
    (active_y - ACTIVE_TOP_SCREEN_Y + INITIAL_TARGET_Y).min(INITIAL_TARGET_Y)
}

/// Smoothly approach the desired target without frame-rate-dependent jumps.
pub fn follow(current_target_y: f32, game: &Game, elapsed_seconds: f32) -> f32 {
    let desired = desired_target_y(game);
    let next = follow_target(current_target_y, desired, elapsed_seconds);
    let visibility_limit = active_world_y(game) + INITIAL_TARGET_Y - MIN_ACTIVE_SCREEN_Y;
    let next = next.min(visibility_limit);
    if (next - desired).abs() < 0.05 {
        desired
    } else {
        next
    }
}

fn follow_target(current_target_y: f32, desired_target_y: f32, elapsed_seconds: f32) -> f32 {
    let response = 1.0 - (-FOLLOW_RESPONSE * elapsed_seconds.max(0.0)).exp();
    current_target_y + (desired_target_y - current_target_y) * response
}

#[cfg(test)]
mod tests {
    use super::*;
    use stack_core::{GameConfig, GamePhase};

    #[test]
    fn camera_stays_still_for_short_towers() {
        let game = Game::new(GameConfig::default());
        assert_eq!(game.phase(), GamePhase::TitleScreen);
        assert_eq!(desired_target_y(&game), INITIAL_TARGET_Y);
    }

    #[test]
    fn camera_target_moves_up_as_layers_grow() {
        let initial = desired_target_y_for_world(TOWER_BASE_Y - LAYER_STEP);
        let tall = desired_target_y_for_world(TOWER_BASE_Y - 9.0 * LAYER_STEP);

        assert!(tall < initial);
    }

    #[test]
    fn world_to_screen_transform_keeps_the_active_top_in_the_safe_band() {
        for layers in [1, 10, 77, 200, 1_000, 5_000] {
            let world_y = TOWER_BASE_Y - layers as f32 * LAYER_STEP;
            let target_y = desired_target_y_for_world(world_y);
            let screen_y = world_to_screen_y(world_y, target_y);

            assert!(screen_y.is_finite());
            if world_y < ACTIVE_TOP_SCREEN_Y {
                assert!((MIN_ACTIVE_SCREEN_Y..=ACTIVE_TOP_SCREEN_Y).contains(&screen_y));
            } else {
                // The camera intentionally remains at its initial target for
                // short towers, so the active layer may sit lower on screen.
                assert_eq!(screen_y, world_y);
                assert!(screen_y > ACTIVE_TOP_SCREEN_Y);
            }
        }
    }

    #[test]
    fn smooth_camera_follow_is_monotonic_and_frame_rate_stable() {
        let mut target_30 = INITIAL_TARGET_Y;
        let mut target_120 = INITIAL_TARGET_Y;
        let desired = -500.0;

        for _ in 0..240 {
            let previous_30 = target_30;
            let previous_120 = target_120;
            target_30 = follow_target(target_30, desired, 1.0 / 30.0);
            target_120 = follow_target(target_120, desired, 1.0 / 120.0);
            assert!(target_30 <= INITIAL_TARGET_Y);
            assert!(target_120 <= INITIAL_TARGET_Y);
            assert!(target_30 <= previous_30);
            assert!(target_120 <= previous_120);
        }

        assert!((target_30 - desired).abs() < 0.05);
        assert!((target_120 - desired).abs() < 0.05);
    }
}
