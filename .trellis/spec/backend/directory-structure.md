# Directory Structure

> How backend code is organized in this project.

---

## Overview

This is a Rust workspace project for Android frame-aware scheduling (FAS) with eBPF support. The codebase follows a modular architecture with clear separation between core framework, CPU control, and eBPF analysis.

---

## Directory Layout

```
fas-rs/
├── src/                          # Main application source
│   ├── main.rs                   # Entry point, CLI handling
│   ├── cpu_common/               # CPU frequency control logic
│   │   ├── mod.rs                # Controller implementation
│   │   ├── cpu_info.rs           # CPU info parsing
│   │   ├── cpu_usage_monitor.rs  # CPU usage tracking
│   │   └── extra_policy.rs       # Additional policy types
│   ├── framework/                # Core framework modules
│   │   ├── mod.rs                # Framework exports
│   │   ├── error.rs              # Error types
│   │   ├── prelude.rs            # Common imports
│   │   ├── config/               # Configuration handling
│   │   ├── extension/            # Extension API (Lua bindings)
│   │   ├── node/                 # Node/mode management
│   │   ├── pid_utils.rs          # PID utilities
│   │   └── scheduler/            # Main scheduling logic
│   │       ├── mod.rs            # Scheduler builder
│   │       ├── thermal.rs        # Thermal management
│   │       ├── topapp.rs         # Top app watching
│   │       └── looper/           # Main event loop
│   │           ├── mod.rs        # Looper implementation
│   │           ├── clean.rs      # Cleanup logic
│   │           ├── buffer/       # Frame buffer analysis
│   │           └── policy/       # Control policy
│   ├── file_handler.rs           # File I/O utilities
│   └── misc.rs                   # Miscellaneous utilities
├── cpcs-analyzer/                # CPCS analyzer crate
├── cpcs-analyzer-common/         # Shared analyzer code
├── cpcs-analyzer-ebpf/           # eBPF kernel programs
├── third_party/                  # Vendored third-party libs
│   └── frame-analyzer-patched/   # Patched frame analyzer
├── xtask/                        # Build automation tasks
│   └── src/main.rs               # xtask CLI
├── module/                       # Kernel module files
├── webui/                        # Web UI assets
└── update/                       # Update scripts
```

---

## Module Organization

### Core Pattern
- **One module = one directory** with `mod.rs` as entry point
- Sub-modules are declared in `mod.rs` and live in sibling files or subdirectories
- Public exports are re-exported from `mod.rs` using `pub use`

### Naming Conventions
- **Module names**: `snake_case` (e.g., `cpu_common`, `pid_utils`)
- **File names**: `snake_case.rs` matching module name
- **Structs/Enums**: `PascalCase` (e.g., `Controller`, `FasData`, `TargetFps`)
- **Functions/methods**: `snake_case` (e.g., `fas_update_freq_weighted`)

### Import Organization
```rust
// 1. Standard library imports
use std::{collections::HashMap, fs, path::Path};

// 2. External crate imports
use anyhow::Result;
use log::{info, warn};

// 3. Internal crate imports
use crate::framework::{Config, Extension};
```

---

## Key Architectural Patterns

### Builder Pattern
Used for constructing complex objects with optional dependencies:

```rust
// Example: src/framework/scheduler/mod.rs:47-68
pub struct Scheduler {
    controller: Option<Controller>,
    config: Option<Config>,
}

impl Scheduler {
    pub const fn new() -> Self { ... }
    pub fn config(mut self, c: Config) -> Self { ... }
    pub fn controller(mut self, c: Controller) -> Self { ... }
    pub fn start_run(self) -> Result<()> { ... }
}
```

### State Machine Pattern
Used for managing FAS working states:

```rust
// Example: src/framework/scheduler/looper/mod.rs:59-64
#[derive(PartialEq, Debug)]
enum State {
    NotWorking,
    Waiting,
    Working,
}
```

---

## Common Mistakes

- **Don't** put business logic in `main.rs` - delegate to framework modules
- **Don't** create circular module dependencies
- **Do** use `mod.rs` to re-export public API, not expose internal structure
- **Do** keep sub-modules focused on single responsibility
