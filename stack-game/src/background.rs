use macroquad::prelude::*;
use std::path::Path;

const LOGICAL_WIDTH: f32 = 320.0;
const LOGICAL_HEIGHT: f32 = 224.0;
const INITIAL_CAMERA_TARGET_Y: f32 = 112.0;
const BASE_ASSET_PATH: &str = "assets/background/sky_world.png";

// Transparent scenery layers use 224-pixel art cards in an overlapping world
// grid. The opaque base scene is drawn only once per frame, so it can never
// create a full-width horizontal slice when the camera moves.
const SCENERY_BAND_HEIGHT: f32 = 216.0;
const REGION_BLEND_HEIGHT: f32 = 6.0;

const OPTIONAL_LAYER_PATHS: &[(&str, f32)] = &[
    ("assets/background/distant_clouds.png", 0.24),
    ("assets/background/floating_cliffs.png", 0.48),
    ("assets/background/foreground_clouds.png", 0.78),
];

#[derive(Clone, Copy, Debug)]
struct SkyRegion {
    start_height: f32,
    fill: Color,
    atmosphere: Color,
    atmosphere_alpha: f32,
    far_parallax: f32,
    cloud_density: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct SkyStyle {
    fill: Color,
    atmosphere: Color,
    atmosphere_alpha: f32,
    far_parallax: f32,
    cloud_density: f32,
}

const fn sky_region(
    start_height: f32,
    fill: Color,
    atmosphere: Color,
    atmosphere_alpha: f32,
    far_parallax: f32,
    cloud_density: f32,
) -> SkyRegion {
    SkyRegion {
        start_height,
        fill,
        atmosphere,
        atmosphere_alpha,
        far_parallax,
        cloud_density,
    }
}

// Continuous altitude regions. The six-layer blend makes each 10–15 layer
// change gradual, while the later entries keep the world evolving beyond the
// initially authored climb.
const SKY_REGIONS: [SkyRegion; 11] = [
    sky_region(
        0.0,
        Color::new(0.06, 0.30, 0.82, 1.0),
        Color::new(0.05, 0.18, 0.48, 1.0),
        0.06,
        0.12,
        1.0,
    ),
    sky_region(
        15.0,
        Color::new(0.08, 0.35, 0.88, 1.0),
        Color::new(0.10, 0.24, 0.58, 1.0),
        0.06,
        0.13,
        0.96,
    ),
    sky_region(
        30.0,
        Color::new(0.12, 0.42, 0.92, 1.0),
        Color::new(0.16, 0.34, 0.72, 1.0),
        0.05,
        0.14,
        0.88,
    ),
    sky_region(
        45.0,
        Color::new(0.22, 0.52, 0.96, 1.0),
        Color::new(0.34, 0.52, 0.86, 1.0),
        0.05,
        0.15,
        0.78,
    ),
    sky_region(
        60.0,
        Color::new(0.34, 0.63, 1.0, 1.0),
        Color::new(0.56, 0.70, 0.96, 1.0),
        0.05,
        0.16,
        0.70,
    ),
    sky_region(
        75.0,
        Color::new(0.46, 0.70, 1.0, 1.0),
        Color::new(0.72, 0.80, 1.0, 1.0),
        0.045,
        0.17,
        0.61,
    ),
    sky_region(
        90.0,
        Color::new(0.58, 0.76, 1.0, 1.0),
        Color::new(0.90, 0.88, 1.0, 1.0),
        0.045,
        0.18,
        0.52,
    ),
    sky_region(
        105.0,
        Color::new(0.66, 0.78, 1.0, 1.0),
        Color::new(1.0, 0.78, 0.56, 1.0),
        0.055,
        0.19,
        0.45,
    ),
    sky_region(
        180.0,
        Color::new(0.72, 0.78, 0.98, 1.0),
        Color::new(1.0, 0.68, 0.48, 1.0),
        0.07,
        0.20,
        0.38,
    ),
    sky_region(
        300.0,
        Color::new(0.66, 0.68, 0.94, 1.0),
        Color::new(0.72, 0.54, 0.78, 1.0),
        0.075,
        0.21,
        0.34,
    ),
    sky_region(
        500.0,
        Color::new(0.78, 0.70, 0.92, 1.0),
        Color::new(1.0, 0.72, 0.44, 1.0),
        0.085,
        0.22,
        0.30,
    ),
];

struct ParallaxLayer {
    texture: Texture2D,
    factor: f32,
}

pub struct Background {
    base: Option<Texture2D>,
    overlays: Vec<ParallaxLayer>,
}

impl Background {
    pub async fn load() -> Self {
        let base = match resolve_asset_path(BASE_ASSET_PATH) {
            Some(path) => match load_texture(&path).await {
                Ok(texture) => {
                    texture.set_filter(FilterMode::Nearest);
                    Some(texture)
                }
                Err(error) => {
                    eprintln!("[stack-it-16] background asset failed to load from {path}: {error}");
                    None
                }
            },
            None => {
                eprintln!("[stack-it-16] background asset unavailable at {BASE_ASSET_PATH}");
                None
            }
        };

        let mut overlays = Vec::new();
        for &(path, factor) in OPTIONAL_LAYER_PATHS {
            let Some(resolved_path) = resolve_asset_path(path) else {
                eprintln!("[stack-it-16] optional background layer unavailable at {path}");
                continue;
            };
            match load_texture(&resolved_path).await {
                Ok(texture) => {
                    texture.set_filter(FilterMode::Nearest);
                    overlays.push(ParallaxLayer { texture, factor });
                }
                Err(error) => {
                    eprintln!(
                        "[stack-it-16] optional background layer failed to load from {resolved_path}: {error}"
                    );
                }
            }
        }

        Self { base, overlays }
    }

