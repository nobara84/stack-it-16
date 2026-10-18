use std::collections::VecDeque;

use stack_core::{Block, Game};

pub const HISTORY_CAPACITY: usize = 128;
pub const HISTORY_WINDOW_SECONDS: f32 = 0.75;
pub const REPLAY_DURATION_SECONDS: f32 = 2.75;
pub const FAILURE_FREEZE_DURATION: f32 = 0.5;
pub const REWIND_DURATION: f32 = 0.3;
pub const FINAL_IMPACT_DURATION: f32 = 0.18;
pub const FALL_DURATION: f32 = 1.0;
const REPLAY_INITIAL_FALL_SPEED_Y: f32 = 18.0;
const REPLAY_GRAVITY_Y: f32 = 220.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ReplaySample {
    pub block: Block,
    pub velocity: f32,
    pub timestamp: f32,
}

#[derive(Debug)]
pub struct MovementHistory {
    samples: VecDeque<ReplaySample>,
}

impl Default for MovementHistory {
    fn default() -> Self {
        Self::new()
    }
}

impl MovementHistory {
    pub fn new() -> Self {
        Self {
            samples: VecDeque::with_capacity(HISTORY_CAPACITY),
        }
    }

    pub fn clear(&mut self) {
        self.samples.clear();
    }

    pub fn record(&mut self, sample: ReplaySample) {
        if self.samples.len() == HISTORY_CAPACITY {
            self.samples.pop_front();
        }
        self.samples.push_back(sample);
        while self.samples.len() > 1
            && sample.timestamp - self.samples.front().unwrap().timestamp > HISTORY_WINDOW_SECONDS
        {
            self.samples.pop_front();
        }
    }

    pub fn record_active(&mut self, game: &Game, timestamp: f32) {
        if let Some(active) = game.active_block() {
            self.record(ReplaySample {
                block: active.block,
                velocity: active.velocity,
                timestamp,
            });
        }
    }

    pub fn snapshot(&self) -> Vec<ReplaySample> {
        self.samples.iter().copied().collect()
    }

