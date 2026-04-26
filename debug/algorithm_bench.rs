// Copyright 2024-2025, shadow3aaa
//
// This file is part of fas-rs.
//
// fas-rs is free software: you can redistribute it and/or modify it under
// the terms of the GNU General Public License as published by the Free
// Software Foundation, either version 3 of the License, or (at your option)
// any later version.
//
// fas-rs is distributed in the hope that it will be useful, but WITHOUT ANY
// WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS
// FOR A PARTICULAR PURPOSE. See the GNU General Public License for more
// details.
//
// You should have received a copy of the GNU General Public License along
// with fas-rs. If not, see <https://www.gnu.org/licenses/>.

//! Algorithm benchmark for comparing fixed vs adaptive frametime blending.
//!
//! This tool simulates various game scenarios and outputs a comparison table
//! showing how the old (fixed) and new (adaptive) algorithms perform.

use std::convert::TryFrom;
use std::time::Duration;

/// Parameters for adaptive frametime blending (mirrored from main codebase)
#[derive(Debug, Clone, Copy)]
struct AdaptiveBlendParams {
    beta_min: f64,
    beta_max: f64,
    cv_threshold_low: f64,
    cv_threshold_high: f64,
    variance_window: usize,
    update_interval: u8,
}

impl Default for AdaptiveBlendParams {
    fn default() -> Self {
        Self {
            beta_min: 0.10,
            beta_max: 0.60,
            cv_threshold_low: 0.05,
            cv_threshold_high: 0.30,
            variance_window: 30,
            update_interval: 10,
        }
    }
}

#[derive(Debug, Clone)]
struct Scenario {
    name: String,
    target_fps: u32,
    frametimes_ms: Vec<f64>,
}

/// Calculate coefficient of variation from frametimes
fn calculate_cv(frametimes: &[Duration]) -> f64 {
    if frametimes.len() < 2 {
        return 0.0;
    }

    let mean: Duration = frametimes.iter().sum::<Duration>()
        / u32::try_from(frametimes.len()).unwrap_or(1);

    if mean.is_zero() {
        return 0.0;
    }

    let variance: f64 = frametimes
        .iter()
        .map(|&d| {
            let diff = if d > mean {
                d.saturating_sub(mean)
            } else {
                mean.saturating_sub(d)
            };
            let diff_secs = diff.as_secs_f64();
            diff_secs * diff_secs
        })
        .sum::<f64>()
        / f64::from(u32::try_from(frametimes.len()).unwrap_or(1));

    let std_dev = variance.sqrt();
    std_dev / mean.as_secs_f64()
}

/// Map CV to blend ratio beta (adaptive algorithm)
fn cv_to_blend_beta(cv: f64, params: &AdaptiveBlendParams) -> f64 {
    if cv <= params.cv_threshold_low {
        params.beta_min
    } else if cv >= params.cv_threshold_high {
        params.beta_max
    } else {
        let t = (cv - params.cv_threshold_low)
            / (params.cv_threshold_high - params.cv_threshold_low);
        params.beta_min + t * (params.beta_max - params.beta_min)
    }
}

/// Fixed blend algorithm (old)
fn fixed_blend_beta() -> f64 {
    0.30
}

/// Generate test scenarios
fn generate_scenarios() -> Vec<Scenario> {
    vec![
        Scenario {
            name: "Rhythm Game (Stable 60fps)".to_string(),
            target_fps: 60,
            frametimes_ms: generate_stable_frametimes(16.67, 0.3),
        },
        Scenario {
            name: "Fighting Game (60fps)".to_string(),
            target_fps: 60,
            frametimes_ms: generate_stable_frametimes(16.67, 1.0),
        },
        Scenario {
            name: "Open World (60fps)".to_string(),
            target_fps: 60,
            frametimes_ms: generate_stable_frametimes(16.67, 2.5),
        },
        Scenario {
            name: "Scene Transition (60fps)".to_string(),
            target_fps: 60,
            frametimes_ms: generate_stable_frametimes(16.67, 5.0),
        },
        Scenario {
            name: "Loading Stutters (60fps)".to_string(),
            target_fps: 60,
            frametimes_ms: generate_stable_frametimes(16.67, 10.0),
        },
        Scenario {
            name: "Stable 120fps".to_string(),
            target_fps: 120,
            frametimes_ms: generate_stable_frametimes(8.33, 0.2),
        },
    ]
}

