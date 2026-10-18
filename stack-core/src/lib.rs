//! Deterministic gameplay rules for Stack-it-16.
//!
//! This crate deliberately contains no windowing, rendering, audio, or input
//! dependencies. A front end advances the simulation with [`Game::update`]
//! and sends actions through [`Game::handle_action`].

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Block {
    pub left: f32,
    pub width: f32,
}

impl Block {
    pub const fn new(left: f32, width: f32) -> Self {
        Self { left, width }
    }

    pub fn right(self) -> f32 {
        self.left + self.width
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GameConfig {
    pub playfield_width: f32,
    pub base_left: f32,
    pub base_width: f32,
    pub initial_speed: f32,
    pub speed_step: f32,
    pub max_speed: f32,
    pub perfect_tolerance: f32,
    pub minimum_width: f32,
    pub points_per_layer: u32,
    pub perfect_bonus: u32,
}

impl Default for GameConfig {
    fn default() -> Self {
        Self {
            playfield_width: 320.0,
            base_left: 60.0,
            base_width: 200.0,
            initial_speed: 78.0,
            speed_step: 5.0,
            max_speed: 168.0,
            perfect_tolerance: 3.0,
            // A two-pixel survivor is still visible at the 320x224 logical
            // resolution. Larger overlaps should not be rejected merely
            // because the tower has become narrow.
            minimum_width: 2.0,
            points_per_layer: 1,
            perfect_bonus: 1,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GamePhase {
    TitleScreen,
    Playing,
    Paused,
    GameOver,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MovingBlock {
    pub block: Block,
    pub velocity: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Placement {
    Success { block: Block, perfect: bool },
    Missed,
}

/// Resolve a drop against the previous layer in gameplay coordinates.
///
/// A near-enough drop is snapped to the previous layer. Otherwise the
/// intersection is retained and the overhang is discarded. Only a very small
/// surviving width is rejected, so a meaningful overlap is not an unfair miss.
pub fn resolve_placement(moving: Block, previous: Block, config: &GameConfig) -> Placement {
    let offset = (moving.left - previous.left).abs();
    if offset <= config.perfect_tolerance {
        return Placement::Success {
            block: previous,
            perfect: true,
        };
    }

    let Some(overlap) = overlap(moving, previous) else {
        return Placement::Missed;
    };
    if overlap.width < config.minimum_width {
        Placement::Missed
    } else {
        Placement::Success {
            block: overlap,
            perfect: false,
        }
    }
}

/// Return the positive horizontal intersection used by both placement rules
/// and diagnostics. Touching edges have no material overlap.
pub fn overlap(moving: Block, previous: Block) -> Option<Block> {
    let overlap_left = moving.left.max(previous.left);
    let overlap_right = moving.right().min(previous.right());
    if overlap_right > overlap_left {
        Some(Block::new(overlap_left, overlap_right - overlap_left))
    } else {
        None
    }
}

/// Return the new high score and whether it changed.
pub fn update_high_score(current: u32, candidate: u32) -> (u32, bool) {
    if candidate > current {
        (candidate, true)
    } else {
        (current, false)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Action {
    Start,
    Place,
    PauseToggle,
    Restart,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum GameEvent {
    Ignored,
    Started,
    Paused,
    Resumed,
    Restarted,
    Placed { perfect: bool, score: u32 },
    GameOver { score: u32 },
}

pub struct Game {
    config: GameConfig,
    phase: GamePhase,
    layers: Vec<Block>,
    active: Option<MovingBlock>,
    score: u32,
    placements: u32,
}

impl Game {
    pub fn new(config: GameConfig) -> Self {
        Self {
            config,
            phase: GamePhase::TitleScreen,
            layers: Vec::new(),
            active: None,
            score: 0,
            placements: 0,
        }
    }

    pub fn config(&self) -> GameConfig {
        self.config
    }

    pub fn phase(&self) -> GamePhase {
        self.phase
    }

    pub fn layers(&self) -> &[Block] {
        &self.layers
    }

    pub fn active_block(&self) -> Option<MovingBlock> {
        self.active
    }

    pub fn score(&self) -> u32 {
        self.score
    }

    pub fn placements(&self) -> u32 {
        self.placements
    }

    pub fn start(&mut self) -> GameEvent {
        self.reset_run();
        self.phase = GamePhase::Playing;
        GameEvent::Started
    }

    pub fn restart(&mut self) -> GameEvent {
        self.reset_run();
        self.phase = GamePhase::Playing;
        GameEvent::Restarted
    }

    pub fn handle_action(&mut self, action: Action) -> GameEvent {
        match action {
            Action::Start if self.phase == GamePhase::TitleScreen => self.start(),
            Action::Place if self.phase == GamePhase::TitleScreen => self.start(),
            Action::PauseToggle => match self.phase {
                GamePhase::Playing => {
                    self.phase = GamePhase::Paused;
                    GameEvent::Paused
                }
                GamePhase::Paused => {
                    self.phase = GamePhase::Playing;
                    GameEvent::Resumed
                }
                _ => GameEvent::Ignored,
            },
            Action::Place if self.phase == GamePhase::Playing => self.place_active(),
            Action::Restart if self.phase == GamePhase::GameOver => self.restart(),
            _ => GameEvent::Ignored,
        }
    }

    pub fn update(&mut self, elapsed_seconds: f32) {
        if self.phase != GamePhase::Playing {
            return;
        }

        let Some(mut active) = self.active else {
            return;
        };
        let dt = elapsed_seconds.max(0.0);
        active.block.left += active.velocity * dt;

        if active.block.left <= 0.0 {
            active.block.left = 0.0;
            active.velocity = active.velocity.abs();
        } else if active.block.right() >= self.config.playfield_width {
            active.block.left = (self.config.playfield_width - active.block.width).max(0.0);
            active.velocity = -active.velocity.abs();
        }
        self.active = Some(active);
    }

    fn reset_run(&mut self) {
        self.layers.clear();
        self.layers
            .push(Block::new(self.config.base_left, self.config.base_width));
        self.score = 0;
        self.placements = 0;
        self.active = Some(self.spawn_block());
    }

    fn spawn_block(&self) -> MovingBlock {
        let width = self
            .layers
            .last()
            .map_or(self.config.base_width, |b| b.width);
        let width = width.min(self.config.playfield_width);
        let moving_left_to_right = self.placements.is_multiple_of(2);
        let left = if moving_left_to_right {
            0.0
        } else {
            (self.config.playfield_width - width).max(0.0)
        };
        let speed = (self.config.initial_speed + self.placements as f32 * self.config.speed_step)
            .min(self.config.max_speed);
        let velocity = if moving_left_to_right { speed } else { -speed };
        MovingBlock {
            block: Block::new(left, width),
            velocity,
        }
    }

    fn place_active(&mut self) -> GameEvent {
        let Some(active) = self.active else {
            return GameEvent::Ignored;
        };
        let Some(previous) = self.layers.last().copied() else {
            return GameEvent::Ignored;
        };

        match resolve_placement(active.block, previous, &self.config) {
            Placement::Missed => {
                self.active = None;
                self.phase = GamePhase::GameOver;
                GameEvent::GameOver { score: self.score }
            }
            Placement::Success { block, perfect } => {
                self.layers.push(block);
                self.placements += 1;
                self.score += self.config.points_per_layer;
                if perfect {
                    self.score += self.config.perfect_bonus;
                }
                self.active = Some(self.spawn_block());
                GameEvent::Placed {
                    perfect,
                    score: self.score,
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> GameConfig {
        GameConfig {
            perfect_tolerance: 3.0,
            minimum_width: 2.0,
            ..GameConfig::default()
        }
    }

    #[test]
    fn exact_alignment_is_perfect_and_keeps_full_width() {
        let previous = Block::new(80.0, 100.0);
        let moving = Block::new(80.0, 100.0);

        assert_eq!(
            resolve_placement(moving, previous, &config()),
            Placement::Success {
                block: previous,
                perfect: true
            }
        );
    }

    #[test]
    fn tolerance_boundary_is_perfect() {
        let cfg = config();
        let previous = Block::new(80.0, 100.0);
        let moving = Block::new(83.0, 100.0);

        assert!(matches!(
            resolve_placement(moving, previous, &cfg),
            Placement::Success { perfect: true, .. }
        ));
    }

    #[test]
    fn placement_just_outside_tolerance_is_trimmed() {
        let cfg = config();
        let previous = Block::new(80.0, 100.0);
        let moving = Block::new(84.0, 100.0);

        assert_eq!(
            resolve_placement(moving, previous, &cfg),
            Placement::Success {
                block: Block::new(84.0, 96.0),
                perfect: false
            }
        );
    }

    #[test]
    fn no_overlap_is_a_miss() {
        let previous = Block::new(80.0, 100.0);
        let moving = Block::new(190.0, 100.0);

        assert_eq!(
            resolve_placement(moving, previous, &config()),
            Placement::Missed
        );
    }

    #[test]
    fn overlap_geometry_matches_the_placement_intersection() {
        let moving = Block::new(120.0, 100.0);
        let previous = Block::new(80.0, 100.0);

        assert_eq!(overlap(moving, previous), Some(Block::new(120.0, 60.0)));
        assert_eq!(overlap(Block::new(180.0, 20.0), previous), None);
    }

    #[test]
    fn overlap_at_minimum_width_is_allowed() {
        let cfg = config();
        let previous = Block::new(80.0, 100.0);
        let moving = Block::new(178.0, 100.0);

        assert_eq!(
            resolve_placement(moving, previous, &cfg),
            Placement::Success {
                block: Block::new(178.0, 2.0),
                perfect: false
            }
        );
    }

    #[test]
    fn overlap_below_minimum_width_is_a_miss() {
        let cfg = config();
        let previous = Block::new(80.0, 100.0);
        let moving = Block::new(179.0, 100.0);

        assert_eq!(resolve_placement(moving, previous, &cfg), Placement::Missed);
    }

    #[test]
    fn narrow_overlaps_follow_the_two_pixel_survivor_rule() {
        let cfg = config();
        let previous = Block::new(80.0, 100.0);

        for (width, expected) in [
            (0.0, Placement::Missed),
            (1.0, Placement::Missed),
            (
                2.0,
                Placement::Success {
                    block: Block::new(178.0, 2.0),
                    perfect: false,
                },
            ),
            (
                5.0,
                Placement::Success {
                    block: Block::new(175.0, 5.0),
                    perfect: false,
                },
            ),
            (
                10.0,
                Placement::Success {
                    block: Block::new(170.0, 10.0),
                    perfect: false,
                },
            ),
        ] {
            let moving_left = previous.right() - width;
            let moving = Block::new(moving_left, 100.0);
            assert_eq!(resolve_placement(moving, previous, &cfg), expected);
        }
    }

    #[test]
    fn level_72_overlap_is_accepted_and_not_enlarged() {
        let cfg = config();
        let moving = Block::new(142.485, 16.512);
        let previous = Block::new(135.398, 16.512);

        let expected = overlap(moving, previous).expect("level 72 has positive overlap");
        assert!((expected.width - 9.425).abs() < 0.001);
        assert_eq!(
            resolve_placement(moving, previous, &cfg),
            Placement::Success {
                block: expected,
                perfect: false,
            }
        );
    }

    #[test]
    fn game_scores_normal_and_perfect_layers() {
        let mut game = Game::new(config());
        assert_eq!(game.handle_action(Action::Start), GameEvent::Started);

        let first = game.active_block().expect("active block after start");
        let previous = game.layers().last().copied().expect("base layer");
        let mut aligned = first;
        aligned.block.left = previous.left;
        game.active = Some(aligned);
        assert_eq!(
            game.handle_action(Action::Place),
            GameEvent::Placed {
                perfect: true,
                score: 2
            }
        );

        let second = game.active_block().expect("second active block");
        let previous = game.layers().last().copied().expect("first layer");
        let mut partial = second;
        partial.block.left = previous.left + 4.0;
        game.active = Some(partial);
        assert_eq!(
            game.handle_action(Action::Place),
            GameEvent::Placed {
                perfect: false,
                score: 3
            }
        );
    }

    #[test]
    fn high_score_only_changes_for_a_new_record() {
        assert_eq!(update_high_score(12, 12), (12, false));
        assert_eq!(update_high_score(12, 9), (12, false));
        assert_eq!(update_high_score(12, 13), (13, true));
    }

    #[test]
    fn ideal_play_can_continue_beyond_two_hundred_placements() {
        let mut game = Game::new(config());
        assert_eq!(game.handle_action(Action::Start), GameEvent::Started);

        for placement in 0..201 {
            let previous = game.layers.last().copied().expect("previous layer");
            let mut active = game.active.expect("active layer");
            active.block.left = previous.left;
            game.active = Some(active);

            assert_eq!(
                game.handle_action(Action::Place),
                GameEvent::Placed {
                    perfect: true,
                    score: (placement + 1) * 2,
                }
            );
            assert_eq!(game.phase, GamePhase::Playing);
        }

        assert_eq!(game.placements(), 201);
        assert_eq!(game.layers().len(), 202);
        assert!(game.active_block().is_some());
    }

    fn assert_valid_state(game: &Game) {
        let cfg = game.config();
        for layer in game.layers() {
            assert!(layer.left.is_finite());
            assert!(layer.width.is_finite());
            assert!(layer.left >= 0.0);
            assert!(layer.width >= cfg.minimum_width);
            assert!(layer.right() <= cfg.playfield_width);
        }

        if let Some(active) = game.active_block() {
            assert!(active.block.left.is_finite());
            assert!(active.block.width.is_finite());
            assert!(active.velocity.is_finite());
            assert!(active.block.left >= 0.0);
            assert!(active.block.width >= cfg.minimum_width);
            assert!(active.block.right() <= cfg.playfield_width);
            assert!(active.velocity.abs() <= cfg.max_speed);
        }
    }

    fn run_deterministic_ideal_play(placements: usize, frames_per_placement: usize, fps: f32) {
        let mut game = Game::new(config());
        assert_eq!(game.handle_action(Action::Start), GameEvent::Started);
        let dt = 1.0 / fps;

        for placement in 0..placements {
            for _ in 0..frames_per_placement + placement % 3 {
                game.update(dt);
                assert_valid_state(&game);
            }

            // This is the deterministic equivalent of an ideal player timing
            // the input at the previous layer's horizontal position. Movement
            // has still been advanced through the real update path above.
            let previous = game.layers().last().copied().expect("previous layer");
            let mut active = game.active_block().expect("active block");
            active.block.left = previous.left;
            game.active = Some(active);

            assert_eq!(
                game.handle_action(Action::Place),
                GameEvent::Placed {
                    perfect: true,
                    score: ((placement as u32) + 1) * 2,
                }
            );
            assert_eq!(game.phase(), GamePhase::Playing);
            assert_valid_state(&game);
        }

        assert_eq!(game.placements(), placements as u32);
        assert_eq!(game.layers().len(), placements + 1);
    }

    #[test]
    fn deterministic_runs_reach_500_1000_and_5000_placements_at_multiple_fps() {
        for fps in [30.0, 60.0, 120.0] {
            run_deterministic_ideal_play(500, 1, fps);
            run_deterministic_ideal_play(1_000, 2, fps);
            run_deterministic_ideal_play(5_000, 3, fps);
        }
    }

    #[test]
    fn imperfect_placements_shrink_until_the_minimum_width_then_legitimately_miss() {
        let cfg = config();
        let mut game = Game::new(cfg);
        assert_eq!(game.handle_action(Action::Start), GameEvent::Started);
        let mut successful_imperfect = 0;

        let game_over = loop {
            let previous = game.layers().last().copied().expect("previous layer");
            let mut active = game.active_block().expect("active block");
            active.block.left = previous.left + cfg.perfect_tolerance + 1.0;
            game.active = Some(active);

            match game.handle_action(Action::Place) {
                GameEvent::Placed { perfect: false, .. } => {
                    successful_imperfect += 1;
                    assert_valid_state(&game);
                }
                event @ GameEvent::GameOver { .. } => break event,
                event => panic!("unexpected event during imperfect run: {event:?}"),
            }
        };

        assert!(successful_imperfect > 40);
        assert_eq!(
            game_over,
            GameEvent::GameOver {
                score: successful_imperfect
            }
        );
        assert_eq!(game.phase(), GamePhase::GameOver);

        let previous = game.layers().last().copied().expect("last valid layer");
        let failed_active = game.active_block();
        assert!(
            failed_active.is_none(),
            "failed placement must clear active block"
        );
        assert!(previous.width >= cfg.minimum_width);
    }

    #[test]
    fn rapid_repeated_place_inputs_never_duplicate_a_placement_or_change_game_over_rules() {
        let mut game = Game::new(config());
        assert_eq!(game.handle_action(Action::Start), GameEvent::Started);
        let mut saw_game_over = false;

        for _ in 0..1_000 {
            let before_placements = game.placements();
            let before_score = game.score();
            let event = game.handle_action(Action::Place);

            assert!(game.placements() <= before_placements + 1);
            if saw_game_over {
                assert_eq!(event, GameEvent::Ignored);
                assert_eq!(game.placements(), before_placements);
                assert_eq!(game.score(), before_score);
            }
            if matches!(event, GameEvent::GameOver { .. }) {
                saw_game_over = true;
                assert!(game.active_block().is_none());
            }
            assert_valid_state(&game);
        }

        assert!(saw_game_over);
    }

    #[test]
    fn every_game_over_from_sampled_positions_has_an_invalid_overlap() {
        let cfg = config();
        let mut game_over_count = 0;

        for moving_left in (-120..641).map(|left| left as f32 * 0.5) {
            let mut game = Game::new(cfg);
            assert_eq!(game.handle_action(Action::Start), GameEvent::Started);
            let previous = game.layers().last().copied().expect("base layer");
            let mut active = game.active_block().expect("active block");
            active.block.left = moving_left;
            game.active = Some(active);

            let event = game.handle_action(Action::Place);
            if matches!(event, GameEvent::GameOver { .. }) {
                game_over_count += 1;
                let intersection = overlap(active.block, previous);
                assert!(
                    (active.block.left - previous.left).abs() > cfg.perfect_tolerance,
                    "a perfect-tolerance position must not game over"
                );
                assert!(
                    intersection.is_none_or(|value| value.width < cfg.minimum_width),
                    "Game Over had a playable overlap at left={moving_left}: {intersection:?}"
                );
            }
        }

        assert!(game_over_count > 0);
    }

    #[test]
    fn diagnostic_overlap_always_matches_non_perfect_placement_geometry() {
        let cfg = config();
        let previous = Block::new(80.0, 100.0);

        for moving_left in (-80..401).map(|left| left as f32 * 0.5) {
            let moving = Block::new(moving_left, 100.0);
            let diagnostic = overlap(moving, previous);
            match resolve_placement(moving, previous, &cfg) {
                Placement::Success {
                    block,
                    perfect: false,
                } => {
                    assert_eq!(diagnostic, Some(block));
                }
                Placement::Success { perfect: true, .. } => {
                    assert!((moving.left - previous.left).abs() <= cfg.perfect_tolerance);
                }
                Placement::Missed => {
                    assert!(diagnostic.is_none_or(|value| value.width < cfg.minimum_width));
                }
            }
        }
    }

    #[test]
    fn high_level_updates_keep_speed_and_coordinates_bounded() {
        let mut game = Game::new(config());
        assert_eq!(game.handle_action(Action::Start), GameEvent::Started);

        for placement in 0..5_000 {
            for fps in [30.0, 60.0, 120.0] {
                game.update(1.0 / fps);
                assert_valid_state(&game);
            }

            let previous = game.layers().last().copied().expect("previous layer");
            let mut active = game.active_block().expect("active block");
            active.block.left = previous.left;
            game.active = Some(active);
            assert!(matches!(
                game.handle_action(Action::Place),
                GameEvent::Placed { perfect: true, .. }
            ));

            if placement % 257 == 0 {
                assert_valid_state(&game);
            }
        }
    }

    #[test]
    fn game_transitions_pause_and_game_over() {
        let mut game = Game::new(config());
        game.start();
        assert_eq!(game.handle_action(Action::PauseToggle), GameEvent::Paused);
        assert_eq!(game.phase(), GamePhase::Paused);
        assert_eq!(game.handle_action(Action::PauseToggle), GameEvent::Resumed);

        game.active = Some(MovingBlock {
            block: Block::new(0.0, 10.0),
            velocity: 0.0,
        });
        assert_eq!(
            game.handle_action(Action::Place),
            GameEvent::GameOver { score: 0 }
        );
        assert_eq!(game.phase(), GamePhase::GameOver);
        assert_eq!(game.handle_action(Action::Restart), GameEvent::Restarted);
        assert_eq!(game.phase(), GamePhase::Playing);
    }

    #[test]
    fn movement_bounces_at_playfield_edges() {
        let mut game = Game::new(config());
        game.start();
        game.update(10.0);
        let active = game.active_block().expect("active block");
        assert_eq!(active.block.left + active.block.width, 320.0);
        assert!(active.velocity < 0.0);
    }
}
