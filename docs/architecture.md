# 系统架构

本文档描述 fas-rs/cpcs 系统的整体架构、模块划分和核心数据流。

## 架构概览

fas-rs/cpcs 采用分层架构设计，从底层 eBPF 监控到上层策略控制，各层职责清晰。

```
┌─────────────────────────────────────────────────────────────┐
│                      应用入口 (main.rs)                      │
├─────────────────────────────────────────────────────────────┤
│                      调度器 (Scheduler)                      │
│  ┌─────────────┬─────────────┬─────────────┬─────────────┐  │
│  │   Looper    │   Thermal   │   TopApp    │    Node     │  │
│  │  (主循环)   │  (温控)     │ (应用监控)  │  (模式节点)  │  │
│  └─────────────┴─────────────┴─────────────┴─────────────┘  │
├─────────────────────────────────────────────────────────────┤
│                      核心框架 (Framework)                     │
│  ┌─────────────┬─────────────┬─────────────┬─────────────┐  │
│  │   Config    │  Extension  │    Node     │  pid_utils  │  │
│  │  (配置)     │  (插件)     │  (模式)     │  (进程工具)  │  │
│  └─────────────┴─────────────┴─────────────┴─────────────┘  │
├─────────────────────────────────────────────────────────────┤
│                    CPU 控制器 (Controller)                   │
│            频率控制、策略约束、使用率监控                      │
├─────────────────────────────────────────────────────────────┤
│                     帧分析器 (Analyzer)                      │
│  ┌────────────────────────┬────────────────────────────┐    │
│  │   Frame Analyzer       │      CPCS Analyzer        │    │
│  │   (帧时间检测)          │    (DAG 关键路径分析)      │    │
│  └────────────────────────┴────────────────────────────┘    │
├─────────────────────────────────────────────────────────────┤
│                       eBPF 层                                │
│      Uprobe (帧检测) + Tracepoint (调度/Futex)               │
└─────────────────────────────────────────────────────────────┘
```

## 模块职责

### 入口层 (main.rs)

- 解析命令行参数
- 初始化日志系统
- 创建 Controller 和 Config
- 构建 Scheduler 并启动运行

### 调度器层 (Scheduler)

**Scheduler** 是系统的核心调度入口，使用 Builder 模式构建：

```rust
Scheduler::new()
    .config(config)
    .controller(cpu)
    .start_run()?;
```

**Looper** 实现主事件循环，包含：

- **Buffer** - 帧时间缓冲区，存储历史帧数据
- **FasState** - FAS 状态机管理 (NotWorking/Waiting/Working)
- **Thermal** - 温度监控与 FPS 偏移调整
- **TopAppsWatcher** - 前台应用监控

### 核心框架层 (Framework)

| 模块 | 职责 |
|------|------|
| Config | 配置管理、热重载、配置合并 |
| Extension | 插件加载与执行 |
| Node | 模式节点控制 (powersave/balance/performance/fast) |
| pid_utils | 进程名获取等工具函数 |

### CPU 控制器 (Controller)

Controller 负责 CPU 频率控制：

- **Info** - CPU 信息 (policy、频率范围)
- **ExtraPolicy** - 额外策略约束 (绝对/相对频率限制)
- **CpuUsageMonitor** - CPU 使用率监控
- **fas_update_freq_weighted** - 基于 CPCS 权重的频率分配

### 帧分析器

系统使用两个互补的分析器：

**Frame Analyzer** (third_party/frame-analyzer-patched)
- 通过 Uprobe 监测 `Surface::queueBuffer`
- 输出帧时间 (frametime)

**CPCS Analyzer** (cpcs-analyzer)
- 基于 DAG 的关键路径分析
- 通过 Tracepoint 监测调度事件和 Futex
- 输出各 policy 的权重建议

## 核心数据流

### 1. 帧检测流程