/// Generate frametimes with specified mean and jitter
fn generate_stable_frametimes(mean_ms: f64, jitter_ms: f64) -> Vec<f64> {
    let mut frametimes = Vec::with_capacity(60);

    // Use simple pseudo-random for reproducibility
    let mut seed = 12345_u64;
    for _ in 0..60 {
        seed = seed.wrapping_mul(1103515245).wrapping_add(12345);
        let rand = f64::from(seed as i32 % 10000) / 10000.0; // 0.0 to 1.0

        let jitter = (rand - 0.5) * 2.0 * jitter_ms;
        let frametime = (mean_ms + jitter).max(1.0);
        frametimes.push(frametime);
    }

    frametimes
}

/// Calculate short average frametime
fn calculate_short_avg(frametimes: &[Duration], target_fps: u32) -> Duration {
    let window = (target_fps as usize).min(frametimes.len());
    if window == 0 {
        return Duration::ZERO;
    }

    let sum: Duration = frametimes.iter().take(window).sum();
    sum / u32::try_from(window).unwrap_or(1)
}

/// Evaluate algorithm performance
fn evaluate_scenario(scenario: &Scenario) -> (f64, f64, f64) {
    let params = AdaptiveBlendParams::default();

    let frametimes: Vec<Duration> = scenario
        .frametimes_ms
        .iter()
        .map(|&ms| Duration::from_secs_f64(ms / 1000.0))
        .collect();

    // Use last 30 frames for CV calculation (variance_window)
    let cv_window: Vec<Duration> = frametimes.iter().copied().take(30).collect();
    let cv = calculate_cv(&cv_window);

    let old_beta = fixed_blend_beta();
    let new_beta = cv_to_blend_beta(cv, &params);

    (cv, old_beta, new_beta)
}

fn main() {
    println!("# Algorithm Comparison Evaluation\n");
    println!("| Scenario | CV | Old Beta | New Beta | Improvement |");
    println!("|----------|-----|----------|----------|-------------|");

    let scenarios = generate_scenarios();

    for scenario in scenarios {
        let (cv, old_beta, new_beta) = evaluate_scenario(&scenario);

        let improvement = if cv < 0.05 {
            "Faster response".to_string()
        } else if cv > 0.30 {
            "More stable".to_string()
        } else {
            "Balanced".to_string()
        };

        println!(
            "| {} | {:.4} | {:.2} | {:.2} | {} |",
            scenario.name, cv, old_beta, new_beta, improvement
        );
    }

    println!("\n## Algorithm Details\n");
    println!("**Fixed Blend (Old)**:");
    println!("- Beta: always 0.30");
    println!("- Blend: `last_frame * 0.70 + short_avg * 0.30`\n");

    println!("**Adaptive Blend (New)**:");
    println!("- CV threshold low: 0.05 → beta = 0.10");
    println!("- CV threshold high: 0.30 → beta = 0.60");
    println!("- Linear interpolation between thresholds");
    println!("- Blend: `last_frame * (1 - beta) + short_avg * beta`\n");

    println!("## Interpretation\n");
    println!("- **CV < 0.05**: Stable frametimes (rhythm games) → Trust single frame, fast response");
    println!("- **0.05 < CV < 0.30**: Moderate variance → Balanced blend");
    println!("- **CV > 0.30**: High variance (scene changes, loading) → Trust short average, smooth stable");
}