    /// Draw a continuous vertical sky world in logical 320×224 coordinates.
    /// `tower_height` is the absolute number of successfully placed layers,
    /// not the number currently visible on screen. Death Replay passes the
    /// same value, so the background remains visually frozen.
    pub fn draw(&self, _time: f32, camera_target_y: f32, tower_height: usize) {
        let camera_delta = camera_target_y - INITIAL_CAMERA_TARGET_Y;
        let style = sky_style(tower_height as f32);
        let screen_top = camera_target_y - INITIAL_CAMERA_TARGET_Y;

        draw_rectangle(0.0, screen_top, LOGICAL_WIDTH, LOGICAL_HEIGHT, style.fill);
        if let Some(texture) = &self.base {
            draw_texture(texture, 0.0, screen_top, Color::new(1.0, 1.0, 1.0, 0.94));
        } else {
            draw_fallback(screen_top, style.fill);
        }

        for (index, layer) in self.overlays.iter().enumerate() {
            let density = (style.cloud_density * (1.0 - index as f32 * 0.08)).clamp(0.18, 1.0);
            draw_tiled_layer(
                &layer.texture,
                layer.factor,
                camera_delta,
                Color::new(1.0, 1.0, 1.0, density),
            );
        }

        // This is drawn before the tower, so altitude mood changes do not
        // tint blocks, HUD elements, or gameplay effects.
        draw_rectangle(
            0.0,
            screen_top,
            LOGICAL_WIDTH,
            LOGICAL_HEIGHT,
            Color::new(
                style.atmosphere.r,
                style.atmosphere.g,
                style.atmosphere.b,
                style.atmosphere_alpha,
            ),
        );
    }
}

fn draw_tiled_layer(texture: &Texture2D, factor: f32, camera_delta: f32, tint: Color) {
    let factor = factor.clamp(0.0, 1.0);
    let center_band = (camera_delta * factor / SCENERY_BAND_HEIGHT).floor() as i32;
    for band in (center_band - 1)..=(center_band + 2) {
        let world_y = scenery_world_y(band) + camera_delta * (1.0 - factor);
        draw_texture(texture, 0.0, world_y, tint);
    }
}

fn scenery_world_y(band: i32) -> f32 {
    band as f32 * SCENERY_BAND_HEIGHT
}

fn sky_style(height: f32) -> SkyStyle {
    let height = height.max(0.0);
    for pair in SKY_REGIONS.windows(2) {
        let lower = pair[0];
        let upper = pair[1];
        if height < upper.start_height {
            let blend = ((height - upper.start_height + REGION_BLEND_HEIGHT) / REGION_BLEND_HEIGHT)
                .clamp(0.0, 1.0);
            return mix_style(lower, upper, blend);
        }
    }

    style_from_region(*SKY_REGIONS.last().expect("sky regions must not be empty"))
}

fn style_from_region(region: SkyRegion) -> SkyStyle {
    SkyStyle {
        fill: region.fill,
        atmosphere: region.atmosphere,
        atmosphere_alpha: region.atmosphere_alpha,
        far_parallax: region.far_parallax,
        cloud_density: region.cloud_density,
    }
}

fn mix_style(lower: SkyRegion, upper: SkyRegion, amount: f32) -> SkyStyle {
    SkyStyle {
        fill: mix_color(lower.fill, upper.fill, amount),
        atmosphere: mix_color(lower.atmosphere, upper.atmosphere, amount),
        atmosphere_alpha: lower.atmosphere_alpha
            + (upper.atmosphere_alpha - lower.atmosphere_alpha) * amount,
        far_parallax: lower.far_parallax + (upper.far_parallax - lower.far_parallax) * amount,
        cloud_density: lower.cloud_density + (upper.cloud_density - lower.cloud_density) * amount,
    }
}

fn mix_color(lower: Color, upper: Color, amount: f32) -> Color {
    Color::new(
        lower.r + (upper.r - lower.r) * amount,
        lower.g + (upper.g - lower.g) * amount,
        lower.b + (upper.b - lower.b) * amount,
        1.0,
    )
}

fn resolve_asset_path(path: &str) -> Option<String> {
    if Path::new(path).is_file() {
        return Some(path.to_owned());
    }

    let package_relative = format!("../{path}");
    Path::new(&package_relative)
        .is_file()
        .then_some(package_relative)
}

fn draw_fallback(screen_top: f32, fill: Color) {
    draw_rectangle(0.0, screen_top, LOGICAL_WIDTH, 96.0, fill);
    draw_rectangle(
        0.0,
        screen_top + 96.0,
        LOGICAL_WIDTH,
        LOGICAL_HEIGHT - 96.0,
        Color::new(
            (fill.r + 0.18).min(1.0),
            (fill.g + 0.18).min(1.0),
            (fill.b + 0.08).min(1.0),
            1.0,
        ),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn base_background_asset_is_present_at_the_runtime_path() {
        let resolved = resolve_asset_path(BASE_ASSET_PATH)
            .expect("background asset must exist from root or package directory");
        let asset = Path::new(&resolved);
        assert!(
            std::fs::metadata(asset).expect("background metadata").len() > 0,
            "background asset is empty: {resolved}"
        );
    }

    #[test]
    fn parallax_factors_are_bounded_and_ordered_by_depth() {
        assert!(OPTIONAL_LAYER_PATHS
            .iter()
            .all(|(_, factor)| (0.0..=1.0).contains(factor)));
        assert!(OPTIONAL_LAYER_PATHS[0].1 < OPTIONAL_LAYER_PATHS[1].1);
        assert!(OPTIONAL_LAYER_PATHS[1].1 < OPTIONAL_LAYER_PATHS[2].1);

        let camera_delta = -3_000.0;
        let movement: Vec<f32> = OPTIONAL_LAYER_PATHS
            .iter()
            .map(|(_, factor)| (camera_delta * factor).abs())
            .collect();
        assert!(movement[0] < movement[1]);
        assert!(movement[1] < movement[2]);
    }

    #[test]
    fn sky_regions_are_selected_at_authored_heights() {
        for height in [0.0, 15.0, 30.0, 45.0, 60.0, 75.0, 90.0, 105.0, 200.0, 500.0] {
            let style = sky_style(height);
            assert!(style.fill.r.is_finite());
            assert!(style.cloud_density.is_finite());
        }
        assert_ne!(sky_style(0.0), sky_style(105.0));
        assert_ne!(sky_style(105.0), sky_style(500.0));
    }

    #[test]
    fn sky_region_transitions_are_smooth() {
        for height in 0..500 {
            let lower = sky_style(height as f32);
            let upper = sky_style(height as f32 + 1.0);
            let maximum_delta = [
                (upper.fill.r - lower.fill.r).abs(),
                (upper.fill.g - lower.fill.g).abs(),
                (upper.fill.b - lower.fill.b).abs(),
                (upper.cloud_density - lower.cloud_density).abs(),
            ]
            .into_iter()
            .fold(0.0, f32::max);
            assert!(maximum_delta < 0.08, "abrupt sky change at height {height}");
        }
    }

    #[test]
    fn scenery_world_positions_are_deterministic_and_continuous() {
        assert_eq!(scenery_world_y(-4), scenery_world_y(-4));
        assert_eq!(
            scenery_world_y(0) - scenery_world_y(-1),
            SCENERY_BAND_HEIGHT
        );
        assert_eq!(scenery_world_y(500 / 12), scenery_world_y(500 / 12));
    }

    #[test]
    fn world_parallax_does_not_reset_at_long_tower_heights() {
        let first = scenery_world_y(-1);
        let at_100 = scenery_world_y(-8);
        let at_500 = scenery_world_y(-40);
        assert!(first > at_100);
        assert!(at_100 > at_500);
        assert!(sky_style(500.0).far_parallax >= sky_style(100.0).far_parallax);
    }

    #[test]
    fn replay_height_input_is_stable_for_a_frozen_tower() {
        let height = 177.0;
        let camera_target = -3_000.0;
        assert_eq!(sky_style(height), sky_style(height));
        let delta = camera_target - INITIAL_CAMERA_TARGET_Y;
        let first = delta * OPTIONAL_LAYER_PATHS[1].1;
        let second = delta * OPTIONAL_LAYER_PATHS[1].1;
        assert_eq!(first, second);
    }

    #[test]
    fn transparent_world_cards_cover_the_viewport_without_gaps() {
        for factor in [0.12, 0.24, 0.48, 0.78] {
            for camera_delta in [-10_000.0, -3_000.0, -216.0, 0.0, 216.0] {
                let center = (camera_delta * factor / SCENERY_BAND_HEIGHT).floor() as i32;
                let positions: Vec<f32> = ((center - 1)..=(center + 2))
                    .map(|band| {
                        scenery_world_y(band) + camera_delta * (1.0 - factor) - camera_delta
                    })
                    .collect();
                assert!(positions[0] <= 0.0);
                assert!(positions[2] <= LOGICAL_HEIGHT);
                assert!(positions[2] + LOGICAL_HEIGHT >= LOGICAL_HEIGHT);
                for pair in positions.windows(2) {
                    assert!(pair[1] <= pair[0] + LOGICAL_HEIGHT);
                }
            }
        }
    }
}
