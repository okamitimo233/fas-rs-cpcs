# 调度器 (Scheduler)

本文档描述 `src/framework/scheduler/` 模块的设计与实现。

## 模块结构

```
scheduler/
├── mod.rs           # Scheduler 入口
├── looper/          # 主循环实现
│   ├── mod.rs       # Looper 主逻辑
│   ├── buffer/      # 帧缓冲区
│   ├── clean.rs     # 清理逻辑
│   └── policy/      # 控制策略
├── thermal.rs       # 温度管理
└── topapp.rs        # 前台应用监控
```

## Scheduler 入口

### Builder 模式

Scheduler 使用 Builder 模式构建，允许灵活配置依赖：

```rust
pub struct Scheduler {
    controller: Option<Controller>,
    config: Option<Config>,
}

impl Scheduler {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            controller: None,
            config: None,
        }
    }

    pub fn config(mut self, c: Config) -> Self {
        self.config = Some(c);
        self
    }

    pub fn controller(mut self, c: Controller) -> Self {
        self.controller = Some(c);
        self
    }

    pub fn start_run(self) -> Result<()> {
        let extension = Extension::init()?;
        let config = self.config.ok_or(Error::SchedulerMissing("Config"))?;
        let controller = self.controller.ok_or(Error::SchedulerMissing("Controller"))?;
        let node = Node::init()?;
        let analyzer = Analyzer::new()?;
        let cpcs_analyzer = CpcsAnalyzer::new()?;

        Looper::new(analyzer, cpcs_analyzer, config, node, extension, controller)
            .enter_loop()
    }
}
```

### 启动流程

```
Scheduler::new()
     ↓
.config(config)
     ↓
.controller(cpu)
     ↓
.start_run()
     ├── Extension::init()
     ├── Node::init()
     ├── Analyzer::new()
     ├── CpcsAnalyzer::new()
     └── Looper::new().enter_loop()
```

## Looper 主循环

### 核心数据结构

```rust
struct FasState {
    mode: Mode,                    // 当前模式
    working_state: State,          // 工作状态
    delay_timer: Instant,          // 延迟计时器
    buffer: Option<Buffer>,        // 帧缓冲区
}

struct AnalyzerState {
    analyzer: Analyzer,            // 帧分析器
    restart_counter: u8,           // 重启计数
    restart_timer: Instant,        // 重启计时
}

struct CpcsState {
    desired: Arc<Mutex<HashSet<i32>>>,  // 目标 PID 集合
    generation: Arc<AtomicU64>,          // 版本号
    latest: Arc<Mutex<HashMap<i32, TimedCpcsWeights>>>,  // 最新权重
    worker: thread::Thread,              // 工作线程句柄
}

struct ControllerState {
    controller: Controller,        // CPU 控制器
    params: ControllerParams,      // 控制参数
    error_ratio_ema: Option<f64>,  // EMA 状态
}
```

### 状态机

FAS 工作状态由状态机管理：

```rust
#[derive(PartialEq, Debug)]
enum State {
    NotWorking,  // 未工作
    Waiting,     // 等待中
    Working,     // 工作中
}
```

状态转换：

```
NotWorking ──(enable_fas)──> Waiting ──(3 秒延迟)──> Working
     ↑                                                        │
     └──────────────────────(disable_fas)─────────────────────┘
```

状态转换详情：

- **NotWorking -> Waiting**：检测到目标游戏，触发 `start_fas` 回调
- **Waiting -> Working**：延迟 3 秒后进入工作状态，触发 `init_cpu_freq` 回调
- **Working -> NotWorking**：游戏退出或窗口切换，触发 `stop_fas` 和 `reset_cpu_freq` 回调
- **Waiting -> NotWorking**：游戏退出，不触发回调

### 主循环逻辑

```rust
pub fn enter_loop(&mut self) -> Result<()> {
    loop {
        self.switch_mode();        // 检查模式切换
        self.update_analyzer();    // 更新分析器附加
        self.retain_topapp();      // 清理非前台进程

        if self.windows_watcher.visible_freeform_window() {
            self.disable_fas();    // 自由窗口时禁用 FAS
        }

        if let Some(data) = self.recv_message() {
            // 收到帧数据
            if let Some(state) = self.buffer_update(&data) {
                match state {
                    BufferWorkingState::Usable => self.do_policy(),
                    BufferWorkingState::Unusable => self.disable_fas(),
                }
            }
        } else if let Some(buffer) = self.fas_state.buffer.as_mut() {
            // 帧丢失 (jank)
            buffer.additional_frametime(&self.extension);
            match buffer.state.working_state {
                BufferWorkingState::Unusable => {
                    self.restart_analyzer();
                    self.disable_fas();
                }
                BufferWorkingState::Usable => self.do_policy(),
            }
        }
    }
}
```

## Buffer 帧缓冲区

