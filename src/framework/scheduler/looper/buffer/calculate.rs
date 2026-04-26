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

use std::time::Duration;

use likely_stable::unlikely;
#[cfg(debug_assertions)]
use log::debug;

use super::Buffer;
use crate::{Extension, api::trigger_target_fps_change, framework::config::TargetFps};

/// Parameters for adaptive frametime blending.
#[derive(Debug, Clone, Copy)]
pub struct AdaptiveBlendParams {
    pub beta_min: f64,
    pub beta_max: f64,
    pub cv_threshold_low: f64,
    pub cv_threshold_high: f64,
    pub variance_window: usize,
    pub update_interval: u8,
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

impl Buffer {
    pub fn calculate_current_fps(&mut self) {
        let avg_time_long = self.calculate_average_frametime(None);
        #[cfg(debug_assertions)]
        debug!("avg_time_long: {avg_time_long:?}");

        self.frametime_state.avg_time_long = avg_time_long;

        let current_fps_long = 1.0 / avg_time_long.as_secs_f64();
        #[cfg(debug_assertions)]
        debug!("current_fps_long: {current_fps_long:.2}");

        self.frametime_state.current_fps_long = current_fps_long;

        let avg_time_short = self
            .calculate_average_frametime(self.target_fps().map(|target_fps| target_fps as usize));
        #[cfg(debug_assertions)]
        debug!("avg_time_short: {avg_time_short:?}");

        self.frametime_state.avg_time_short = avg_time_short;

        let current_fps_short = 1.0 / avg_time_short.as_secs_f64();
        #[cfg(debug_assertions)]
        debug!("current_fps_short: {current_fps_short:.2}");

        self.frametime_state.current_fps_short = current_fps_short;
    }

    fn calculate_average_frametime(&self, it_takes: Option<usize>) -> Duration {
        let total_time: Duration = self
            .frametime_state
            .frametimes
            .iter()
            .take(it_takes.unwrap_or(self.frametime_state.frametimes.len()))
            .sum::<Duration>()
            .saturating_add(self.frametime_state.additional_frametime);

        total_time
            .checked_div(
                it_takes
                    .unwrap_or(self.frametime_state.frametimes.len())
                    .min(self.frametime_state.frametimes.len())
                    .try_into()
                    .unwrap(),
            )
            .unwrap_or_default()
    }

    pub fn calculate_target_fps(&mut self, extension: &Extension) {
        let new_target_fps = self.target_fps();
        if self.target_fps_state.target_fps != new_target_fps || new_target_fps.is_none() {
            self.reset_frametime_state();
            if let Some(target_fps) = new_target_fps {
                self.trigger_target_fps_change(extension, target_fps);
            }
            self.target_fps_state.target_fps = new_target_fps;
            self.unusable();
        }
    }

    fn reset_frametime_state(&mut self) {
        self.frametime_state.frametimes.clear();
    }

    fn trigger_target_fps_change(&self, extension: &Extension, target_fps: u32) {
        trigger_target_fps_change(extension, target_fps, self.package_info.pkg.clone());
    }

    fn target_fps(&self) -> Option<u32> {
        let target_fpses = match &self.target_fps_state.target_fps_config {
            TargetFps::Value(t) => vec![*t],
            TargetFps::Array(arr) => arr.clone(),
        };

        let current_fps = self.frametime_state.current_fps_long;

        if unlikely(current_fps < (target_fpses.first()?.saturating_sub(10).max(10)).into()) {
            return None;
        }

        for &target_fps in &target_fpses {
            if current_fps <= f64::from(target_fps) + 3.0 {
                #[cfg(debug_assertions)]
                debug!("Matched target_fps: current: {current_fps:.2} target_fps: {target_fps}");
                return Some(target_fps);
            }
        }

        target_fpses.last().copied()
    }

    /// Calculates coefficient of variation (CV) for frametime volatility.
    /// Returns CV = std_dev / mean, a dimensionless measure of volatility.
    pub fn calculate_volatility(&mut self, params: &AdaptiveBlendParams) {
        self.frametime_state.volatility_update_counter += 1;

        // Only update every N frames
        if self.frametime_state.volatility_update_counter < params.update_interval {
            return;
        }
        self.frametime_state.volatility_update_counter = 0;

        let window = params
            .variance_window
            .min(self.frametime_state.frametimes.len());
        if window < 2 {
            self.frametime_state.volatility_cv = 0.0;
            return;
        }

        // Collect frametimes for double-pass calculation (mean + variance)
        let frametimes: Vec<Duration> = self
            .frametime_state
            .frametimes
            .iter()
            .copied()
            .take(window)
            .collect();

        // Calculate mean
        let mean: Duration =
            frametimes.iter().sum::<Duration>() / u32::try_from(window).unwrap_or(1);

        if mean.is_zero() {
            self.frametime_state.volatility_cv = 0.0;
            return;
        }

        // Calculate variance
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
            / f64::from(u32::try_from(window).unwrap_or(1));

        // CV = std_dev / mean
        let std_dev = variance.sqrt();
        let cv = std_dev / mean.as_secs_f64();

        self.frametime_state.volatility_cv = cv;

        #[cfg(debug_assertions)]
        debug!("Volatility CV: {:.4}, mean: {:?}", cv, mean);
    }

    /// Maps coefficient of variation to blend ratio beta.
    /// Low CV → low beta (trust single frame)
    /// High CV → high beta (trust short average)
    #[must_use]
    pub const fn cv_to_blend_beta(cv: f64, params: &AdaptiveBlendParams) -> f64 {
        if cv <= params.cv_threshold_low {
            params.beta_min
        } else if cv >= params.cv_threshold_high {
            params.beta_max
        } else {
            // Linear interpolation between thresholds
            let t = (cv - params.cv_threshold_low)
                / (params.cv_threshold_high - params.cv_threshold_low);
            params.beta_min + t * (params.beta_max - params.beta_min)
        }
    }
}
