# Error Handling

> How errors are handled in this project.

---

## Overview

This project uses a combination of `thiserror` for library errors and `anyhow` for application errors. Custom error types provide context-specific messages while leveraging `#[error(transparent)]` for automatic error propagation.

---

## Error Types

### Primary Error Enum

Defined in `src/framework/error.rs`:

```rust
use thiserror::Error as ThisError;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(ThisError, Debug)]
pub enum Error {
    #[error(transparent)]
    Anyhow(#[from] anyhow::Error),
    
    #[error(transparent)]
    FrameAnalyzer(#[from] AnalyzerError),
    
    #[error("Got an error when parsing config")]
    ParseConfig,
    
    #[error("Got an error when parsing node")]
    ParseNode,
    
    #[error("No such a node")]
    NodeNotFound,
    
    #[error(transparent)]
    SerToml(#[from] toml::ser::Error),
    
    #[error(transparent)]
    DeToml(#[from] toml::de::Error),
    
    #[error(transparent)]
    SerXml(#[from] quick_xml::DeError),
    
    #[error("Missing {0} when building Scheduler")]
    SchedulerMissing(&'static str),
    
    #[error(transparent)]
    Io(#[from] io::Error),
    
    #[error(transparent)]
    Lua {
        #[from]
        source: mlua::Error,
    },
    
    #[error("Got an error: {0}")]
    Other(&'static str),
}
```

---

## Error Handling Patterns

### Using Result Type Alias

Always use the `Result<T>` type alias defined in the framework:

```rust
// Example: src/framework/scheduler/mod.rs:70-82
pub fn start_run(self) -> Result<()> {
    let extension = Extension::init()?;
    let config = self.config.ok_or(Error::SchedulerMissing("Config"))?;
    let controller = self.controller.ok_or(Error::SchedulerMissing("Controller"))?;
    // ...
}
```

### Application Entry Point

Use `anyhow::Result` at the application boundary:

```rust
// Example: src/main.rs:39,54
use anyhow::Result;

fn main() -> Result<()> {
    // ...
}
```

### Error Chaining for Debugging

Log error chains at the application level:

```rust
// Example: src/main.rs:67-72
run(&args[2]).unwrap_or_else(|e| {
    for cause in e.chain() {
        error!("{cause:#?}");
    }
    error!("{:#?}", e.backtrace());
});
```

---

## Error Propagation

### Use `?` Operator

Propagate errors with context when needed:

```rust
// Simple propagation
let config = Config::new(USER_CONFIG, std_path)?;

// With context (using anyhow)
let data = fs::read_to_string(&path)
    .with_context(|| format!("Failed to read {}", path.display()))?;
```

### Custom Errors for Missing Values

Use `ok_or` for Option types:

```rust
// Example: src/framework/scheduler/mod.rs:73-76
let config = self.config.ok_or(Error::SchedulerMissing("Config"))?;
```

---

## Common Mistakes

- **Don't** unwrap() in library code - propagate errors instead
- **Don't** use `expect()` except in test code or truly impossible failures
- **Don't** catch and silently ignore errors - at minimum log them
- **Do** use `#[error(transparent)]` to wrap external errors without adding noise
- **Do** provide descriptive error messages for custom variants
- **Do** use `anyhow::Context` for adding context at application boundaries
