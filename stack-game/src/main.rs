mod arcade;
mod background;
mod camera;
mod input;
mod persistence;
mod renderer;
mod replay;
mod text;

use macroquad::miniquad::conf::{LinuxBackend, Platform};
use macroquad::prelude::*;
use replay::{DeathReplay, FailedPlacement, FailureReason, MovementHistory};
use stack_core::{overlap, update_high_score, Action, Game, GameConfig, GameEvent};

const LOGICAL_WIDTH: f32 = 320.0;
const LOGICAL_HEIGHT: f32 = 224.0;

fn window_conf() -> Conf {
    Conf {
        window_title: "Stack-it-16".to_owned(),
        window_width: 960,
        window_height: 672,
        window_resizable: true,
        high_dpi: false,
        sample_count: 1,
        platform: Platform {
            linux_backend: LinuxBackend::WaylandWithX11Fallback,
            ..Default::default()
        },
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    let mut game = Game::new(GameConfig::default());
    let mut arcade = arcade::ArcadeClient::from_env();
    let background = background::Background::load().await;
    let mut high_score = persistence::load_high_score();
    let mut perfect_timer = 0.0;
    let mut perfect_pulse = 0.0;
    let mut elapsed = 0.0;
    let mut camera_target_y = camera::INITIAL_TARGET_Y;
    let mut falling_fragments = Vec::new();
    let mut debug_overlay = false;
    let mut movement_history = MovementHistory::new();
    let mut death_replay: Option<DeathReplay> = None;

    'game_loop: loop {
        let dt = get_frame_time().min(0.05);
        if let Some(replay) = death_replay.as_mut() {
            if input::replay_skip_pressed() {
                replay.skip();
            } else {
                replay.update(dt);
            }
            if replay.is_complete() {
                death_replay = None;
            }
        } else {
            elapsed += dt;

            if input::debug_overlay_pressed() {
                debug_overlay = !debug_overlay;
            }

            let mut failed_placement = None;
            let actions = if game.phase() == stack_core::GamePhase::GameOver && arcade.enabled() {
                match arcade
                    .attempts_remaining()
                    .and_then(|remaining| input::arcade_game_over_action(remaining))
                {
                    Some(input::ArcadeGameOverAction::NextAttempt) => vec![Action::Restart],
                    Some(input::ArcadeGameOverAction::ReturnToRadio) => {
                        arcade.voluntary_exit();
                        break 'game_loop;
                    }
                    None => Vec::new(),
                }
            } else {
                input::poll_actions(game.phase())
            };
            for action in actions {
                if let Some(outcome) = apply_action(
                    &mut game,
                    action,
                    elapsed,
                    &mut high_score,
                    &mut falling_fragments,
                    &mut movement_history,
                ) {
                    if matches!(outcome.event, GameEvent::Started | GameEvent::Restarted) {
                        camera_target_y = camera::desired_target_y(&game);
                        falling_fragments.clear();
                        movement_history.clear();
                        arcade.begin_attempt();
                    }
                    if matches!(outcome.event, GameEvent::Placed { .. }) {
                        arcade.successful_placement(game.placements());
                    }
                    if matches!(outcome.event, GameEvent::Placed { perfect: true, .. }) {
                        perfect_timer = 1.1;
                        perfect_pulse = 0.0;
                    }
                    if outcome.failure.is_some() {
                        arcade.finish_attempt(game.placements());
                        failed_placement = outcome.failure;
                        break;
                    }
                }
            }

            if let Some(failure) = failed_placement {
                // Capture the already-followed gameplay camera at the exact
                // placement frame. Death Replay must reuse this transform;
                // it must not compose a new close-up shot.
                let mut replay = DeathReplay::new(movement_history.snapshot(), failure);
                replay.set_camera_target_y(camera_target_y);
                death_replay = Some(replay);
            } else {
                game.update(dt);
                camera_target_y = camera::follow(camera_target_y, &game, dt);
                falling_fragments.retain_mut(|fragment| fragment.update(dt));
                perfect_timer = (perfect_timer - dt).max(0.0);
                perfect_pulse += dt;
                movement_history.record_active(&game, elapsed);
            }
        }

        clear_background(BLACK);
        let viewport = logical_viewport();
        let world_target_y = death_replay
            .as_ref()
            .map_or(camera_target_y, DeathReplay::camera_target_y);
        set_camera(&logical_camera(viewport, world_target_y));

        background.draw(elapsed, world_target_y, game.placements() as usize);
        if let Some(replay) = death_replay.as_ref() {
            renderer::draw_death_replay(&game, replay);
        } else {
            renderer::draw_game(&game, elapsed, &falling_fragments, debug_overlay);
        }

        // The HUD uses a fixed logical camera so it remains screen-anchored
        // while the world camera follows the tower upward.
        set_camera(&logical_camera(viewport, camera::INITIAL_TARGET_Y));
        renderer::draw_screen_ui(
            &game,
            high_score,
            perfect_timer,
            perfect_pulse,
            debug_overlay,
            death_replay.is_none(),
            arcade.attempts_remaining(),
        );
        if let Some(replay) = death_replay.as_ref() {
            renderer::draw_death_replay_ui(replay);
        }
        set_default_camera();
        next_frame().await;
    }
}

