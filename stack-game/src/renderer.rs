use macroquad::prelude::*;
use stack_core::{overlap, Block, Game, GamePhase};

use crate::arcade::{NEXT_BUTTON, RETURN_BUTTON};
use crate::replay::{DeathReplay, FailureReason};
use crate::text;

const OUTLINE: Color = Color::new(0.055, 0.035, 0.14, 1.0);
const GOLD: Color = Color::new(1.0, 0.82, 0.08, 1.0);
const HUD_WHITE: Color = Color::new(1.0, 0.98, 0.91, 1.0);
const BUILD_IDENTIFIER: &str = "V0.1.0";

const TITLE_TEXT: &str = "STACK-IT-16";
const TITLE_SUBTITLE: &str = "EIN PIXEL-ARCADE-SPIEL";
const TITLE_START: &str = "LEERTASTE ZUM STARTEN";
const HUD_SCORE: &str = "PUNKTE";
const HUD_HIGH_SCORE: &str = "REKORD";
const PERFECT_TEXT: &str = "PERFEKT +1";
const PAUSE_TEXT: &str = "PAUSE";
const PAUSE_HINT: &str = "ESC ZUM FORTSETZEN";
const GAME_OVER_TEXT: &str = "SPIEL VORBEI";
const RESTART_TEXT: &str = "R ZUM NEUSTART";
const NEXT_ATTEMPT_TEXT: &str = "NÄCHSTER VERSUCH";
const RETURN_TO_RADIO_TEXT: &str = "ZURÜCK ZUM RADIO";
const ATTEMPTS_LEFT_TEXT: &str = "VERSUCHE ÜBRIG";
const ALL_ATTEMPTS_USED_TEXT: &str = "ALLE VERSUCHE VERBRAUCHT";
const REPLAY_TEXT: &str = "WIEDERHOLUNG";
const MISS_TEXT: &str = "DANEBEN!";

