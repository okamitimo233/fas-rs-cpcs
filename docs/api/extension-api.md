# 插件 API 文档

本文档详细描述 fas-rs 插件 API 的版本演进、回调函数和辅助函数。

## API 版本演进

| 版本 | 新增回调 | 新增辅助函数 | 说明 |
|------|----------|--------------|------|
| v0 | 基础回调 | - | 初始版本 |
| v1 | - | set_policy_freq_offset | 添加频率偏移控制 (v4.2.0 已移除) |
| v2 | target_fps_change | - | 添加帧率变更通知 |
| v3 | - | set_ignore_policy | 添加策略忽略控制 |
| v4 | - | set_extra_policy_abs, set_extra_policy_rel, remove_extra_policy | 重构频率约束系统 |

### 版本兼容性

插件通过 `api_version` 变量声明使用的 API 版本：

```lua
api_version = 4
```

系统根据版本号选择对应的回调函数实现。**注意：辅助函数对所有 API 版本可用**，版本号仅影响回调函数的触发。例如，`set_ignore_policy` 在 API v3 时添加，但 v0/v1/v2 插件也可以调用它。

## 回调函数

所有回调函数都是可选的，插件只需实现需要的回调。

### load_fas

当目标游戏被检测到时触发。

**参数：**
- `pid` (number): 进程 ID
- `pkg` (string): 包名

**示例：**
```lua
function load_fas(pid, pkg)
    log_info("检测到游戏: " .. pkg .. " (PID: " .. pid .. ")")
end
```

### unload_fas

当目标游戏退出前台时触发。

**参数：**
- `pid` (number): 进程 ID
- `pkg` (string): 包名

**示例：**
```lua
function unload_fas(pid, pkg)
    log_info("游戏退出: " .. pkg)
end
```

### start_fas

当 FAS 开始工作时触发 (进入 Waiting 状态时)。

**参数：** 无

**说明：** 在检测到目标游戏后立即触发，此时 FAS 进入 Waiting 状态，等待 3 秒延迟后才会进入 Working 状态并开始调度。如需在 Working 状态初始化时执行代码，应使用 `init_cpu_freq` 回调。

**示例：**
```lua
function start_fas()
    log_info("FAS 开始工作")
    -- 注意：此时尚未进入 Working 状态
    -- 如需初始化 CPU 频率约束，请使用 init_cpu_freq 回调
end
```

### stop_fas

当 FAS 停止工作时触发 (从 Working 状态退出时)。

**参数：** 无

**说明：** 仅在从 Working 状态退出时触发。如果 FAS 处于 Waiting 状态时游戏退出，不会触发此回调。如需清理 CPU 频率约束，应使用 `reset_cpu_freq` 回调。

**示例：**
```lua
function stop_fas()
    log_info("FAS 停止工作")
    -- 注意：如需清理 CPU 频率约束，请使用 reset_cpu_freq 回调
end
```

### init_cpu_freq

当进入游戏模式时触发 (Working 状态初始化)。

**参数：** 无

**说明：** 用于设置游戏模式下的 CPU 频率约束。

**示例：**
```lua
function init_cpu_freq()
    -- 限制 policy4 最高 2.0 GHz
    set_extra_policy_abs(4, nil, 2000000)
    -- policy6 跟随 policy4
    set_extra_policy_rel(6, 4, 0, 500000)
end
```

### reset_cpu_freq

当退出游戏模式时触发。

**参数：** 无

**说明：** 用于清理自定义的 CPU 频率约束。

**示例：**
```lua
function reset_cpu_freq()
    remove_extra_policy(4)
    remove_extra_policy(6)
end
```

### target_fps_change

当检测到目标帧率变更时触发。

**参数：**
- `target_fps` (number): 新的目标帧率
- `pkg` (string): 包名

**API 版本：** v2+

**示例：**
```lua
function target_fps_change(target_fps, pkg)
    log_info(pkg .. " 目标帧率变更为: " .. target_fps)
end
```

## 辅助函数

### log_info

输出 INFO 级别日志。

**参数：**
- `message` (string): 日志消息

**示例：**
```lua
log_info("这是一条信息日志")
```

### log_debug

输出 DEBUG 级别日志 (仅在 debug 构建中有效)。

**参数：**
- `message` (string): 日志消息

**示例：**
```lua
log_debug("这是一条调试日志")
```

### log_error

输出 ERROR 级别日志。

**参数：**
- `message` (string): 日志消息

**示例：**
```lua
log_error("这是一条错误日志")
```

### set_ignore_policy

设置是否忽略指定 policy。

**参数：**
- `policy` (number): CPU policy 编号
- `val` (boolean): true 为忽略，false 为恢复

**引入版本：** v3 (所有版本可用)

**说明：** 被忽略的 policy 将不会被 FAS 调整频率。

**示例：**
```lua
-- 忽略 policy0
set_ignore_policy(0, true)

-- 恢复 policy0
set_ignore_policy(0, false)
```

### set_extra_policy_abs

设置绝对频率约束。

