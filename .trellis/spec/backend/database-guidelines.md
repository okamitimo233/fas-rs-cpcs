# Database Guidelines

> Database patterns and conventions for this project.

---

## Overview

This project does **not** use a traditional database. Configuration is stored in TOML files and managed through the `Config` module with file system watching for hot-reloading. All state is kept in memory during runtime.

---

## Configuration Storage

### File-Based Configuration

Configuration is stored in TOML format:

- **User config**: `/sdcard/Android/fas-rs/games.toml`
- **Standard config**: Provided via CLI argument

### Configuration Loading

```rust
// Example: src/framework/config/mod.rs:45-72
impl Config {
    pub fn new<P>(p: P, sp: P) -> Result<Self>
    where
        P: AsRef<Path>,
    {
        let path = p.as_ref();
        let std_path = sp.as_ref();
        let toml_raw = fs::read_to_string(path)?;
        let toml: ConfigData = toml::from_str(&toml_raw)?;

        let (sx, rx) = mpsc::channel();
        let inner = Inner::new(toml, rx);

        // Spawn watcher thread for hot-reload
        thread::Builder::new()
            .name("ConfigThread".into())
            .spawn(move || {
                wait_and_read(&path, &std_path, &sx).unwrap_or_else(|e| error!("{e:#?}"));
            })?;

        Ok(Self { inner })
    }
}
```

---

## State Management

### In-Memory State

All runtime state is kept in memory using Rust's type system:

```rust
// Example: Thread-safe shared state
pub static EXTRA_POLICY_MAP: OnceLock<HashMap<i32, Mutex<ExtraPolicy>>> = OnceLock::new();
pub static IGNORE_MAP: OnceLock<HashMap<i32, AtomicBool>> = OnceLock::new();
```

### Thread-Safe Patterns

- `Arc<Mutex<T>>` for shared mutable state across threads
- `OnceLock<T>` for one-time initialization
- `AtomicU64` for lock-free counters

```rust
// Example: src/framework/scheduler/looper/mod.rs:79-84
struct CpcsState {
    desired: Arc<Mutex<HashSet<i32>>>,
    generation: Arc<AtomicU64>,
    latest: Arc<Mutex<HashMap<i32, TimedCpcsWeights>>>,
    worker: thread::Thread,
}
```

---

## Serialization

### TOML for Configuration

```rust
// Example: src/framework/config/data/mod.rs
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct ConfigData {
    pub game_list: HashMap<String, Value>,
    pub scene_game_list: HashSet<String>,
    // ...
}
```

### XML for Android Resources

```rust
// Used for parsing Android system files
use quick_xml::DeError;
```

---

## Common Mistakes

- **Don't** assume file operations succeed - always handle `io::Error`
- **Don't** block the main thread on file I/O - use watcher threads
- **Don't** forget to sync state when config reloads
- **Do** use `parking_lot::Mutex` for better performance than `std::sync::Mutex`
- **Do** implement proper cleanup when state becomes stale
- **Do** validate configuration after parsing

---

## Migration Notes

If database support is added in the future:

1. Use SQLite for embedded storage (no external dependencies)
2. Use `rusqlite` crate for Rust bindings
3. Keep migrations in `migrations/` directory
4. Use transactional writes for consistency
5. Maintain backward compatibility with file-based config during transition