```
应用调用 Surface::queueBuffer
        ↓
eBPF Uprobe 捕获事件
        ↓
Frame Analyzer 解析帧时间
        ↓
Buffer 存储帧时间数据
        ↓
calculate_control 计算控制比例
        ↓
Controller 调整 CPU 频率
```

### 2. CPCS 分析流程

```
eBPF Tracepoint 捕获调度/Futex 事件
        ↓
构建帧 DAG (线程执行时间、边关系)
        ↓
推断关键子图 (Critical Subgraph)
        ↓
计算各 policy 权重
        ↓
Controller 按权重分配频率预算
```

### 3. 控制策略流程

```
获取目标帧率 (target_fps)
        ↓
计算边际帧率 (margin_fps)
        ↓
获取当前帧时间 → 计算误差比例
        ↓
EMA 平滑处理
        ↓
输出 control_ratio
        ↓
计算总频率预算 (total_budget_khz)
        ↓
按 CPCS 权重分配给各 policy
        ↓
应用额外策略约束
        ↓
写入 CPU 频率
```

## FAS 原理

### 帧感知调度核心思想

传统 CPU 调度器基于负载调整频率，但游戏等实时应用的关键指标是**帧时间**而非 CPU 使用率。

FAS 的核心思路：

1. **监测帧时间**：通过 eBPF 非侵入式地获取帧渲染时间
2. **计算控制量**：根据帧时间与目标帧时间的偏差计算频率调整比例
3. **智能分配**：通过 CPCS 分析器识别关键线程，优先为其分配 CPU 资源

### 控制算法

```rust
// 控制算法核心逻辑
let error_ratio = current_frametime / target_frametime - 1.0;

// 裁剪误差，防止单帧异常污染 EMA 状态
let clipped_error = error_ratio.clamp(-error_clip_ratio, error_clip_ratio);

// EMA 平滑滤波
let smooth_error = alpha * prev_error + (1.0 - alpha) * clipped_error;

// 输出控制比例
let control_ratio = smooth_error * kp;
```

关键参数：
- `kp = 0.4` - 比例增益
- `error_ema_alpha = 0.5` - EMA 平滑因子
- `error_clip_ratio = 0.8` - 误差裁剪比例

控制比例范围：`[-0.8, 1.0]`，最终目标频率 = 当前频率 * (1 + control_ratio)

### 状态机

FAS 工作状态由状态机管理：

```
NotWorking ──(检测到目标游戏)──> Waiting ──(延迟 3 秒)──> Working
     ↑                                                        │
     └────────────(游戏退出或窗口切换)────────────────────────┘
```

- **NotWorking**：未进行 FAS 调度
- **Waiting**：检测到目标游戏，等待稳定，触发 `start_fas` 回调
- **Working**：正在执行 FAS 调度，触发 `init_cpu_freq` 回调

状态转换时触发的回调：
- `NotWorking -> Waiting`：触发 `start_fas`
- `Waiting -> Working`：触发 `init_cpu_freq`
- `Working -> NotWorking`：触发 `stop_fas` 和 `reset_cpu_freq`

## 多线程模型

系统使用多线程提高并发性：

| 线程名 | 职责 |
|--------|------|
| main | 主事件循环 (Looper) |
| ConfigThread | 配置文件监控与热重载 |
| ExtensionThread | 插件执行 |
| cpcs | CPCS 分析器工作线程 |

## 进程间通信

- **mpsc::channel** - Config 变更通知
- **mpsc::sync_channel** - Extension API 触发
- **Arc<Mutex>** - CPCS 权重共享
- **AtomicU64** - CPCS generation 版本号

## 文件系统交互

| 路径 | 用途 |
|------|------|
| `/sdcard/Android/fas-rs/games.toml` | 用户配置文件 |
| `/dev/fas_rs/` | 模式节点目录 |
| `/dev/fas_rs/extensions/` | 插件目录 |
| `/sys/devices/system/cpu/cpufreq/` | CPU 频率控制 |
| `/sys/devices/virtual/thermal/` | 温度传感器 |
