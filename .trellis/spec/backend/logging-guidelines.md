# Logging Guidelines

> How logging is done in this project.

---

## Overview

This project uses the `log` crate facade with `flexi_logger` as the backend. Logs are written to stdout with a custom timestamp format. Debug logs are conditionally compiled based on build profile.

---

## Log Levels

| Level | When to Use | Example |
|-------|-------------|---------|
| `error!` | Unrecoverable errors, critical failures | Parse failures, missing required files |
| `warn!` | Recoverable issues, unexpected states | Retry attempts, fallback behavior |
| `info!` | Important lifecycle events | Mode switches, config loaded, service started |
| `debug!` | Detailed diagnostic information | Variable values, algorithm intermediate results |

---

## Logging Setup

### Logger Initialization

```rust
// Example: src/main.rs:82-91
#[cfg(not(debug_assertions))]
let logger_spec = LogSpecification::info();

#[cfg(debug_assertions)]
let logger_spec = LogSpecification::debug();

Logger::with(logger_spec)
    .log_to_stdout()
    .format(log_format)
    .start()?;
```

### Custom Format

```rust
// Example: src/main.rs:115-122
fn log_format(
    write: &mut dyn Write,
    now: &mut DeferredNow,
    record: &Record<'_>,
) -> Result<(), io::Error> {
    let time = now.format("%Y-%m-%d %H:%M:%S");
    write!(write, "[{time}] {}: {}", record.level(), record.args())
}
```

Output format: `[2024-01-15 14:30:00] INFO: Config watcher started`

---

## Conditional Debug Logging

Debug logs are only included in debug builds using `#[cfg(debug_assertions)]`:

```rust
// Example: src/cpu_common/mod.rs:34-36,79-80
#[cfg(debug_assertions)]
use log::debug;

#[cfg(debug_assertions)]
debug!("cpu infos: {cpu_infos:?}");
```

**Important**: Always use this pattern for debug logs to avoid performance overhead in release builds.

---

## Import Patterns

### Conditional Import

```rust
#[cfg(debug_assertions)]
use log::debug;
use log::{error, warn};
```

### Standard Import

```rust
use log::{debug, error, info};
use log::warn;
```

---

## What to Log

### Log These
- Service lifecycle: startup, shutdown, mode changes
- Configuration events: loaded, reloaded, errors
- Resource state changes: FAS enabled/disabled, buffer created
- Retry attempts and recovery actions
- Performance-critical thresholds reached

### Examples from Codebase

```rust
// Lifecycle events
info!("Config watcher started");
info!("Switch mode: {} -> {}", self.fas_state.mode, new_mode);
info!("New fas buffer on: [{pkg}]");

// Warnings for recoverable issues
warn!("Failed to set fas-rs affinity to cpu1: {e:#}");
warn!("cpcs worker recv disconnected");
warn!("cpcs worker attach failed pid={pid}: {e:#}");

// Debug for diagnostics
debug!("original frametime: {:?}", data.frametime);
debug!("current_fps_long: {current_fps_long:.2}");
debug!("cpu infos: {cpu_infos:?}");
```

---

## What NOT to Log

- **PII**: Process names, user paths (except when useful for debugging in debug builds)
- **Secrets/Keys**: Never log credentials or security-sensitive data
- **High-frequency noise**: Avoid logging in tight loops without rate limiting
- **Repetitive state**: Don't log the same unchanged state repeatedly

---

## Common Mistakes

- **Don't** use `println!` for logging - use `log` macros
- **Don't** include debug logs without `#[cfg(debug_assertions)]` guard
- **Don't** log sensitive information (file paths with usernames, etc.)
- **Do** use format specifiers for readability: `{:#?}` for pretty debug, `{e:#}` for error context
- **Do** include relevant context in log messages: pid, package name, mode