**参数：**
- `policy` (number): CPU policy 编号
- `min` (number|nil): 最小频率 (kHz)，nil 表示不限制
- `max` (number|nil): 最大频率 (kHz)，nil 表示不限制

**引入版本：** v4 (所有版本可用)

**说明：** 绝对约束限制 policy 的频率范围。约束在 `reset_cpu_freq` 或 `remove_extra_policy` 时清除。

**示例：**
```lua
-- 限制 policy4 频率在 1.0-2.0 GHz 之间
set_extra_policy_abs(4, 1000000, 2000000)

-- 只限制最大频率
set_extra_policy_abs(4, nil, 2000000)

-- 只限制最小频率
set_extra_policy_abs(4, 1000000, nil)
```

### set_extra_policy_rel

设置相对频率约束。

**参数：**
- `policy` (number): CPU policy 编号
- `target_policy` (number): 参考的 CPU policy 编号
- `min` (number|nil): 相对最小偏移 (kHz)，nil 表示不限制
- `max` (number|nil): 相对最大偏移 (kHz)，nil 表示不限制

**引入版本：** v4 (所有版本可用)

**说明：** 相对约束使 policy 的频率跟随另一个 policy，并限制偏移范围。适用于大小核架构。

**示例：**
```lua
-- policy6 的频率跟随 policy4，最多高 500 MHz
set_extra_policy_rel(6, 4, 0, 500000)

-- policy6 的频率跟随 policy4，范围在 -200 ~ +800 MHz
set_extra_policy_rel(6, 4, -200000, 800000)
```

### remove_extra_policy

移除指定 policy 的额外约束。

**参数：**
- `policy` (number): CPU policy 编号

**引入版本：** v4 (所有版本可用)

**示例：**
```lua
remove_extra_policy(4)
```

### set_policy_freq_offset (已弃用)

设置 policy 频率偏移。

**状态：** v4.2.0 已移除

**说明：** 此函数已被 `set_extra_policy_abs` 和 `set_extra_policy_rel` 替代。调用时会输出警告日志。

## 完整插件示例

### 简单日志插件

```lua
-- 简单日志插件
api_version = 4

function load_fas(pid, pkg)
    log_info("游戏加载: " .. pkg)
end

function unload_fas(pid, pkg)
    log_info("游戏卸载: " .. pkg)
end

function start_fas()
    log_info("FAS 开始")
end

function stop_fas()
    log_info("FAS 停止")
end
```

### 性能策略插件

```lua
-- 性能策略插件：限制大核频率
api_version = 4

function init_cpu_freq()
    -- 假设 policy6 是大核
    -- 限制最大 2.5 GHz
    set_extra_policy_abs(6, nil, 2500000)
    log_info("已限制大核最大频率为 2.5 GHz")
end

function reset_cpu_freq()
    remove_extra_policy(6)
    log_info("已恢复大核频率限制")
end
```

### 大小核协调插件

```lua
-- 大小核协调插件
api_version = 4

function init_cpu_freq()
    -- 假设 policy0 是小核，policy4 是中核，policy6 是大核
    -- 中核频率跟随小核，最多高 800 MHz
    set_extra_policy_rel(4, 0, 0, 800000)
    -- 大核频率跟随中核，最多高 1000 MHz
    set_extra_policy_rel(6, 4, 0, 1000000)
    log_info("已设置大小核协调策略")
end

function reset_cpu_freq()
    remove_extra_policy(4)
    remove_extra_policy(6)
    log_info("已清除大小核协调策略")
end
```

### 温控增强插件

```lua
-- 温控增强插件
api_version = 4

local is_throttling = false

function start_fas()
    is_throttling = false
end

function target_fps_change(target_fps, pkg)
    -- 检测帧率下降，可能是温控降频
    if target_fps < 60 and not is_throttling then
        is_throttling = true
        log_info("检测到可能的温控降频: " .. target_fps .. " FPS")
        -- 降低大核频率以减少发热
        set_extra_policy_abs(6, nil, 1800000)
    elseif target_fps >= 60 and is_throttling then
        is_throttling = false
        log_info("帧率恢复")
        remove_extra_policy(6)
    end
end
```

## 调试技巧

### 查看日志

```bash
# 实时查看日志
logcat -s fas-rs
```

### 测试插件

1. 将插件放入 `/dev/fas_rs/extensions/`
2. 插件会自动加载
3. 修改插件后自动重载

### 常见问题

**Q: 插件没有执行？**
- 检查 `api_version` 是否正确设置
- 检查回调函数名是否拼写正确
- 查看 logcat 是否有加载错误

**Q: set_extra_policy 无效？**
- 确认 policy 编号正确 (查看 `/sys/devices/system/cpu/cpufreq/`)
- 确认在 `init_cpu_freq` 中调用 (在 Working 状态初始化时)
- 确认在 `reset_cpu_freq` 中清除

**Q: 频率没有变化？**
- 确认 policy 没有被 `set_ignore_policy` 忽略
- 检查频率约束是否合理 (不要超过硬件限制)
- 查看 debug 日志了解约束应用情况
