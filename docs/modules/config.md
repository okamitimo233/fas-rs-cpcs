# 配置系统 (Config)

本文档描述 `src/framework/config/` 模块的设计与实现。

## 模块结构

```
config/
├── mod.rs           # Config 入口
├── data/            # 数据结构定义
│   ├── mod.rs       # ConfigData, ModeConfig 等
│   └── default.rs   # 默认值
├── inner.rs         # 内部状态管理
├── merge.rs         # 配置合并逻辑
└── read.rs          # 文件监控与读取
```

## 配置文件结构

### 文件位置

- **用户配置**：`/sdcard/Android/fas-rs/games.toml`
- **标准配置**：由模块打包提供

### TOML 格式

```toml
[config]
keep_std = true
scene_game_list = true

# 游戏列表
# 格式: 包名 = 目标帧率
# 目标帧率可以是: 整数、数组、"auto"
"com.game.example" = 60
"com.game.example2" = [30, 45, 60, 90, 120]
"com.game.example3" = "auto"

# 模式配置
[powersave]
margin_fps = 3.0
core_temp_thresh = 45

[balance]
margin_fps = 2.0
core_temp_thresh = 50

[performance]
margin_fps = 1.0
core_temp_thresh = 55

[fast]
margin_fps = 0.0
core_temp_thresh = "disabled"
```

## 数据结构

### ConfigData

```rust
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ConfigData {
    pub config: Config,
    pub game_list: Table,
    #[serde(skip)]
    pub scene_game_list: HashSet<String>,
    pub powersave: ModeConfig,
    pub balance: ModeConfig,
    pub performance: ModeConfig,
    pub fast: ModeConfig,
}
```

### Config

```rust
#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub struct Config {
    pub keep_std: bool,           // 是否保留标准配置
    pub scene_game_list: bool,    // 是否启用场景游戏列表
}
```

### ModeConfig

```rust
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModeConfig {
    pub margin_fps: MarginFps,           // 边际帧率
    pub core_temp_thresh: TemperatureThreshold, // 温度阈值
}
```

### TargetFps

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TargetFps {
    Value(u32),           // 单一帧率
    Array(Vec<u32>),      // 多个可选帧率
}
```

### MarginFps

```rust
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum MarginFps {
    BaseOnly(MarginFpsValue),           // 基础值
    Advanced {                          // 高级配置
        base: MarginFpsValue,
        overrides: HashMap<String, MarginFpsValue>, // 按帧率覆盖
    },
}

pub enum MarginFpsValue {
    Float(f64),
    Int(u64),
}
```

### TemperatureThreshold

```rust
#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub enum TemperatureThreshold {
    Disabled,     // 禁用温控
    Temp(u64),    // 温度阈值 (摄氏度)
}
```

## Config API

### 创建

```rust
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

        // 启动配置监控线程
        thread::spawn(move || {
            wait_and_read(&path, &std_path, &sx).unwrap();
        });

        Ok(Self { inner })
    }
}
```

### 查询接口

```rust
impl Config {
    // 检查是否需要 FAS
    pub fn need_fas<S>(&mut self, pkg: S) -> bool
    where
        S: AsRef<str>;

    // 获取目标帧率
    pub fn target_fps<S>(&mut self, pkg: S) -> Option<TargetFps>
    where
        S: AsRef<str>;

    // 获取模式配置
    #[must_use]
    pub fn mode_config(&mut self, m: Mode) -> &ModeConfig;
}
```

## 配置合并

### 合并策略

```rust
impl Config {
    pub fn merge(local: &str, std: &str) -> Result<String>;
}
```

合并规则：
1. 用户配置优先
2. 标准配置填充缺失字段
3. `keep_std = true` 时保留标准配置的游戏列表
4. 模式配置合并，用户值覆盖标准值

### 使用场景

```bash
# 命令行合并配置
fas-rs merge /path/to/games.toml
```

用于模块更新时合并新配置与用户配置。

## 热重载

### 监控机制

```rust
fn wait_and_read(path: &Path, std_path: &Path, sx: &Sender<()>) -> Result<()> {
    let inotify = Inotify::init()?;

    inotify.watches().add(
        path.parent().unwrap(),
        WatchMask::CLOSE_WRITE | WatchMask::DELETE_SELF,
    )?;

    loop {
        // 等待文件变更
        inotify.read_events(&mut buffer)?;

        // 重新加载配置
        let toml_raw = fs::read_to_string(path)?;
        let toml: ConfigData = toml::from_str(&toml_raw)?;

        // 通知更新
        sx.send(())?;
    }
}
```

### 更新流程

```
文件变更
    ↓
inotify 检测到事件
    ↓
重新读取配置文件
    ↓
解析 TOML
    ↓
通过 channel 通知 Inner
    ↓
Inner 更新内部状态
```

## 内部状态 (Inner)

```rust
struct Inner {
    config: RwLock<ConfigData>,
    rx: Receiver<()>,
}

impl Inner {
    fn new(config: ConfigData, rx: Receiver<()>) -> Self;

    fn config(&mut self) -> RwLockReadGuard<ConfigData> {
        // 检查是否有更新
        while self.rx.try_recv().is_ok() {}

        self.config.read()
    }
}
```

使用 `RwLock` 实现读写分离，支持并发读取。

## 场景游戏列表

当 `scene_game_list = true` 时，系统会从系统场景识别获取游戏列表：

- 通过解析系统配置识别游戏应用
- 自动为这些应用启用 FAS
- 默认使用 `[30, 45, 60, 90, 120, 144]` 作为目标帧率

## 错误处理

```rust
// 配置解析错误
let toml: ConfigData = toml::from_str(&toml_raw)?;

// 无效帧率配置
if value != "auto" {
    error!("Find target game {pkg} in config, but meet illegal data type");
    error!("Sugg: try '{pkg} = \"auto\"'");
}
```

配置错误会记录日志但不会导致程序崩溃，使用默认值或跳过无效条目。

## 最佳实践

### 目标帧率配置

```toml
# 推荐：明确指定帧率
"com.game.example" = 60

# 多帧率游戏
"com.game.example" = [30, 60, 120]

# 让系统自动检测
"com.game.example" = "auto"
```

### 边际帧率配置

```toml
# 简单配置
margin_fps = 2.0

# 按帧率精细化配置
[powersave.margin_fps]
base = 3.0
"60" = 2.5
"120" = 2.0
```

### 温控配置

```toml
# 禁用温控
core_temp_thresh = "disabled"

# 启用温控
core_temp_thresh = 50
```