### 数据结构

```rust
pub struct Buffer {
    pub package_info: PackageInfo,      // 包信息
    pub frametime_state: FrameTimeState, // 帧时间状态
    pub target_fps_state: TargetFpsState, // 目标帧率状态
    pub state: BufferState,              // 缓冲区状态
}

pub struct FrameTimeState {
    pub current_fps_long: f64,      // 长期 FPS
    pub avg_time_long: Duration,    // 长期平均帧时间
    pub current_fps_short: f64,     // 短期 FPS
    pub avg_time_short: Duration,   // 短期平均帧时间
    pub frametimes: VecDeque<Duration>, // 帧时间队列
    pub additional_frametime: Duration, // 额外帧时间 (jank)
}
```

### 工作状态

```rust
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum BufferWorkingState {
    Unusable,  // 不可用 (预热中)
    Usable,    // 可用
}
```

缓冲区需要收集足够帧数据后才变为可用状态 (约 1 秒)。

### 目标帧率推断

Buffer 通过帧时间分布推断目标帧率：

```rust
fn calculate_target_fps(&mut self, extension: &Extension) {
    // 分析帧时间分布，匹配 30/45/60/90/120/144 FPS
    // 触发 TargetFpsChange 回调
}
```

## 控制策略

### 控制参数

```rust
pub struct ControllerParams {
    pub kp: f64,              // 比例增益 (默认 0.4)
    pub error_ema_alpha: f64, // EMA 平滑因子 (默认 0.5)
    pub error_clip_ratio: f64, // 误差裁剪比例 (默认 0.8)
}
```

### 控制计算

```rust
pub fn calculate_control(
    buffer: &Buffer,
    config: &mut Config,
    mode: Mode,
    controller_state: &mut ControllerState,
    target_fps_offset_thermal: f64,
) -> Option<ControlOutput>
```

控制算法流程：

1. **获取目标帧率**：从配置或自动推断
2. **计算边际帧率**：`margin_fps = target_fps / 60 * base_margin`
3. **调整目标帧率**：考虑温控偏移
4. **计算误差比例**：`error_ratio = current_frametime / target_frametime - 1.0`
5. **裁剪误差**：`clipped_error = error_ratio.clamp(-0.8, 0.8)`
6. **EMA 平滑**：`smooth_error = alpha * prev + (1-alpha) * clipped_error`
7. **输出控制比例**：`control_ratio = smooth_error * kp`

```rust
pub struct ControlOutput {
    pub control_ratio: f64,  // 控制比例 [-0.8, 1.0]
    pub is_janked: bool,     // 是否卡顿
}
```

## Thermal 温度管理

### 功能

- 监控 CPU 温度
- 超过阈值时降低目标帧率
- 实现温度保护

```rust
impl Thermal {
    pub fn new() -> Result<Self>;
    pub fn target_fps_offset(&mut self, config: &mut Config, mode: Mode) -> f64;
}
```

### 温度节点

自动检测以下温度传感器：
- `cpu-*`
- `soc_max`
- `mtktscpu`

### 温控策略

```rust
if self.core_temperature > target_core_temperature {
    self.target_fps_offset -= 0.1;  // 降低目标
} else {
    self.target_fps_offset += 0.1;  // 恢复目标
}
```

## TopAppsWatcher 前台应用监控

### 功能

- 监控前台应用 PID
- 检测自由窗口状态
- 过滤非目标应用

```rust
impl TopAppsWatcher {
    pub fn new() -> Self;
    pub fn topapp_pids(&self) -> &Vec<i32>;
    pub fn visible_freeform_window(&self) -> bool;
}
```

## CPCS 工作线程

CPCS 分析器在独立线程中运行：

```rust
fn cpcs_worker_loop(
    mut analyzer: CpcsAnalyzer,
    desired: Arc<Mutex<HashSet<i32>>>,
    generation: Arc<AtomicU64>,
    latest: Arc<Mutex<HashMap<i32, TimedCpcsWeights>>>,
) {
    loop {
        // 检查目标变更
        if latest_generation != applied_generation {
            reconcile_cpcs_targets(...);
        }

        // 接收权重
        match analyzer.recv_timeout(CPCS_RECV_TIMEOUT) {
            Ok((pid, weights)) => {
                // 存储最新权重
            }
            // ...
        }
    }
}
```

主线程和工作线程通过共享内存通信：
- `desired` - 目标 PID 集合
- `generation` - 版本号 (用于触发同步)
- `latest` - 最新权重数据

## 关键常量

```rust
const DELAY_TIME: Duration = Duration::from_secs(3);       // 进入工作状态延迟
const CPCS_RECV_TIMEOUT: Duration = Duration::from_millis(150); // CPCS 接收超时
const CPCS_STALE_TIMEOUT: Duration = Duration::from_secs(2);    // CPCS 数据过期时间
```
