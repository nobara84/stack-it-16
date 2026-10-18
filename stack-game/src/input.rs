use macroquad::prelude::*;
use stack_core::{Action, GamePhase};

use crate::arcade::{NEXT_BUTTON, RETURN_BUTTON};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArcadeGameOverAction {
    NextAttempt,
    ReturnToRadio,
}

/// Translate device input into game actions. The simulation never reads a
/// keyboard or pointer directly, leaving room for a future gamepad mapper.
pub fn poll_actions(phase: GamePhase) -> Vec<Action> {
    let mut actions = Vec::with_capacity(2);

    if is_key_pressed(KeyCode::Escape) {
        actions.push(Action::PauseToggle);
    }

    match phase {
        GamePhase::TitleScreen => {
            if is_key_pressed(KeyCode::Space)
                || is_key_pressed(KeyCode::Enter)
                || is_mouse_button_pressed(MouseButton::Left)
                || touch_started()
            {
                actions.push(Action::Start);
            }
        }
        GamePhase::Playing => {
            if is_key_pressed(KeyCode::Space)
                || is_key_pressed(KeyCode::Enter)
                || is_mouse_button_pressed(MouseButton::Left)
                || touch_started()
            {
                actions.push(Action::Place);
            }
        }
        GamePhase::GameOver => {
            if is_key_pressed(KeyCode::R) {
                actions.push(Action::Restart);
            }
        }
        GamePhase::Paused => {}
    }

    actions
}

/// Toggle the diagnostic collision overlay. It is deliberately separate from
/// gameplay actions so the core simulation remains renderer-independent.
pub fn debug_overlay_pressed() -> bool {
    is_key_pressed(KeyCode::F3)
}

/// Skip only the renderer-side death replay. This is deliberately polled
/// separately from gameplay actions so the placement input that caused the
/// replay cannot also skip it in the same frame.
pub fn replay_skip_pressed() -> bool {
    is_key_pressed(KeyCode::Space)
        || is_key_pressed(KeyCode::Enter)
        || is_mouse_button_pressed(MouseButton::Left)
        || touch_started()
}

/// Poll the large, screen-space arcade buttons shown after a failed attempt.
/// Keyboard shortcuts remain useful on a desktop, while pointer coordinates
/// are converted to the same 320x224 logical viewport used by the renderer.
pub fn arcade_game_over_action(attempts_remaining: u8) -> Option<ArcadeGameOverAction> {
    if attempts_remaining > 0
        && (is_key_pressed(KeyCode::Space)
            || is_key_pressed(KeyCode::Enter)
            || is_key_pressed(KeyCode::R))
    {
        return Some(ArcadeGameOverAction::NextAttempt);
    }
    if is_key_pressed(KeyCode::Escape) {
        return Some(ArcadeGameOverAction::ReturnToRadio);
    }

    let pointer = if is_mouse_button_pressed(MouseButton::Left) {
        Some(mouse_position())
    } else {
        touches()
            .iter()
            .find(|touch| touch.phase == TouchPhase::Started)
            .map(|touch| (touch.position.x, touch.position.y))
    }?;
    let logical = logical_pointer_position(pointer)?;
    if attempts_remaining > 0 && in_rect(logical, NEXT_BUTTON) {
        Some(ArcadeGameOverAction::NextAttempt)
    } else if in_rect(logical, RETURN_BUTTON) {
        Some(ArcadeGameOverAction::ReturnToRadio)
    } else {
        None
    }
}

fn touch_started() -> bool {
    touches()
        .iter()
        .any(|touch| touch.phase == TouchPhase::Started)
}

fn logical_pointer_position(pointer: (f32, f32)) -> Option<(f32, f32)> {
    let scale = (screen_width() / 320.0)
        .min(screen_height() / 224.0)
        .floor();
    if scale < 1.0 {
        return None;
    }
    let viewport_width = 320.0 * scale;
    let viewport_height = 224.0 * scale;
    let left = (screen_width() - viewport_width) * 0.5;
    let top = (screen_height() - viewport_height) * 0.5;
    Some(((pointer.0 - left) / scale, (pointer.1 - top) / scale))
}

fn in_rect(point: (f32, f32), rect: (f32, f32, f32, f32)) -> bool {
    point.0 >= rect.0
        && point.0 <= rect.0 + rect.2
        && point.1 >= rect.1
        && point.1 <= rect.1 + rect.3
}
