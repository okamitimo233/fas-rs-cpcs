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
- **Do** run `cargo xtask format` before committing
- **Do** run `cargo xtask lint --fix` to auto-fix warnings
- **Do** write Chinese or English commit messages only (per CONTRIBUTING.md)
