# feat: 自适应帧时间混合算法

## Goal

改进 fas-rs 的 CPU 频率调整算法，将固定比例的帧时间混合策略改为基于波动性的自适应策略，提升不同游戏场景下的调度响应性和稳定性。

## What I already know

### 当前实现

* 位置: `src/framework/scheduler/looper/policy/controll.rs:62-88`
* 当前使用固定混合比例: `SHORT_AVG_BLEND = 0.30`
* 公式: `representative = last_frame * 0.7 + short_avg_frame * 0.3`

### 问题分析

* 稳定场景（音游/格斗 60fps）→ 单帧足够可靠，但固定比例导致响应偏慢
* 波动场景（开放世界游戏）→ 单帧噪声大，但固定比例不够保守
* 需要根据帧时间波动性动态调整混合比例

### 设计方案

* 使用变异系数 (CV) 作为波动性度量
* CV = 标准差 / 均值，无量纲，可跨帧率比较
* 将 CV 映射到 beta ∈ [BETA_MIN, BETA_MAX]

## Assumptions (temporary)

* 方差窗口大小 30 帧是合理的
* CV 阈值 [0.05, 0.30] 适用于大多数游戏场景
* 不需要额外的配置参数暴露给用户

## Open Questions

(none — all resolved)

## Decision (ADR-lite)

**Context**: 需要决定是否将自适应混合参数暴露为可配置项。

**Decision**: 选择方案 2 — 在代码中预留参数结构体，使用默认值，暂不暴露给用户配置，但预留扩展接口。

**Consequences**: 
- 代码结构清晰，方便后续调优
- 不增加用户配置复杂度
- 未来可轻松扩展为可配置项

## Requirements

### 配置层

1. 在 `Config` 结构体中添加 `experimental_scheduler: bool` 字段
2. 默认值为 `false`，需要显式启用
3. 在 `default.rs` 中添加默认值函数

### 核心算法层

4. 定义 `AdaptiveBlendParams` 结构体，包含所有可调参数
5. 在 `FrameTimeState` 中增加 `volatility_cv` 字段存储变异系数
6. 在 `FrameTimeState` 中增加 `volatility_update_counter` 字段
7. 实现变异系数计算函数 `calculate_volatility()`，使用最近 30 帧数据
8. 实现 `cv_to_blend_beta()` 映射函数，使用参数结构体
9. 修改 `get_normalized_last_frame()`：根据配置开关选择算法
10. 方差计算频率优化：每 10 帧更新一次

### 测试工具

11. 创建 `debug/` 目录存放调试工具
12. 实现 `debug/algorithm_bench.rs` 对抗性测试程序
13. 独立最小化实现新旧算法
14. 模拟多种游戏负载场景
15. 生成对比评估表格（Markdown 格式）

## Acceptance Criteria

- [ ] 配置项 `experimental_scheduler` 默认为 `false`
- [ ] 配置关闭时使用原有固定比例混合算法
- [ ] 配置开启时使用自适应混合算法
- [ ] 稳定场景 (CV < 0.05) 使用 beta = 0.10，快速响应
- [ ] 高波动场景 (CV > 0.30) 使用 beta = 0.60，平滑稳定
- [ ] 中间场景线性插值
- [ ] 不影响现有 API 接口
- [ ] 编译通过，无 lint 警告
- [ ] 测试工具可独立运行，生成评估表格

## Definition of Done

* 单元测试覆盖变异系数计算逻辑
* 编译通过 (cargo build --release)
* Lint 通过 (cargo clippy)
* 在实际设备上测试不同游戏场景

## Out of Scope

* PID 控制器改进（独立任务）
* 非对称频率变化限制（独立任务）
* 温控策略改进（独立任务）
* 负载预测（高阶改进，暂不实现）
* 用户可配置 CV 阈值和 Beta 范围参数（仅提供开关）

## Technical Notes

### 关键文件

* `src/framework/config/data/mod.rs` — 配置结构体定义
* `src/framework/config/data/default.rs` — 默认值定义
* `src/framework/scheduler/looper/policy/controll.rs` — 控制逻辑
* `src/framework/scheduler/looper/buffer/mod.rs` — Buffer 数据结构
* `src/framework/scheduler/looper/buffer/calculate.rs` — 计算函数
* `debug/algorithm_bench.rs` — 对抗性测试工具（新建）

### 测试工具设计

#### 模拟场景

| 场景 | 帧率 | 帧时间特征 | CV 范围 |
|------|------|------------|---------|
| 音游稳定 | 60fps | 低抖动 ±1ms | 0.02-0.04 |
| 格斗游戏 | 60fps | 中低抖动 ±2ms | 0.03-0.06 |
| 开放世界 | 60fps | 中等抖动 ±4ms | 0.08-0.15 |
| 场景切换 | 60fps | 高抖动 ±10ms+ | 0.20-0.40 |
| 加载卡顿 | 60fps | 极高抖动 ±20ms | 0.40+ |

#### 测试指标

* **响应速度**: 帧时间变化后算法调整 beta 的速度
* **稳定性**: 连续帧时间序列下 beta 的方差
* **边界行为**: 极端 CV 值下的 beta 表现

#### 输出格式

```markdown
## 算法对比评估

| 场景 | CV | 旧算法 beta | 新算法 beta | 响应改进 |
|------|-----|-------------|-------------|----------|
| 音游稳定 | 0.03 | 0.30 | 0.10 | ✅ 更快响应 |
| ... | ... | ... | ... | ... |
```

### 配置示例 (games.toml)

```toml
[config]
keep_std = true
scene_game_list = true
experimental_scheduler = true  # 启用实验性调度算法，默认 false
```

### 参数结构体设计

```rust
#[derive(Debug, Clone, Copy)]
pub struct AdaptiveBlendParams {
    pub beta_min: f64,           // 默认 0.10
    pub beta_max: f64,           // 默认 0.60
    pub cv_threshold_low: f64,   // 默认 0.05
    pub cv_threshold_high: f64,  // 默认 0.30
    pub variance_window: usize,  // 默认 30
    pub update_interval: u8,     // 默认 10
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

### 关键参数

| 参数 | 默认值 | 说明 |
|------|--------|------|
| `BETA_MIN` | 0.10 | 最小混合（信任单帧）|
| `BETA_MAX` | 0.60 | 最大混合（信任平均）|
| `CV_THRESHOLD_LOW` | 0.05 | 低于此值 → beta = MIN |
| `CV_THRESHOLD_HIGH` | 0.30 | 高于此值 → beta = MAX |
| 方差窗口 | 30 帧 | 计算方差的数据量 |
| 更新频率 | 10 帧 | 方差更新间隔 |

### 预期收益

| 场景 | CV 范围 | Beta | 效果 |
|------|---------|------|------|
| 音游/格斗 | 0.02-0.04 | 0.10 | 快速响应 |
| 开放世界 | 0.10-0.20 | 0.25-0.40 | 平衡 |
| 场景切换 | 0.30+ | 0.60 | 平滑稳定 |