const TITLE_Y: f32 = 25.0;
const TITLE_SCALE: f32 = 2.5;
const SUBTITLE_Y: f32 = 49.0;
const START_Y: f32 = 207.0;
const START_SCALE: f32 = 1.0;

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq)]
struct LayoutRect {
    left: f32,
    top: f32,
    right: f32,
    bottom: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct BlockColors {
    top: Color,
    left: Color,
    right: Color,
}

#[derive(Debug, Clone, Copy)]
struct PaletteStop {
    layer: usize,
    colors: BlockColors,
}

const fn block_colors(top: Color, left: Color, right: Color) -> BlockColors {
    BlockColors { top, left, right }
}

const IMPACT_COLORS: BlockColors = block_colors(
    Color::new(1.0, 0.14, 0.18, 1.0),
    Color::new(0.58, 0.015, 0.06, 1.0),
    Color::new(0.86, 0.04, 0.08, 1.0),
);
const LANDING_GHOST_COLORS: BlockColors = block_colors(
    Color::new(1.0, 0.86, 0.20, 0.26),
    Color::new(0.72, 0.18, 0.18, 0.18),
    Color::new(1.0, 0.42, 0.08, 0.22),
);
const MISS_MARKER: Color = Color::new(1.0, 0.16, 0.18, 0.95);

// Isometric geometry is a fixed visual depth, not a simulated movement axis.
// Falling entities change only their world Y coordinate; these values therefore
// remain unchanged throughout a fall.
const BLOCK_DEPTH: f32 = 11.0;
const BLOCK_RISE: f32 = 5.0;
const BLOCK_SIDE_HEIGHT: f32 = 18.0;
const FALL_GRAVITY_Y: f32 = 220.0;
const INITIAL_FRAGMENT_VELOCITY_Y: f32 = 12.0;

// The primary ramp deliberately uses 15-layer bands, but interpolates between
// them so adjacent layers never jump from one flat swatch to another.
const PRIMARY_STOPS: [PaletteStop; 11] = [
    PaletteStop {
        layer: 0,
        colors: block_colors(
            Color::new(0.34, 0.04, 0.54, 1.0),
            Color::new(0.15, 0.01, 0.28, 1.0),
            Color::new(0.25, 0.02, 0.40, 1.0),
        ),
    },
    PaletteStop {
        layer: 15,
        colors: block_colors(
            Color::new(0.46, 0.05, 0.66, 1.0),
            Color::new(0.21, 0.01, 0.36, 1.0),
            Color::new(0.34, 0.03, 0.50, 1.0),
        ),
    },
    PaletteStop {
        layer: 30,
        colors: block_colors(
            Color::new(0.62, 0.07, 0.78, 1.0),
            Color::new(0.29, 0.02, 0.45, 1.0),
            Color::new(0.46, 0.04, 0.61, 1.0),
        ),
    },
    PaletteStop {
        layer: 45,
        colors: block_colors(
            Color::new(0.78, 0.07, 0.72, 1.0),
            Color::new(0.40, 0.02, 0.39, 1.0),
            Color::new(0.59, 0.03, 0.56, 1.0),
        ),
    },
    PaletteStop {
        layer: 60,
        colors: block_colors(
            Color::new(0.91, 0.09, 0.64, 1.0),
            Color::new(0.50, 0.02, 0.33, 1.0),
            Color::new(0.70, 0.04, 0.49, 1.0),
        ),
    },
    PaletteStop {
        layer: 75,
        colors: block_colors(
            Color::new(1.0, 0.16, 0.58, 1.0),
            Color::new(0.59, 0.02, 0.30, 1.0),
            Color::new(0.80, 0.04, 0.45, 1.0),
        ),
    },
    PaletteStop {
        layer: 90,
        colors: block_colors(
            Color::new(1.0, 0.18, 0.42, 1.0),
            Color::new(0.64, 0.02, 0.20, 1.0),
            Color::new(0.84, 0.05, 0.31, 1.0),
        ),
    },
    PaletteStop {
        layer: 105,
        colors: block_colors(
            Color::new(1.0, 0.32, 0.27, 1.0),
            Color::new(0.70, 0.08, 0.12, 1.0),
            Color::new(0.90, 0.18, 0.18, 1.0),
        ),
    },
    PaletteStop {
        layer: 120,
        colors: block_colors(
            Color::new(1.0, 0.48, 0.17, 1.0),
            Color::new(0.75, 0.16, 0.05, 1.0),
            Color::new(0.93, 0.29, 0.07, 1.0),
        ),
    },
    PaletteStop {
        layer: 135,
        colors: block_colors(
            Color::new(1.0, 0.62, 0.13, 1.0),
            Color::new(0.79, 0.27, 0.03, 1.0),
            Color::new(0.95, 0.43, 0.05, 1.0),
        ),
    },
    PaletteStop {
        layer: 150,
        colors: block_colors(
            Color::new(1.0, 0.78, 0.20, 1.0),
            Color::new(0.82, 0.40, 0.04, 1.0),
            Color::new(0.96, 0.58, 0.07, 1.0),
        ),
    },
];

// Above layer 150 the palette remains bright, but takes a very slow secondary
// tour through pale gold, apricot, and light coral. The final stop matches the
// first stop, making the long cycle continuous rather than a hard reset.
const SECONDARY_STOPS: [PaletteStop; 5] = [
    PaletteStop {
        layer: 0,
        colors: block_colors(
            Color::new(1.0, 0.78, 0.20, 1.0),
            Color::new(0.82, 0.40, 0.04, 1.0),
            Color::new(0.96, 0.58, 0.07, 1.0),
        ),
    },
    PaletteStop {
        layer: 150,
        colors: block_colors(
            Color::new(1.0, 0.86, 0.36, 1.0),
            Color::new(0.86, 0.51, 0.10, 1.0),
            Color::new(0.98, 0.68, 0.17, 1.0),
        ),
    },
    PaletteStop {
        layer: 300,
        colors: block_colors(
            Color::new(1.0, 0.78, 0.42, 1.0),
            Color::new(0.84, 0.43, 0.17, 1.0),
            Color::new(0.95, 0.58, 0.29, 1.0),
        ),
    },
    PaletteStop {
        layer: 450,
        colors: block_colors(
            Color::new(1.0, 0.70, 0.45, 1.0),
            Color::new(0.82, 0.35, 0.24, 1.0),
            Color::new(0.94, 0.50, 0.35, 1.0),
        ),
    },
    PaletteStop {
        layer: 600,
        colors: block_colors(
            Color::new(1.0, 0.78, 0.20, 1.0),
            Color::new(0.82, 0.40, 0.04, 1.0),
            Color::new(0.96, 0.58, 0.07, 1.0),
        ),
    },
];

#[derive(Debug, Clone, Copy)]
pub struct FallingFragment {
    block: Block,
    world_y: f32,
    velocity_y: f32,
    age: f32,
    lifetime: f32,
    colors: BlockColors,
}

impl FallingFragment {
    pub fn from_cutoff(moving: Block, previous: Block, layer_index: usize) -> Option<Self> {
        let block = if moving.left < previous.left {
            Block::new(moving.left, previous.left - moving.left)
        } else if moving.right() > previous.right() {
            Block::new(previous.right(), moving.right() - previous.right())
        } else {
            return None;
        };

        Some(Self {
            block,
            world_y: crate::camera::TOWER_BASE_Y - layer_index as f32 * crate::camera::LAYER_STEP,
            velocity_y: INITIAL_FRAGMENT_VELOCITY_Y,
            age: 0.0,
            lifetime: 1.25,
            colors: palette_colors(layer_index),
        })
    }

