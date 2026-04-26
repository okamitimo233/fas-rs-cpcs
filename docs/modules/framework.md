# 核心框架 (Framework)

本文档描述 `src/framework/` 模块的设计与实现。

## 模块概述

Framework 是 fas-rs 的核心框架层，提供配置管理、插件系统、模式控制和错误处理等基础设施。

```
framework/
├── mod.rs           # 模块入口，导出公共 API
├── prelude.rs       # 常用导入
├── error.rs         # 错误类型定义
├── config/          # 配置管理
├── extension/       # 插件系统
├── scheduler/       # 调度器
├── node/            # 模式节点
└── pid_utils.rs     # 进程工具函数
```

## 模块组织

### 公共导出 (mod.rs)

```rust
pub use config::Config;
pub use error::Result;
pub use extension::{Api, Extension, api};
pub use node::Mode;
pub use scheduler::Scheduler;
```

框架采用模块化设计，每个子模块负责单一职责，通过 `mod.rs` 统一导出公共 API。

### 错误处理 (error.rs)

使用 `thiserror` 定义统一的错误类型：

```rust
#[derive(ThisError, Debug)]
pub enum Error {
    #[error(transparent)]
    Anyhow(#[from] anyhow::Error),
    #[error("Got an error when parsing config")]
    ParseConfig,
    #[error("Got an error when parsing node")]
    ParseNode,
    #[error("No such a node")]
    NodeNotFound,
    #[error("Missing {0} when building Scheduler")]
    SchedulerMissing(&'static str),
    // ...
}

pub type Result<T> = std::result::Result<T, Error>;
```

错误处理原则：
- 库代码使用 `Error` 枚举和 `Result<T>` 类型别名
- 应用边界使用 `anyhow::Result` 和 `Context` trait
- 使用 `#[error(transparent)]` 包装外部错误

### Prelude (prelude.rs)

提供常用导入的便捷汇总：

```rust
pub use crate::framework::{Config, Extension, Mode, Scheduler};
pub use crate::framework::config::TargetFps;
pub use crate::framework::error::Result;
```

使用方式：

```rust
use crate::framework::prelude::*;
```

## 关键类型

### Config

配置管理器，负责：
- 加载和解析 TOML 配置
- 监控配置文件变更 (热重载)
- 合并用户配置与标准配置

```rust
impl Config {
    pub fn new<P>(p: P, sp: P) -> Result<Self>;
    pub fn need_fas<S>(&mut self, pkg: S) -> bool;
    pub fn target_fps<S>(&mut self, pkg: S) -> Option<TargetFps>;
    pub fn mode_config(&mut self, m: Mode) -> &ModeConfig;
    pub fn merge(local: &str, std: &str) -> Result<String>;
}
```

详见 [配置系统](config.md)。

### Extension

插件系统入口，负责：
- 加载 Lua 插件
- 触发 API 回调
- 提供辅助函数

```rust
impl Extension {
    pub fn init() -> Result<Self>;
    pub fn trigger_extentions(&self, trigger: impl Api + 'static);
}
```

详见 [插件系统](extension.md)。

### Mode

调度模式枚举：

```rust
pub enum Mode {
    Powersave,    // 省电模式
    Balance,      // 均衡模式
    Performance,  // 性能模式
    Fast,         // 快速模式
}
```

不同模式有不同的 `margin_fps` 和 `core_temp_thresh` 配置。

### Scheduler

调度器入口，使用 Builder 模式构建：

```rust
impl Scheduler {
    pub const fn new() -> Self;
    pub fn config(mut self, c: Config) -> Self;
    pub fn controller(mut self, c: Controller) -> Self;
    pub fn start_run(self) -> Result<()>;
}
```

详见 [调度器](scheduler.md)。

## 设计原则

### 模块化

每个子模块独立，通过 `mod.rs` 导出公共 API，隐藏内部实现细节。

### 类型安全

- 使用 `Result<T>` 处理错误
- 使用 `Option<T>` 表示可能缺失的值
- 使用 Builder 模式构建复杂对象

### 错误传播

```rust
// 使用 ? 操作符传播错误
let config = self.config.ok_or(Error::SchedulerMissing("Config"))?;
```

### 条件编译

调试代码使用 `#[cfg(debug_assertions)]` 守卫：

```rust
#[cfg(debug_assertions)]
use log::debug;

#[cfg(debug_assertions)]
debug!("Diagnostic info: {:?}", data);
```

## 依赖关系

```
main.rs
    └── Scheduler
            ├── Config (config/)
            ├── Controller (cpu_common/)
            ├── Extension (extension/)
            ├── Node (node/)
            └── Looper (scheduler/looper/)
                    ├── Buffer (looper/buffer/)
                    ├── Thermal (scheduler/thermal.rs)
                    └── TopAppsWatcher (scheduler/topapp.rs)
```

框架各模块通过清晰的接口协作，避免循环依赖。