struct ActionOutcome {
    event: GameEvent,
    failure: Option<FailedPlacement>,
}

fn apply_action(
    game: &mut Game,
    action: Action,
    timestamp: f32,
    high_score: &mut u32,
    falling_fragments: &mut Vec<renderer::FallingFragment>,
    movement_history: &mut MovementHistory,
) -> Option<ActionOutcome> {
    let moving_before = game.active_block();
    let previous_before = game.layers().last().copied();
    let layer_index = game.layers().len();
    if action == Action::Place {
        if let Some(moving) = moving_before {
            movement_history.record(replay::ReplaySample {
                block: moving.block,
                velocity: moving.velocity,
                timestamp,
            });
        }
    }
    let event = game.handle_action(action);
    let mut failure = None;
    if let GameEvent::GameOver { score } = event {
        log_failed_placement(game, moving_before, previous_before);
        failure = failed_placement(game, moving_before, previous_before);
        let (updated, changed) = update_high_score(*high_score, score);
        *high_score = updated;
        if changed {
            persistence::save_high_score(score);
        }
    }
    if event == GameEvent::Ignored {
        None
    } else {
        if matches!(event, GameEvent::Placed { perfect: false, .. }) {
            if let (Some(moving), Some(previous)) = (moving_before, previous_before) {
                if let Some(fragment) =
                    renderer::FallingFragment::from_cutoff(moving.block, previous, layer_index)
                {
                    falling_fragments.push(fragment);
                }
            }
        }
        Some(ActionOutcome { event, failure })
    }
}

fn failed_placement(
    game: &Game,
    moving_before: Option<stack_core::MovingBlock>,
    previous_before: Option<stack_core::Block>,
) -> Option<FailedPlacement> {
    let moving = moving_before?;
    let previous = previous_before?;
    let intersection = overlap(moving.block, previous);
    let config = game.config();
    let reason = match intersection {
        None => FailureReason::ZeroOverlap,
        Some(value) if value.width < config.minimum_width => FailureReason::BelowMinimumWidth,
        Some(_) => return None,
    };
    Some(FailedPlacement {
        active: moving.block,
        previous,
        overlap: intersection,
        minimum_width: config.minimum_width,
        reason,
    })
}

fn log_failed_placement(
    game: &Game,
    moving_before: Option<stack_core::MovingBlock>,
    previous_before: Option<stack_core::Block>,
) {
    let Some(moving) = moving_before else {
        eprintln!("[stack-it-16] placement failed: active block was unavailable");
        return;
    };
    let Some(previous) = previous_before else {
        eprintln!("[stack-it-16] placement failed: previous layer was unavailable");
        return;
    };

    let intersection = overlap(moving.block, previous);
    let config = game.config();
    let failure_reason = match intersection {
        None => "zero_overlap",
        Some(value) if value.width < config.minimum_width => "overlap_below_minimum_width",
        Some(_) => "unknown_placement_failure",
    };
    let overlap_text = match intersection {
        Some(value) => format!(
            "left={:.3} right={:.3} width={:.3}",
            value.left,
            value.right(),
            value.width
        ),
        None => "none".to_owned(),
    };
    eprintln!(
        "[stack-it-16] placement failed at level {}: reason={}; active left={:.3} right={:.3} width={:.3} velocity={:.3}; previous left={:.3} right={:.3} width={:.3}; overlap={}; offset={:.3}; tolerance={:.3}; minimum_width={:.3}",
        game.placements() + 1,
        failure_reason,
        moving.block.left,
        moving.block.right(),
        moving.block.width,
        moving.velocity,
        previous.left,
        previous.right(),
        previous.width,
        overlap_text,
        (moving.block.left - previous.left).abs(),
        config.perfect_tolerance,
        config.minimum_width,
    );
}

fn logical_camera(viewport: (i32, i32, i32, i32), target_y: f32) -> Camera2D {
    Camera2D {
        target: vec2(LOGICAL_WIDTH / 2.0, target_y),
        // Macroquad's screen render pass applies its own Y inversion when a
        // Camera2D has no render target. Positive Y preserves our logical
        // top-left coordinate system for both world and HUD drawing.
        zoom: vec2(2.0 / LOGICAL_WIDTH, 2.0 / LOGICAL_HEIGHT),
        viewport: Some(viewport),
        ..Default::default()
    }
}

fn logical_viewport() -> (i32, i32, i32, i32) {
    let scale = (screen_width() / LOGICAL_WIDTH)
        .min(screen_height() / LOGICAL_HEIGHT)
        .floor()
        .max(1.0);
    let width = (LOGICAL_WIDTH * scale) as i32;
    let height = (LOGICAL_HEIGHT * scale) as i32;
    (
        ((screen_width() as i32 - width) / 2).max(0),
        ((screen_height() as i32 - height) / 2).max(0),
        width,
        height,
    )
}