    pub fn update(&mut self, elapsed_seconds: f32) -> bool {
        let dt = elapsed_seconds.max(0.0);
        self.age += dt;
        self.world_y += self.velocity_y * dt;
        self.velocity_y += FALL_GRAVITY_Y * dt;
        self.age < self.lifetime && self.world_y < 280.0
    }
}

pub fn draw_game(
    game: &Game,
    time: f32,
    falling_fragments: &[FallingFragment],
    debug_overlay: bool,
) {
    match game.phase() {
        GamePhase::TitleScreen => draw_title(time),
        GamePhase::Playing | GamePhase::Paused | GamePhase::GameOver => {
            draw_tower(game);
            for fragment in falling_fragments {
                draw_block(fragment.block, fragment.world_y, fragment.colors);
            }
            if debug_overlay {
                draw_collision_bounds(game);
            }
        }
    }
}

pub fn draw_death_replay(game: &Game, replay: &DeathReplay) {
    // Replay the same world scene as gameplay. The placed layers are not part
    // of the replay simulation, so they remain stationary while only the
    // failed active block is sampled from DeathReplay.
    draw_placed_layers(game);
    let active_y = crate::camera::active_world_y(game);
    if replay.show_miss_target() {
        draw_landing_ghost(replay.previous_block(), active_y);
        draw_miss_marker(replay, active_y);
    }
    let block_y = active_y + replay.fall_offset_y();
    let colors = if replay.is_flashing() {
        IMPACT_COLORS
    } else {
        palette_colors(game.layers().len())
    };
    draw_block(replay.current_block(), block_y, colors);
    if replay.show_miss_feedback() {
        let label_scale = text::fit_scale(MISS_TEXT, 64.0, 1.4, 1.0);
        let label_width = text::width(MISS_TEXT, label_scale);
        let label_x = text::clamp_x(
            replay.current_block().left + replay.current_block().width * 0.5 - label_width * 0.5,
            label_width,
            320.0,
            4.0,
        );
        text::draw(
            MISS_TEXT,
            label_x + 1.0,
            block_y - 15.0,
            label_scale,
            OUTLINE,
        );
        text::draw(MISS_TEXT, label_x, block_y - 16.0, label_scale, GOLD);
    }
}

pub fn draw_screen_ui(
    game: &Game,
    high_score: u32,
    perfect_timer: f32,
    pulse: f32,
    debug_overlay: bool,
    show_game_over: bool,
    arcade_attempts_remaining: Option<u8>,
) {
    draw_build_identifier();
    if game.phase() == GamePhase::TitleScreen {
        return;
    }

    draw_hud(game, high_score, perfect_timer, pulse);
    if debug_overlay && show_game_over {
        draw_debug_panel(game);
    }
    match game.phase() {
        GamePhase::Paused => draw_overlay(PAUSE_TEXT, PAUSE_HINT),
        GamePhase::GameOver if show_game_over => {
            if let Some(remaining) = arcade_attempts_remaining {
                draw_arcade_game_over_overlay(game.score(), high_score, remaining);
            } else {
                draw_game_over_overlay(game.score(), high_score);
            }
        }
        _ => {}
    }
}

pub fn draw_death_replay_ui(replay: &DeathReplay) {
    if replay.is_failure_freeze() {
        draw_centered_fit(MISS_TEXT, 112.0, 2.0, 1.5, GOLD, Some(OUTLINE));
    } else if replay.is_rewinding() {
        draw_centered_fit(REPLAY_TEXT, 39.0, 1.0, 0.8, GOLD, Some(OUTLINE));
    } else if replay.is_slow_motion() {
        let indicator_y = 39.0;
        let indicator_scale = text::fit_scale(REPLAY_TEXT, 300.0, 1.0, 0.8);
        draw_centered(
            REPLAY_TEXT,
            indicator_y,
            indicator_scale,
            GOLD,
            Some(OUTLINE),
        );
        draw_centered_fit(
            "ZEITLUPE",
            indicator_y + text::line_height(indicator_scale) + 2.0,
            0.9,
            0.7,
            HUD_WHITE,
            None,
        );
        draw_rectangle(112.0, 61.0, 96.0, 3.0, OUTLINE);
        draw_rectangle(112.0, 61.0, 96.0 * replay.progress(), 3.0, GOLD);
        draw_rectangle_lines(111.0, 60.0, 98.0, 5.0, 1.0, HUD_WHITE);
    }
}

fn draw_collision_bounds(game: &Game) {
    let Some(active) = game.active_block() else {
        return;
    };
    let Some(previous) = game.layers().last().copied() else {
        return;
    };

    let active_y =
        crate::camera::TOWER_BASE_Y - game.layers().len() as f32 * crate::camera::LAYER_STEP;
    let previous_y = active_y + crate::camera::LAYER_STEP;
    let active_color = Color::new(1.0, 0.22, 0.35, 0.95);
    let previous_color = Color::new(0.25, 0.88, 1.0, 0.95);
    draw_rectangle_lines(
        active.block.left,
        active_y,
        active.block.width,
        18.0,
        1.0,
        active_color,
    );
    draw_rectangle_lines(
        previous.left,
        previous_y,
        previous.width,
        18.0,
        1.0,
        previous_color,
    );
    if let Some(intersection) = overlap(active.block, previous) {
        draw_rectangle_lines(
            intersection.left,
            active_y + 7.0,
            intersection.width,
            4.0,
            1.0,
            Color::new(1.0, 0.86, 0.16, 0.95),
        );
    }
}

fn draw_debug_panel(game: &Game) {
    let Some(active) = game.active_block() else {
        return;
    };
    let Some(previous) = game.layers().last().copied() else {
        return;
    };
    let panel = Color::new(0.035, 0.02, 0.13, 0.94);
    draw_rectangle(4.0, 181.0, 312.0, 39.0, panel);
    draw_rectangle_lines(4.0, 181.0, 312.0, 39.0, 1.0, GOLD);
    text::draw("DIAGNOSE F3", 8.0, 184.0, 1.0, GOLD);
    text::draw(
        &format!(
            "AKT {}-{}",
            format_coord(active.block.left),
            format_coord(active.block.right())
        ),
        8.0,
        194.0,
        1.0,
        HUD_WHITE,
    );
    text::draw(
        &format!(
            "VOR {}-{}",
            format_coord(previous.left),
            format_coord(previous.right())
        ),
        85.0,
        194.0,
        1.0,
        Color::new(0.25, 0.88, 1.0, 1.0),
    );
    let overlap_text = overlap(active.block, previous)
        .map(|value| format_coord(value.width))
        .unwrap_or_else(|| "KEIN".to_owned());
    text::draw(&format!("UEB {}", overlap_text), 163.0, 194.0, 1.0, GOLD);
    text::draw(
        "ROT=AKT BLAU=VOR GOLD=UEB",
        8.0,
        205.0,
        1.0,
        Color::new(0.70, 0.86, 1.0, 1.0),
    );
}

fn format_coord(value: f32) -> String {
    format!("{:03}", value.round() as i32)
}

fn draw_title(_time: f32) {
    let sample = [
        Block::new(79.0, 162.0),
        Block::new(96.0, 128.0),
        Block::new(110.0, 100.0),
    ];
    for (index, block) in sample.iter().enumerate() {
        draw_block(
            *block,
            174.0 - index as f32 * 18.0,
            palette_colors(index + 1),
        );
    }

    draw_centered_fit(TITLE_TEXT, TITLE_Y, TITLE_SCALE, 1.8, GOLD, Some(OUTLINE));
    draw_centered_fit(TITLE_SUBTITLE, SUBTITLE_Y, 1.0, 0.7, HUD_WHITE, None);
    draw_centered_fit(TITLE_START, START_Y, START_SCALE, 0.7, GOLD, Some(OUTLINE));
}

fn draw_tower(game: &Game) {
    draw_placed_layers(game);
    if let Some(active) = game.active_block() {
        let y =
            crate::camera::TOWER_BASE_Y - game.layers().len() as f32 * crate::camera::LAYER_STEP;
        draw_block(active.block, y, palette_colors(game.layers().len()));
    }
}

fn draw_placed_layers(game: &Game) {
    for (index, block) in game.layers().iter().enumerate() {
        let y = crate::camera::TOWER_BASE_Y - index as f32 * crate::camera::LAYER_STEP;
        draw_block(*block, y, palette_colors(index));
    }
}

fn draw_block(block: Block, y: f32, colors: BlockColors) {
    let BlockColors { top, left, right } = colors;
    let width = block.width.max(1.0);
    let depth = BLOCK_DEPTH;
    let rise = BLOCK_RISE;
    let side_height = BLOCK_SIDE_HEIGHT;
    let x0 = block.left;
    let x1 = block.left + width;
    let a = vec2(x0, y);
    let b = vec2(x1, y);
    let c = vec2(x1 + depth, y + rise);
    let d = vec2(x0 + depth, y + rise);
    let down = vec2(0.0, side_height);

    draw_triangle(a, d, d + down, left);
    draw_triangle(a, d + down, a + down, left);
    draw_triangle(d, c, c + down, right);
    draw_triangle(d, c + down, d + down, right);
    draw_triangle(a, b, c, top);
    draw_triangle(a, c, d, top);

    draw_line(a.x, a.y, b.x, b.y, 1.5, OUTLINE);
    draw_line(b.x, b.y, c.x, c.y, 1.5, OUTLINE);
    draw_line(c.x, c.y, d.x, d.y, 1.5, OUTLINE);
    draw_line(d.x, d.y, a.x, a.y, 1.5, OUTLINE);
    draw_line(a.x, a.y, a.x, a.y + side_height, 1.5, OUTLINE);
    draw_line(d.x, d.y, d.x, d.y + side_height, 1.5, OUTLINE);
    draw_line(c.x, c.y, c.x, c.y + side_height, 1.5, OUTLINE);
    draw_line(a.x, a.y + side_height, d.x, d.y + side_height, 1.5, OUTLINE);
    draw_line(d.x, d.y + side_height, c.x, c.y + side_height, 1.5, OUTLINE);
}

fn draw_landing_ghost(block: Block, y: f32) {
    let BlockColors { top, left, right } = LANDING_GHOST_COLORS;
    let width = block.width.max(1.0);
    let depth = BLOCK_DEPTH;
    let rise = BLOCK_RISE;
    let side_height = BLOCK_SIDE_HEIGHT;
    let x0 = block.left;
    let x1 = block.left + width;
    let a = vec2(x0, y);
    let b = vec2(x1, y);
    let c = vec2(x1 + depth, y + rise);
    let d = vec2(x0 + depth, y + rise);
    let down = vec2(0.0, side_height);
    let outline = Color::new(1.0, 0.88, 0.24, 0.68);

    draw_triangle(a, d, d + down, left);
    draw_triangle(a, d + down, a + down, left);
    draw_triangle(d, c, c + down, right);
    draw_triangle(d, c + down, d + down, right);
    draw_triangle(a, b, c, top);
    draw_triangle(a, c, d, top);
    draw_line(a.x, a.y, b.x, b.y, 1.0, outline);
    draw_line(b.x, b.y, c.x, c.y, 1.0, outline);
    draw_line(c.x, c.y, d.x, d.y, 1.0, outline);
    draw_line(d.x, d.y, a.x, a.y, 1.0, outline);
}

fn draw_miss_marker(replay: &DeathReplay, active_y: f32) {
    let active = replay.current_block();
    let previous = replay.previous_block();
    let marker_x = match replay.failure_reason() {
        FailureReason::ZeroOverlap => {
            (active.left + active.width * 0.5 + previous.left + previous.width * 0.5) * 0.5
        }
        FailureReason::BelowMinimumWidth => replay
            .overlap_block()
            .map_or(active.left + active.width * 0.5, |overlap| {
                overlap.left + overlap.width * 0.5
            }),
    };
    text::draw("!", marker_x - 2.0, active_y - 27.0, 1.5, MISS_MARKER);
}

fn draw_hud(game: &Game, high_score: u32, perfect_timer: f32, pulse: f32) {
    let panel = Color::new(0.035, 0.06, 0.24, 0.88);
    draw_rectangle(5.0, 8.0, 86.0, 27.0, panel);
    draw_rectangle(204.0, 8.0, 111.0, 27.0, panel);
    draw_rectangle_lines(5.0, 8.0, 86.0, 27.0, 1.0, Color::new(0.42, 0.72, 1.0, 0.55));
    draw_rectangle_lines(
        204.0,
        8.0,
        111.0,
        27.0,
        1.0,
        Color::new(0.42, 0.72, 1.0, 0.55),
    );

    let score_x = 9.0;
    let high_score_x = text::right_aligned_x(HUD_HIGH_SCORE, 310.0, 1.0);
    let high_value = format_score(high_score);
    let high_value_x = text::right_aligned_x(&high_value, 310.0, 2.0);
    text::draw(
        HUD_SCORE,
        score_x,
        11.0,
        1.0,
        Color::new(0.70, 0.86, 1.0, 1.0),
    );
    text::draw(&format_score(game.score()), score_x, 20.0, 2.0, HUD_WHITE);
    text::draw(
        HUD_HIGH_SCORE,
        high_score_x,
        11.0,
        1.0,
        Color::new(0.70, 0.86, 1.0, 1.0),
    );
    text::draw(&high_value, high_value_x, 20.0, 2.0, HUD_WHITE);

    draw_rectangle(151.0, 14.0, 18.0, 17.0, OUTLINE);
    draw_rectangle(155.0, 16.0, 4.0, 13.0, HUD_WHITE);
    draw_rectangle(162.0, 16.0, 4.0, 13.0, HUD_WHITE);

    if perfect_timer > 0.0 {
        let scale = 1.8 + (pulse * 7.0).sin().abs() * 0.2;
        let width = text::width(PERFECT_TEXT, scale);
        let x = (320.0 - width) / 2.0;
        text::draw(PERFECT_TEXT, x + 1.0, 38.0 + 1.0, scale, OUTLINE);
        text::draw(PERFECT_TEXT, x, 38.0, scale, GOLD);
    }
}

fn draw_build_identifier() {
    let scale = text::fit_scale(BUILD_IDENTIFIER, 58.0, 0.7, 0.5);
    let x = text::right_aligned_x(BUILD_IDENTIFIER, 318.0, scale);
    text::draw(BUILD_IDENTIFIER, x + 1.0, 1.0, scale, OUTLINE);
    text::draw(BUILD_IDENTIFIER, x, 0.0, scale, GOLD);
}

fn draw_overlay(title: &str, hint: &str) {
    draw_rectangle(61.0, 82.0, 198.0, 62.0, Color::new(0.035, 0.02, 0.13, 0.92));
    draw_rectangle_lines(62.0, 83.0, 196.0, 60.0, 1.0, GOLD);
    draw_centered_fit(title, 94.0, 2.0, 1.2, HUD_WHITE, Some(OUTLINE));
    draw_centered_fit(hint, 121.0, 1.0, 0.7, GOLD, None);
}

fn draw_game_over_overlay(score: u32, high_score: u32) {
    draw_rectangle(
        44.0,
        65.0,
        232.0,
        111.0,
        Color::new(0.035, 0.02, 0.13, 0.94),
    );
    draw_rectangle_lines(45.0, 66.0, 230.0, 109.0, 1.0, GOLD);
    draw_centered_fit(GAME_OVER_TEXT, 76.0, 2.0, 1.5, HUD_WHITE, Some(OUTLINE));
    draw_centered_fit(
        &format!("{HUD_SCORE} {}", format_score(score)),
        105.0,
        2.0,
        1.0,
        HUD_WHITE,
        None,
    );
    draw_centered_fit(
        &format!("{HUD_HIGH_SCORE} {}", format_score(high_score)),
        130.0,
        1.0,
        0.8,
        Color::new(0.70, 0.86, 1.0, 1.0),
        None,
    );
    draw_centered_fit(RESTART_TEXT, 158.0, 1.0, 0.7, GOLD, None);
}

fn draw_arcade_game_over_overlay(score: u32, high_score: u32, attempts_remaining: u8) {
    let panel_height = if attempts_remaining > 0 { 169.0 } else { 145.0 };
    let panel_top = if attempts_remaining > 0 { 39.0 } else { 51.0 };
    draw_rectangle(
        16.0,
        panel_top,
        288.0,
        panel_height,
        Color::new(0.035, 0.02, 0.13, 0.94),
    );
    draw_rectangle_lines(17.0, panel_top + 1.0, 286.0, panel_height - 2.0, 1.0, GOLD);
    draw_centered_fit(
        GAME_OVER_TEXT,
        panel_top + 10.0,
        2.0,
        1.35,
        HUD_WHITE,
        Some(OUTLINE),
    );
    draw_centered_fit(
        &format!("{HUD_SCORE} {}", format_score(score)),
        panel_top + 39.0,
        1.4,
        0.8,
        HUD_WHITE,
        None,
    );
    draw_centered_fit(
        &format!("{HUD_HIGH_SCORE} {}", format_score(high_score)),
        panel_top + 54.0,
        1.0,
        0.7,
        Color::new(0.70, 0.86, 1.0, 1.0),
        None,
    );
    let attempts_label = if attempts_remaining == 0 {
        ALL_ATTEMPTS_USED_TEXT.to_owned()
    } else {
        format!("{ATTEMPTS_LEFT_TEXT}: {attempts_remaining}")
    };
    draw_centered_fit(&attempts_label, panel_top + 70.0, 1.0, 0.65, GOLD, None);

    if attempts_remaining > 0 {
        draw_arcade_button(NEXT_BUTTON, NEXT_ATTEMPT_TEXT, GOLD);
        draw_arcade_button(
            RETURN_BUTTON,
            RETURN_TO_RADIO_TEXT,
            Color::new(0.70, 0.86, 1.0, 1.0),
        );
    } else {
        draw_arcade_button(RETURN_BUTTON, RETURN_TO_RADIO_TEXT, GOLD);
    }
}

fn draw_arcade_button(rect: (f32, f32, f32, f32), label: &str, color: Color) {
    let (x, y, width, height) = rect;
    draw_rectangle(x, y, width, height, Color::new(0.16, 0.08, 0.30, 0.96));
    draw_rectangle_lines(x, y, width, height, 1.0, color);
    let scale = text::fit_scale(label, width - 12.0, 1.0, 0.65);
    let label_width = text::width(label, scale);
    let label_x = x + (width - label_width) * 0.5;
    text::draw(label, label_x + 1.0, y + 11.0, scale, OUTLINE);
    text::draw(label, label_x, y + 10.0, scale, color);
}

fn draw_centered(value: &str, y: f32, scale: f32, color: Color, shadow: Option<Color>) {
    let measured = text::bounds(value, 0.0, y, scale);
    let x = text::clamp_x(
        text::centered_x(value, 320.0, scale),
        measured.right - measured.left,
        320.0,
        0.0,
    );
    if let Some(shadow) = shadow {
        text::draw(value, x + 2.0, y + 2.0, scale, shadow);
    }
    text::draw(value, x, y, scale, color);
}

fn draw_centered_fit(
    value: &str,
    y: f32,
    preferred_scale: f32,
    minimum_scale: f32,
    color: Color,
    shadow: Option<Color>,
) {
    let scale = text::fit_scale(value, 300.0, preferred_scale, minimum_scale);
    draw_centered(value, y, scale, color, shadow);
}

#[cfg(test)]
fn centered_text_bounds(value: &str, y: f32, scale: f32) -> LayoutRect {
    let bounds = text::bounds(value, text::centered_x(value, 320.0, scale), y, scale);
    LayoutRect {
        left: bounds.left,
        top: bounds.top,
        right: bounds.right,
        bottom: bounds.bottom,
    }
}

#[cfg(test)]
fn title_layout_bounds() -> [LayoutRect; 4] {
    [
        centered_text_bounds(TITLE_TEXT, TITLE_Y, TITLE_SCALE),
        centered_text_bounds(TITLE_SUBTITLE, SUBTITLE_Y, 1.0),
        LayoutRect {
            left: 79.0,
            top: 138.0,
            right: 221.0,
            bottom: 197.0,
        },
        centered_text_bounds(TITLE_START, START_Y, START_SCALE),
    ]
}

#[cfg(test)]
fn rectangles_overlap(first: LayoutRect, second: LayoutRect) -> bool {
    first.left < second.right
        && first.right > second.left
        && first.top < second.bottom
        && first.bottom > second.top
}

#[cfg(test)]
fn text_bounds_overlap(first: text::Bounds, second: text::Bounds) -> bool {
    first.left < second.right
        && first.right > second.left
        && first.top < second.bottom
        && first.bottom > second.top
}

fn palette_colors(layer_index: usize) -> BlockColors {
    if layer_index <= 150 {
        interpolate_stops(&PRIMARY_STOPS, layer_index)
    } else {
        let cycle_layer = (layer_index - 150) % 600;
        interpolate_stops(&SECONDARY_STOPS, cycle_layer)
    }
}

fn interpolate_stops(stops: &[PaletteStop], layer: usize) -> BlockColors {
    let Some(last) = stops.last() else {
        unreachable!("palette must contain at least one stop");
    };
    if layer >= last.layer {
        return quantize_colors(last.colors);
    }

    let upper_index = stops
        .iter()
        .position(|stop| stop.layer >= layer)
        .unwrap_or(stops.len() - 1);
    if upper_index == 0 {
        return quantize_colors(stops[0].colors);
    }

    let lower = stops[upper_index - 1];
    let upper = stops[upper_index];
    let span = (upper.layer - lower.layer) as f32;
    let amount = (layer - lower.layer) as f32 / span;
    BlockColors {
        top: mix_color(lower.colors.top, upper.colors.top, amount),
        left: mix_color(lower.colors.left, upper.colors.left, amount),
        right: mix_color(lower.colors.right, upper.colors.right, amount),
    }
}

fn mix_color(lower: Color, upper: Color, amount: f32) -> Color {
    quantize_color(
        lower.r + (upper.r - lower.r) * amount,
        lower.g + (upper.g - lower.g) * amount,
        lower.b + (upper.b - lower.b) * amount,
    )
}

fn quantize_colors(colors: BlockColors) -> BlockColors {
    BlockColors {
        top: quantize_color(colors.top.r, colors.top.g, colors.top.b),
        left: quantize_color(colors.left.r, colors.left.g, colors.left.b),
        right: quantize_color(colors.right.r, colors.right.g, colors.right.b),
    }
}

fn quantize_color(red: f32, green: f32, blue: f32) -> Color {
    const LEVELS: f32 = 31.0;
    let quantize = |value: f32| (value.clamp(0.0, 1.0) * LEVELS).round() / LEVELS;
    Color::new(quantize(red), quantize(green), quantize(blue), 1.0)
}

fn format_score(score: u32) -> String {
    format!("{score:06}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn left_cutoff_fragment_preserves_removed_width_and_falls() {
        let mut fragment =
            FallingFragment::from_cutoff(Block::new(40.0, 100.0), Block::new(60.0, 100.0), 3)
                .expect("left overhang creates a fragment");
        let initial_left = fragment.block.left;
        let initial_width = fragment.block.width;
        let initial_y = fragment.world_y;

        assert_eq!(fragment.block, Block::new(40.0, 20.0));
        assert!(fragment.update(0.25));
        assert_eq!(fragment.block.left, initial_left);
        assert_eq!(fragment.block.width, initial_width);
        assert!(fragment.world_y > initial_y);
        assert!(fragment.velocity_y > INITIAL_FRAGMENT_VELOCITY_Y);
    }

    #[test]
    fn cutoff_fragments_fall_only_on_world_y_at_all_frame_rates() {
        for fps in [30.0, 60.0, 120.0] {
            let mut fragment =
                FallingFragment::from_cutoff(Block::new(40.0, 100.0), Block::new(60.0, 100.0), 3)
                    .expect("left overhang creates a fragment");
            let initial_block = fragment.block;
            let initial_y = fragment.world_y;
            let mut previous_velocity = fragment.velocity_y;

            for _ in 0..fps as usize {
                assert!(fragment.update(1.0 / fps));
                assert_eq!(fragment.block.left, initial_block.left);
                assert_eq!(fragment.block.width, initial_block.width);
                assert!(fragment.velocity_y >= previous_velocity);
                previous_velocity = fragment.velocity_y;
            }

            assert!(fragment.world_y > initial_y);
        }
    }

    #[test]
    fn exact_alignment_does_not_create_a_fragment() {
        assert!(
            FallingFragment::from_cutoff(Block::new(60.0, 100.0), Block::new(60.0, 100.0), 1,)
                .is_none()
        );
    }

    #[test]
    fn palette_interpolates_smoothly_across_the_primary_150_layer_ramp() {
        let mut changed_pairs = 0;
        for layer in 0..150 {
            let lower = palette_colors(layer).top;
            let upper = palette_colors(layer + 1).top;
            let largest_channel_delta = [
                (upper.r - lower.r).abs(),
                (upper.g - lower.g).abs(),
                (upper.b - lower.b).abs(),
            ]
            .into_iter()
            .fold(0.0, f32::max);

            assert!(
                largest_channel_delta <= 2.0 / 31.0,
                "abrupt palette jump between {layer} and {}",
                layer + 1
            );
            changed_pairs += (lower != upper) as usize;
        }
        // Quantization intentionally allows a few adjacent layers to share a
        // color value while the underlying ramp remains continuous.
        assert!(changed_pairs > 50);
    }

    #[test]
    fn palette_is_dark_low_and_progressively_brighter_through_layer_150() {
        let checkpoints = [0, 15, 30, 45, 60, 75, 90, 105, 120, 135, 150];
        let luminances: Vec<f32> = checkpoints
            .into_iter()
            .map(|layer| perceived_luminance(palette_colors(layer).top))
            .collect();

        assert!(luminances[0] < 0.2);
        for pair in luminances.windows(2) {
            assert!(pair[1] >= pair[0], "brightness regressed: {pair:?}");
        }
        assert!(luminances[10] > luminances[0] * 4.0);
    }

    #[test]
    fn palette_does_not_reach_orange_gold_immediately() {
        let layer_0 = palette_colors(0).top;
        let layer_50 = palette_colors(50).top;
        let layer_100 = palette_colors(100).top;
        let layer_150 = palette_colors(150).top;

        assert!(layer_0.b > layer_0.g);
        assert!(layer_50.b > layer_50.g);
        assert!(layer_100.g < 0.5);
        assert!(layer_150.g > 0.7);
    }

    #[test]
    fn palette_is_quantized_and_has_coordinated_long_run_behavior() {
        for layer in [0, 50, 100, 150, 200, 500] {
            let colors = palette_colors(layer);
            for channel in [
                colors.top.r,
                colors.top.g,
                colors.top.b,
                colors.left.r,
                colors.left.g,
                colors.left.b,
                colors.right.r,
                colors.right.g,
                colors.right.b,
            ] {
                let quantized = channel * 31.0;
                assert!((quantized - quantized.round()).abs() < 0.0001);
            }
        }

        assert_ne!(palette_colors(150), palette_colors(200));
        assert_ne!(palette_colors(200), palette_colors(500));
        assert!(perceived_luminance(palette_colors(200).top) > 0.6);
        assert!(perceived_luminance(palette_colors(500).top) > 0.6);
    }

    #[test]
    fn placed_layer_colors_are_stable_and_fragments_inherit_exact_colors() {
        let layer_50_before = palette_colors(50);
        let layer_100_before = palette_colors(100);
        let layer_150_before = palette_colors(150);

        // Re-evaluating a layer after the tower grows must produce the same
        // color; draw_tower passes the permanent layer index, not tower height.
        assert_eq!(layer_50_before, palette_colors(50));
        assert_eq!(layer_100_before, palette_colors(100));
        assert_eq!(layer_150_before, palette_colors(150));

        let fragment =
            FallingFragment::from_cutoff(Block::new(40.0, 100.0), Block::new(60.0, 100.0), 150)
                .expect("cutoff should create a fragment");
        assert_eq!(fragment.colors, layer_150_before);
    }

    #[test]
    fn title_layout_fits_the_viewport_without_overlapping_regions() {
        let [title, subtitle, tower, start] = title_layout_bounds();
        for region in [title, subtitle, tower, start] {
            assert!(region.left >= 0.0 && region.right <= 320.0);
            assert!(region.top >= 0.0 && region.bottom <= 224.0);
        }
        assert!(!rectangles_overlap(title, subtitle));
        assert!(!rectangles_overlap(subtitle, tower));
        assert!(!rectangles_overlap(tower, start));
        assert!(!rectangles_overlap(title, tower));
        assert!(!rectangles_overlap(subtitle, start));
    }

    #[test]
    fn death_replay_text_phases_have_separate_safe_regions() {
        let hud = LayoutRect {
            left: 5.0,
            top: 8.0,
            right: 315.0,
            bottom: 35.0,
        };
        let miss = centered_text_bounds(MISS_TEXT, 82.0, 2.0);
        let replay = centered_text_bounds(REPLAY_TEXT, 39.0, 1.0);
        let slow_motion = centered_text_bounds("ZEITLUPE", 50.0, 0.9);
        let progress = LayoutRect {
            left: 111.0,
            top: 60.0,
            right: 209.0,
            bottom: 65.0,
        };

        for label in [miss, replay, slow_motion] {
            assert!(label.left >= 4.0 && label.right <= 316.0);
            assert!(label.top >= 35.0 && label.bottom <= 224.0);
        }
        assert!(!rectangles_overlap(miss, hud));
        assert!(!rectangles_overlap(replay, slow_motion));
        assert!(!rectangles_overlap(slow_motion, progress));
    }

    #[test]
    fn hud_and_build_identifier_fit_their_screen_regions() {
        let build_scale = text::fit_scale(BUILD_IDENTIFIER, 58.0, 0.7, 0.5);
        let build = text::bounds(
            BUILD_IDENTIFIER,
            text::right_aligned_x(BUILD_IDENTIFIER, 318.0, build_scale),
            0.0,
            build_scale,
        );
        let high_label = text::bounds(
            HUD_HIGH_SCORE,
            text::right_aligned_x(HUD_HIGH_SCORE, 310.0, 1.0),
            11.0,
            1.0,
        );
        let high_value = format_score(123_456);
        let high_score = text::bounds(
            &high_value,
            text::right_aligned_x(&high_value, 310.0, 2.0),
            20.0,
            2.0,
        );

        assert!(build.left >= 0.0 && build.right <= 318.0 && build.bottom <= 8.0);
        assert!(high_label.left >= 204.0 && high_label.right <= 315.0);
        assert!(high_score.left >= 204.0 && high_score.right <= 315.0);
        assert!(!text_bounds_overlap(build, high_label));
    }

    #[test]
    fn player_facing_strings_are_german_and_fit_the_logical_viewport() {
        let strings = [
            TITLE_TEXT,
            TITLE_SUBTITLE,
            TITLE_START,
            HUD_SCORE,
            HUD_HIGH_SCORE,
            PERFECT_TEXT,
            PAUSE_TEXT,
            PAUSE_HINT,
            GAME_OVER_TEXT,
            RESTART_TEXT,
            REPLAY_TEXT,
            MISS_TEXT,
            BUILD_IDENTIFIER,
            "ZEITLUPE",
            NEXT_ATTEMPT_TEXT,
            RETURN_TO_RADIO_TEXT,
            ATTEMPTS_LEFT_TEXT,
            ALL_ATTEMPTS_USED_TEXT,
        ];
        for value in strings {
            assert!(
                text::width(value, 2.0) <= 320.0,
                "text is too wide: {value}"
            );
            assert!(!value.contains("SCORE"));
            assert!(!value.contains("GAME OVER"));
            assert!(!value.contains("REPLAY"));
            assert!(!value.contains("MISS"));
            assert!(!value.contains("PRESS"));
        }
    }

    fn perceived_luminance(color: Color) -> f32 {
        color.r * 0.21 + color.g * 0.72 + color.b * 0.07
    }
}
