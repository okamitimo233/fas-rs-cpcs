# Quality Guidelines

> Code quality standards for backend development.

---

## Overview

This project enforces strict Clippy linting with `deny` on all and pedantic lints. Code must pass `cargo xtask format` and `cargo xtask lint` before commit. The codebase uses Rust edition 2024 with modern language features.

---

## Clippy Configuration

### Lint Levels

```rust
// src/main.rs:18-26
#![deny(clippy::all, clippy::pedantic)]
#![warn(clippy::nursery)]
#![allow(
    clippy::module_name_repetitions,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    clippy::cast_possible_wrap
)]
```

### Meaning
- **deny**: Compilation will fail if these lints trigger
- **warn**: Lints produce warnings but don't block compilation
- **allow**: Explicitly permitted patterns that would otherwise be flagged

---

## Forbidden Patterns

### 1. Unnecessary Mutability
```rust
// BAD: Unused mut
let mut config = load_config()?;

// GOOD: Only mut when needed
let config = load_config()?;
```

### 2. Implicit Clone on Large Types
```rust
// BAD: Cloning without thinking
fn process(data: HashMap<String, Vec<i32>>) {
    let copy = data.clone();
}

// GOOD: Use references or explicit Clone trait bounds
fn process(data: &HashMap<String, Vec<i32>>) {
    // ...
}
```

### 3. Missing Documentation on Public Items
```rust
// BAD
pub fn calculate_fps(frames: &[Duration]) -> f64 { ... }

// GOOD
/// Calculates the average FPS from a slice of frame durations.
pub fn calculate_fps(frames: &[Duration]) -> f64 { ... }
```

---

## Required Patterns

### 1. Builder Pattern for Complex Construction

```rust
// Example: src/framework/scheduler/mod.rs
pub struct Scheduler {
    controller: Option<Controller>,
    config: Option<Config>,
}

impl Scheduler {
    #[must_use]
    pub const fn new() -> Self { ... }
    
    #[must_use]
    pub fn config(mut self, c: Config) -> Self { ... }
}
```

### 2. Type Aliases for Result Types

```rust
// Example: src/framework/error.rs
pub type Result<T> = std::result::Result<T, Error>;
```

### 3. Conditional Compilation for Debug Code

```rust
// Example: Pattern used throughout
#[cfg(debug_assertions)]
use log::debug;

#[cfg(debug_assertions)]
debug!("Diagnostic info: {:?}", data);
```

### 4. License Headers on All Files

```rust
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
```

### 5. Performance Optimization: Downsampled Recalculation

When expensive calculations don't need per-frame precision, use counter-based downsampling:

```rust
// Example: src/framework/scheduler/looper/buffer/mod.rs
pub struct FrameTimeState {
    // ... other fields ...
    volatility_update_counter: u8,
}

// In update loop:
if self.volatility_update_counter == 0 {
    // Expensive calculation (e.g., variance) runs every N frames
    self.volatility_cv = calculate_volatility(&self.frames, params.variance_window);
    self.volatility_update_counter = params.update_interval;
} else {
    self.volatility_update_counter -= 1;
}
```

**Why**: Reduces CPU overhead for expensive operations while maintaining acceptable responsiveness.

**When to Apply**: 
- Calculations that don't change significantly frame-to-frame
- Operations with O(n) or worse complexity per frame
- Metrics with natural temporal coherence (variance, trends, averages)

### 6. Backward Compatibility via Experimental Flags

When introducing new algorithms that change core behavior:

```rust
// Example: src/framework/config/data/mod.rs
pub struct Config {
    // ... existing fields ...
    pub experimental_scheduler: bool,
}

// In algorithm selection:
pub fn get_normalized_last_frame(&self, config: &Config) -> Duration {
    let last_frame = self.last_frame;
    let short_avg = self.short_avg_frame();
    
    if config.experimental_scheduler {
        // New adaptive algorithm
        let cv = self.volatility_cv;
        let beta = cv_to_blend_beta(cv, &AdaptiveBlendParams::default());
        last_frame.mul_f64(1.0 - beta) + short_avg.mul_f64(beta)
    } else {
        // Old fixed-ratio algorithm (default)
        const SHORT_AVG_BLEND: f64 = 0.30;
        last_frame.mul_f64(1.0 - SHORT_AVG_BLEND) + short_avg.mul_f64(SHORT_AVG_BLEND)
    }
}
```

**Why**: 
- Users must explicitly opt-in to new behavior
- Default behavior remains unchanged and stable
- Easy to A/B test in production
- Safe rollback if issues arise