    #[cfg(test)]
    fn samples(&self) -> impl Iterator<Item = &ReplaySample> {
        self.samples.iter()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FailureReason {
    ZeroOverlap,
    BelowMinimumWidth,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FailedPlacement {
    pub active: Block,
    pub previous: Block,
    pub overlap: Option<Block>,
    pub minimum_width: f32,
    pub reason: FailureReason,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ReplayStage {
    FailureFreeze,
    Rewind,
    SlowMotion,
    FinalImpact,
    Falling,
    Complete,
}

#[derive(Debug)]
pub struct DeathReplay {
    samples: Vec<ReplaySample>,
    failure: FailedPlacement,
    stage: ReplayStage,
    failure_freeze_seconds: f32,
    rewind_seconds: f32,
    playback_seconds: f32,
    impact_seconds: f32,
    fall_seconds: f32,
    fall_offset_y: f32,
    fall_velocity_y: f32,
    source_duration: f32,
    current_block: Block,
    camera_target_y: f32,
}

impl DeathReplay {
    pub fn new(mut samples: Vec<ReplaySample>, failure: FailedPlacement) -> Self {
        if samples.is_empty() {
            samples.push(ReplaySample {
                block: failure.active,
                velocity: 0.0,
                timestamp: 0.0,
            });
        }

        // The placement action is authoritative. Always make its exact
        // collision position the final replay sample, even if the caller's
        // frame recorder was one sample behind.
        let last_timestamp = samples.last().map_or(0.0, |sample| sample.timestamp);
        if samples
            .last()
            .is_none_or(|sample| sample.block != failure.active)
        {
            samples.push(ReplaySample {
                block: failure.active,
                velocity: samples.last().map_or(0.0, |sample| sample.velocity),
                timestamp: last_timestamp,
            });
        } else if let Some(last) = samples.last_mut() {
            last.block = failure.active;
        }

        let first_timestamp = samples[0].timestamp;
        let final_timestamp = samples
            .last()
            .map_or(first_timestamp, |sample| sample.timestamp);
        let source_duration = (final_timestamp - first_timestamp).max(0.0);

        Self {
            current_block: failure.active,
            samples,
            failure,
            stage: ReplayStage::FailureFreeze,
            failure_freeze_seconds: 0.0,
            rewind_seconds: 0.0,
            playback_seconds: 0.0,
            impact_seconds: 0.0,
            fall_seconds: 0.0,
            fall_offset_y: 0.0,
            fall_velocity_y: REPLAY_INITIAL_FALL_SPEED_Y,
            source_duration,
            camera_target_y: crate::camera::INITIAL_TARGET_Y,
        }
    }

    pub fn set_camera_target_y(&mut self, camera_target_y: f32) {
        self.camera_target_y = camera_target_y;
    }

    pub fn update(&mut self, elapsed_seconds: f32) {
        if self.stage == ReplayStage::Complete {
            return;
        }

        let dt = elapsed_seconds.max(0.0);
        match self.stage {
            ReplayStage::FailureFreeze => {
                self.failure_freeze_seconds += dt;
                if self.failure_freeze_seconds >= FAILURE_FREEZE_DURATION {
                    self.stage = ReplayStage::Rewind;
                    self.rewind_seconds = 0.0;
                }
            }
            ReplayStage::Rewind => {
                self.rewind_seconds += dt;
                let amount = (self.rewind_seconds / REWIND_DURATION).clamp(0.0, 1.0);
                let first_timestamp = self.samples[0].timestamp;
                let last_timestamp = self
                    .samples
                    .last()
                    .map_or(first_timestamp, |sample| sample.timestamp);
                let timestamp = last_timestamp - (last_timestamp - first_timestamp) * amount;
                self.current_block = interpolate_samples(&self.samples, timestamp);
                if amount >= 1.0 {
                    self.stage = ReplayStage::SlowMotion;
                    self.playback_seconds = 0.0;
                    self.current_block = self.samples[0].block;
                }
            }
            ReplayStage::SlowMotion => {
                if self.source_duration <= f32::EPSILON {
                    self.finish_slow_motion();
                    return;
                }

                self.playback_seconds += dt * self.playback_rate();
                if self.playback_seconds >= self.source_duration {
                    self.finish_slow_motion();
                } else {
                    let target_timestamp =
                        self.samples[0].timestamp + self.playback_seconds / self.playback_rate();
                    self.current_block = interpolate_samples(&self.samples, target_timestamp);
                }
            }
            ReplayStage::FinalImpact => {
                self.impact_seconds += dt;
                if self.impact_seconds >= FINAL_IMPACT_DURATION {
                    self.stage = ReplayStage::Falling;
                    self.fall_seconds = 0.0;
                    self.fall_offset_y = 0.0;
                }
            }
            ReplayStage::Falling => {
                self.fall_seconds += dt;
                self.fall_offset_y += self.fall_velocity_y * dt;
                self.fall_velocity_y += REPLAY_GRAVITY_Y * dt;
                if self.fall_seconds >= FALL_DURATION {
                    self.stage = ReplayStage::Complete;
                }
            }
            ReplayStage::Complete => {}
        }
    }

    pub fn skip(&mut self) {
        self.stage = ReplayStage::Complete;
    }

    pub fn is_complete(&self) -> bool {
        self.stage == ReplayStage::Complete
    }

    pub fn current_block(&self) -> Block {
        self.current_block
    }

    pub fn camera_target_y(&self) -> f32 {
        self.camera_target_y
    }

    pub fn previous_block(&self) -> Block {
        self.failure.previous
    }

    pub fn overlap_block(&self) -> Option<Block> {
        self.failure.overlap
    }

    pub fn failure_reason(&self) -> FailureReason {
        self.failure.reason
    }

    pub fn show_miss_target(&self) -> bool {
        matches!(
            self.stage,
            ReplayStage::FailureFreeze | ReplayStage::FinalImpact | ReplayStage::Falling
        )
    }

    pub fn is_failure_freeze(&self) -> bool {
        self.stage == ReplayStage::FailureFreeze
    }

    pub fn is_rewinding(&self) -> bool {
        self.stage == ReplayStage::Rewind
    }

    pub fn is_slow_motion(&self) -> bool {
        self.stage == ReplayStage::SlowMotion
    }

    #[cfg(test)]
    pub fn is_final_impact(&self) -> bool {
        self.stage == ReplayStage::FinalImpact
    }

    #[cfg(test)]
    pub fn is_falling(&self) -> bool {
        self.stage == ReplayStage::Falling
    }

    pub fn fall_offset_y(&self) -> f32 {
        self.fall_offset_y
    }

    pub fn show_miss_feedback(&self) -> bool {
        matches!(self.stage, ReplayStage::FinalImpact | ReplayStage::Falling)
    }

    pub fn is_flashing(&self) -> bool {
        (self.stage == ReplayStage::FailureFreeze && self.failure_freeze_seconds < 0.20)
            || (self.stage == ReplayStage::FinalImpact && self.impact_seconds < 0.12)
    }

    pub fn playback_rate(&self) -> f32 {
        if self.source_duration <= f32::EPSILON {
            1.0
        } else {
            self.source_duration / REPLAY_DURATION_SECONDS
        }
    }

    pub fn progress(&self) -> f32 {
        match self.stage {
            ReplayStage::SlowMotion => {
                if self.source_duration <= f32::EPSILON {
                    1.0
                } else {
                    (self.playback_seconds / self.source_duration).clamp(0.0, 1.0)
                }
            }
            ReplayStage::FailureFreeze | ReplayStage::Rewind => 0.0,
            ReplayStage::FinalImpact | ReplayStage::Falling | ReplayStage::Complete => 1.0,
        }
    }

    fn finish_slow_motion(&mut self) {
        self.current_block = self.failure.active;
        self.stage = ReplayStage::FinalImpact;
        self.impact_seconds = 0.0;
    }
}

fn interpolate_samples(samples: &[ReplaySample], timestamp: f32) -> Block {
    if timestamp <= samples[0].timestamp {
        return samples[0].block;
    }

    for pair in samples.windows(2) {
        let [lower, upper] = pair else {
            unreachable!();
        };
        if timestamp <= upper.timestamp {
            let span = upper.timestamp - lower.timestamp;
            if span <= f32::EPSILON {
                return upper.block;
            }
            let amount = ((timestamp - lower.timestamp) / span).clamp(0.0, 1.0);
            return Block::new(
                lower.block.left + (upper.block.left - lower.block.left) * amount,
                lower.block.width + (upper.block.width - lower.block.width) * amount,
            );
        }
    }

    samples
        .last()
        .map_or(samples[0].block, |sample| sample.block)
}

#[cfg(test)]
mod tests {
    use super::*;
    use stack_core::Block;

    fn sample(left: f32, timestamp: f32) -> ReplaySample {
        ReplaySample {
            block: Block::new(left, 20.0),
            velocity: 80.0,
            timestamp,
        }
    }

    fn failure(reason: FailureReason) -> FailedPlacement {
        FailedPlacement {
            active: Block::new(141.0, 9.425),
            previous: Block::new(135.0, 16.512),
            overlap: Some(Block::new(141.0, 9.425)),
            minimum_width: 2.0,
            reason,
        }
    }

    #[test]
    fn history_is_bounded_and_keeps_chronological_order() {
        let mut history = MovementHistory::new();
        for index in 0..(HISTORY_CAPACITY + 5) {
            history.record(sample(index as f32, index as f32 * 0.005));
        }

        let timestamps: Vec<f32> = history.samples().map(|value| value.timestamp).collect();
        assert_eq!(timestamps.len(), HISTORY_CAPACITY);
        assert!((timestamps[0] - 0.025).abs() < 0.00001);
        assert!(
            (timestamps[HISTORY_CAPACITY - 1] - (HISTORY_CAPACITY + 4) as f32 * 0.005).abs()
                < 0.00001
        );
    }

    #[test]
    fn history_keeps_the_configured_window_at_low_frame_rates() {
        let mut history = MovementHistory::new();
        for index in 0..120 {
            history.record(sample(index as f32, index as f32 / 30.0));
        }

        let samples: Vec<ReplaySample> = history.samples().copied().collect();
        assert!((samples.last().unwrap().timestamp - samples[0].timestamp) <= 1.0);
        assert!(samples.len() <= 31);
    }

    #[test]
    fn replay_preserves_the_exact_failed_placement_frame() {
        let final_block = Block::new(142.485, 16.512);
        let previous_block = Block::new(135.0, 16.512);
        let mut replay = DeathReplay::new(
            vec![sample(100.0, 10.0), sample(140.0, 11.0)],
            FailedPlacement {
                active: final_block,
                previous: previous_block,
                ..failure(FailureReason::BelowMinimumWidth)
            },
        );

        assert!(replay.is_failure_freeze());
        assert_eq!(replay.current_block(), final_block);
        assert_eq!(replay.previous_block(), previous_block);
        assert!(replay.show_miss_target());

        replay.update(FAILURE_FREEZE_DURATION);
        assert!(replay.is_rewinding());
        replay.update(REWIND_DURATION);
        assert!(replay.is_slow_motion());
        replay.update(REPLAY_DURATION_SECONDS);
        assert!(replay.is_final_impact());
        assert_eq!(replay.current_block(), final_block);
        assert!(replay.show_miss_target());
    }

    #[test]
    fn replay_preserves_the_camera_captured_at_failure() {
        let captured_target_y = -214.5;
        let mut replay = DeathReplay::new(
            vec![sample(100.0, 10.0), sample(140.0, 11.0)],
            failure(FailureReason::ZeroOverlap),
        );
        replay.set_camera_target_y(captured_target_y);

        for delta in [
            FAILURE_FREEZE_DURATION,
            REWIND_DURATION,
            REPLAY_DURATION_SECONDS,
            FINAL_IMPACT_DURATION,
            0.25,
        ] {
            replay.update(delta);
            assert_eq!(replay.camera_target_y(), captured_target_y);
        }
    }

    #[test]
    fn replay_transitions_through_rewind_impact_and_falling() {
        let mut replay = DeathReplay::new(
            vec![sample(0.0, 0.0), sample(100.0, 1.0)],
            failure(FailureReason::ZeroOverlap),
        );

        assert!(replay.is_failure_freeze());
        replay.update(FAILURE_FREEZE_DURATION);
        assert!(replay.is_rewinding());
        replay.update(REWIND_DURATION);
        assert!(replay.is_slow_motion());
        replay.update(REPLAY_DURATION_SECONDS);
        assert!(replay.is_final_impact());
        replay.update(FINAL_IMPACT_DURATION);
        assert!(replay.is_falling());
        let initial_fall_offset = replay.fall_offset_y();
        replay.update(0.25);
        assert!(replay.fall_offset_y() > initial_fall_offset);
        replay.update(FALL_DURATION);
        assert!(replay.is_complete());
    }

    #[test]
    fn replay_fall_keeps_block_footprint_fixed_at_all_frame_rates() {
        for fps in [30.0, 60.0, 120.0] {
            let mut replay = DeathReplay::new(
                vec![sample(0.0, 0.0), sample(100.0, 1.0)],
                failure(FailureReason::ZeroOverlap),
            );
            replay.update(FAILURE_FREEZE_DURATION);
            replay.update(REWIND_DURATION);
            replay.update(REPLAY_DURATION_SECONDS);
            replay.update(FINAL_IMPACT_DURATION);
            assert!(replay.is_falling());

            let initial_block = replay.current_block();
            let initial_offset = replay.fall_offset_y();
            let mut previous_velocity = replay.fall_velocity_y;
            for _ in 0..fps as usize {
                replay.update(1.0 / fps);
                assert_eq!(replay.current_block(), initial_block);
                assert!(replay.fall_velocity_y >= previous_velocity);
                previous_velocity = replay.fall_velocity_y;
            }

            assert!(replay.fall_offset_y() > initial_offset);
        }
    }

    #[test]
    fn replay_skip_is_immediate_and_does_not_place_anything() {
        let mut replay = DeathReplay::new(
            vec![sample(0.0, 0.0), sample(100.0, 1.0)],
            failure(FailureReason::ZeroOverlap),
        );
        replay.skip();
        assert!(replay.is_complete());
    }

    #[test]
    fn replay_skip_is_available_from_every_phase() {
        let mut replay = DeathReplay::new(
            vec![sample(0.0, 0.0), sample(100.0, 1.0)],
            failure(FailureReason::ZeroOverlap),
        );
        assert!(replay.is_failure_freeze());
        replay.skip();
        assert!(replay.is_complete());

        let mut replay = DeathReplay::new(
            vec![sample(0.0, 0.0), sample(100.0, 1.0)],
            failure(FailureReason::ZeroOverlap),
        );
        replay.update(FAILURE_FREEZE_DURATION);
        assert!(replay.is_rewinding());
        replay.skip();
        assert!(replay.is_complete());

        let mut replay = DeathReplay::new(
            vec![sample(0.0, 0.0), sample(100.0, 1.0)],
            failure(FailureReason::ZeroOverlap),
        );
        replay.update(FAILURE_FREEZE_DURATION);
        replay.update(REWIND_DURATION);
        assert!(replay.is_slow_motion());
        replay.skip();
        assert!(replay.is_complete());

        let mut replay = DeathReplay::new(
            vec![sample(0.0, 0.0), sample(100.0, 1.0)],
            failure(FailureReason::ZeroOverlap),
        );
        replay.update(FAILURE_FREEZE_DURATION);
        replay.update(REWIND_DURATION);
        replay.update(REPLAY_DURATION_SECONDS);
        assert!(replay.is_final_impact());
        replay.skip();
        assert!(replay.is_complete());

        let mut replay = DeathReplay::new(
            vec![sample(0.0, 0.0), sample(100.0, 1.0)],
            failure(FailureReason::ZeroOverlap),
        );
        replay.update(FAILURE_FREEZE_DURATION);
        replay.update(REWIND_DURATION);
        replay.update(REPLAY_DURATION_SECONDS);
        replay.update(FINAL_IMPACT_DURATION);
        assert!(replay.is_falling());
        replay.skip();
        assert!(replay.is_complete());
    }

    #[test]
    fn replay_progress_tracks_slow_motion_only() {
        let mut replay = DeathReplay::new(
            vec![sample(0.0, 0.0), sample(100.0, 1.0)],
            failure(FailureReason::ZeroOverlap),
        );

        assert_eq!(replay.progress(), 0.0);
        replay.update(FAILURE_FREEZE_DURATION);
        replay.update(REWIND_DURATION);
        assert!(replay.is_slow_motion());
        assert_eq!(replay.progress(), 0.0);
        replay.update(REPLAY_DURATION_SECONDS * 0.5);
        assert!((replay.progress() - 0.5).abs() < 0.01);
        replay.update(REPLAY_DURATION_SECONDS * 0.5);
        assert!(replay.is_final_impact());
        assert_eq!(replay.progress(), 1.0);
    }

    #[test]
    fn replay_duration_is_consistent_across_frame_rates() {
        for fps in [30.0, 60.0, 120.0] {
            let mut replay = DeathReplay::new(
                vec![sample(0.0, 0.0), sample(100.0, 1.0)],
                failure(FailureReason::ZeroOverlap),
            );
            replay.update(FAILURE_FREEZE_DURATION);
            replay.update(REWIND_DURATION);
            assert!(replay.is_slow_motion());
            let mut elapsed = 0.0;
            while replay.is_slow_motion() && elapsed < 20.0 {
                replay.update(1.0 / fps);
                elapsed += 1.0 / fps;
            }
            assert!((2.6..=2.9).contains(&elapsed));
        }
    }

    #[test]
    fn replay_is_separate_from_core_state_and_score() {
        let mut game = Game::new(Default::default());
        game.start();
        let phase = game.phase();
        let score = game.score();
        let tower = game.layers().to_vec();
        let mut replay = DeathReplay::new(
            vec![sample(0.0, 0.0), sample(100.0, 1.0)],
            failure(FailureReason::ZeroOverlap),
        );

        for _ in 0..360 {
            replay.update(1.0 / 60.0);
        }

        assert!(replay.is_complete());
        assert_eq!(game.phase(), phase);
        assert_eq!(game.score(), score);
        assert_eq!(game.layers(), tower.as_slice());
    }
}