**When to Apply**: 
- New algorithms that replace existing core logic
- Behavioral changes that affect scheduling/performance
- Features that need real-world validation before becoming default

---

## Design Decisions

### 1. Extensibility-First Parameter Design

**Context**: When implementing new features with configurable parameters, balance between flexibility and user complexity.

**Decision**: Define parameter structures with default values, keep them internal initially, but design for future exposure.

**Example**:
```rust
// src/framework/scheduler/looper/buffer/calculate.rs
#[derive(Debug, Clone, Copy)]
pub struct AdaptiveBlendParams {
    pub beta_min: f64,           // Default 0.10
    pub beta_max: f64,           // Default 0.60
    pub cv_threshold_low: f64,   // Default 0.05
    pub cv_threshold_high: f64,  // Default 0.30
    pub variance_window: usize,  // Default 30
    pub update_interval: u8,     // Default 10
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
```

**Why**: 
- Clean code structure from the start
- No user configuration complexity initially
- Easy to expose as config later if needed
- Centralized parameter management

**When to Apply**: New features with tunable parameters that may need user exposure in the future.

### 2. Cross-Framerate Normalization Using Dimensionless Metrics

**Context**: Need to compare or adjust behavior across different framerates (30fps, 60fps, 120fps).

**Decision**: Use coefficient of variation (CV = std_dev / mean) instead of absolute variance.

**Example**:
```rust
// src/framework/scheduler/looper/buffer/calculate.rs
fn calculate_volatility(frames: &[Duration], window: usize) -> Option<f64> {
    let recent: Vec<Duration> = frames.iter().copied().take(window).collect();
    let mean = recent.iter().sum::<Duration>() / recent.len();
    let variance = recent.iter()
        .map(|&d| {
            let diff = d.as_nanos() as f64 - mean.as_nanos() as f64;
            diff * diff
        })
        .sum::<f64>() / recent.len() as f64;
    
    // CV is dimensionless, works across any framerate
    let cv = variance.sqrt() / mean.as_nanos() as f64;
    Some(cv)
}
```

**Why**:
- Dimensionless metric allows cross-framerate comparison
- 60fps ±2ms has same relative variability as 120fps ±1ms
- More robust than absolute variance thresholds

**When to Apply**: Any algorithm that needs framerate-independent behavior.

---

## Testing Requirements

### Current State
- No dedicated test suite exists in the project
- Testing relies on debug builds and manual verification
- Integration testing done through actual device deployment

### When Tests Are Added
- Unit tests should be in the same file using `#[cfg(test)] mod tests { ... }`
- Use `#[test]` attribute for test functions
- Prefer integration tests in `tests/` directory for complex scenarios

---

## Code Review Checklist

- [ ] Code compiles without warnings
- [ ] `cargo xtask format` applied
- [ ] `cargo xtask lint` passes
- [ ] License header present on new files
- [ ] Error handling uses proper `Result` propagation
- [ ] Debug logging uses `#[cfg(debug_assertions)]`
- [ ] No hardcoded values that should be configurable
- [ ] Public functions have documentation comments

---

## Commit Message Format

Follow Angular commit message guidelines:

```
<type>(<scope>): <subject>

<body>

<footer>
```

### Types
- `feat`: New feature
- `fix`: Bug fix
- `perf`: Performance improvement
- `refactor`: Code refactoring
- `chore`: Maintenance tasks
- `docs`: Documentation changes
- `style`: Formatting changes
- `test`: Adding tests

### Examples
```
perf(cpcs): increase worker recv timeout to 150ms
fix(cpcs): reconcile stale attachments in worker loop
refactor(cpcs): replace cmd channel with shared desired-set worker sync
```

---

## Common Mistakes

- **Don't** bypass Clippy warnings with `#[allow(...)]` without good reason
- **Don't** commit without running format and lint
- **Don't** use `unwrap()` in production code paths
- **Don't** use redundant `.copied()` chains when collecting iterators
  ```rust
  // BAD: Redundant .copied() calls
  let recent: Vec<Duration> = frames.iter().copied().take(n).collect();
  
  // GOOD: Collect directly
  let recent: Vec<Duration> = frames.iter().take(n).copied().collect();
  ```
- **Don't** forget to add `use std::convert::TryFrom;` when using `.try_from()` in standalone tools
- **Do** run `cargo xtask format` before committing
- **Do** run `cargo xtask lint --fix` to auto-fix warnings
- **Do** write Chinese or English commit messages only (per CONTRIBUTING.md)
- **Do** use `for _` instead of `for i` when the loop variable is unused
