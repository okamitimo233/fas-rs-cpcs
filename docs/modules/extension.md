# 插件系统 (Extension)

本文档描述 `src/framework/extension/` 模块的设计与实现。

## 概述

fas-rs 提供基于 LuaJIT 的插件系统，允许用户扩展功能而无需修改核心代码。插件可以实现自定义的 CPU 频率控制策略、日志记录等功能。

## 模块结构

```
extension/
├── mod.rs           # Extension 入口
├── core.rs          # 插件加载与执行
└── api/             # API 定义
    ├── mod.rs       # API trait 和触发函数
    ├── v0.rs        # API v0
    ├── v1.rs        # API v1
    ├── v2.rs        # API v2
    ├── v3.rs        # API v3
    ├── v4.rs        # API v4
    ├── helper_funs.rs # 辅助函数
    └── misc.rs      # 工具函数
```

## Extension 入口

### 初始化

```rust
pub struct Extension {
    sx: SyncSender<Box<dyn Api>>,
}

impl Extension {
    pub fn init() -> Result<Self> {
        let _ = fs::create_dir_all(EXTENSIONS_PATH);
        let (sx, rx) = mpsc::sync_channel(16);

        thread::Builder::new()
            .name("ExtensionThread".into())
            .spawn(move || core::thread(&rx))?;

        Ok(Self { sx })
    }
}
```

Extension 在独立线程中运行，通过同步通道接收 API 触发请求。

### 触发 API

```rust
pub fn trigger_extentions(&self, trigger: impl Api + 'static) {
    let _ = self.sx.try_send(trigger.into_box());
}
```

使用 `try_send` 避免阻塞主线程。

## 插件加载 (core.rs)

### 插件目录

插件存放在 `/dev/fas_rs/extensions/` 目录：

```
/dev/fas_rs/extensions/
├── plugin1.lua
├── plugin2.lua
└── ...
```

### 加载流程

```rust
fn load_extensions() -> Result<ExtensionMap> {
    let mut map: ExtensionMap = HashMap::new();

    for file in fs::read_dir(EXTENSIONS_PATH)?
        .filter(|f| f.path().extension().unwrap() == "lua")
    {
        let lua = Lua::new();
        let path = file.path();
        let file = fs::read_to_string(&path)?;

        // 注册全局函数
        lua.globals().set("log_info", ...)?;
        lua.globals().set("log_debug", ...)?;
        lua.globals().set("log_error", ...)?;
        // ... 更多辅助函数

        // 执行插件
        match lua.load(&file).exec() {
            Ok(()) => {
                info!("Extension loaded successfully: {}", path.display());
                map.insert(path, lua);
            }
            Err(e) => {
                error!("Extension loading failed, reason: {e:#?}");
            }
        }
    }

    Ok(map)
}
```

### 热重载

使用 inotify 监控插件目录变更：

```rust
fn need_update(inotify: &mut Inotify) -> bool {
    inotify.read_events(&mut [0; 1024]).is_ok()
}

// 在主循环中
loop {
    if need_update(&mut inotify) {
        extensions = load_extensions().unwrap_or_default();
    }

    if let Ok(trigger) = rx.recv_timeout(Duration::from_secs(1)) {
        trigger.handle_api(&extensions);
    }
}
```

## Lua 环境

### 全局函数

插件可以访问以下全局函数：

| 函数 | 参数 | 说明 |
|------|------|------|
| `log_info(message)` | string | 输出 INFO 级别日志 |
| `log_debug(message)` | string | 输出 DEBUG 级别日志 |
| `log_error(message)` | string | 输出 ERROR 级别日志 |

### 辅助函数

以下辅助函数对所有 API 版本可用：

| 函数 | 参数 | 引入版本 | 说明 |
|------|------|----------|------|
| `set_ignore_policy(policy, val)` | int, bool | v3 | 忽略指定 policy |
| `set_extra_policy_abs(policy, min, max)` | int, int?, int? | v4 | 设置绝对频率约束 |
| `set_extra_policy_rel(policy, target, min, max)` | int, int, int?, int? | v4 | 设置相对频率约束 |
| `remove_extra_policy(policy)` | int | v4 | 移除额外策略 |

### 已弃用函数

| 函数 | 说明 |
|------|------|
| `set_policy_freq_offset(policy, offset)` | v1 引入，v4.2.0 已移除 |

## 插件示例

### 基础插件

```lua
-- 插件必须定义 api_version
api_version = 4

-- 可选：定义回调函数
function init_cpu_freq()
    log_info("初始化 CPU 频率")
end

function reset_cpu_freq()
    log_info("重置 CPU 频率")
end

function load_fas(pid, pkg)
    log_info("加载 FAS: " .. pkg)
end

function unload_fas(pid, pkg)
    log_info("卸载 FAS: " .. pkg)
end

function start_fas()
    log_info("FAS 开始")
end

function stop_fas()
    log_info("FAS 停止")
end

function target_fps_change(target_fps, pkg)
    log_info("目标帧率变更: " .. target_fps)
end
```

### 高级插件 (API v4)

```lua
api_version = 4

-- 策略约束示例
function init_cpu_freq()
    -- 限制 policy4 最高 2.0 GHz
    set_extra_policy_abs(4, nil, 2000000)

    -- policy6 跟随 policy4，最多高 500 MHz
    set_extra_policy_rel(6, 4, 0, 500000)
end

function reset_cpu_freq()
    -- 移除所有约束
    remove_extra_policy(4)
    remove_extra_policy(6)
end
```

## 执行流程

```
主线程触发 API (trigger_xxx)
        ↓
SyncSender.try_send(ApiVx::Xxx)
        ↓
ExtensionThread 接收
        ↓
trigger.handle_api(&extensions)
        ↓
遍历匹配版本的插件
        ↓
调用 Lua 回调函数
```

## 线程安全

- 插件在独立线程中执行，不阻塞主循环
- 使用 `SyncSender` 保证线程安全
- 使用 `try_send` 避免背压

## 错误处理

- 插件加载失败时记录错误但不影响其他插件
- 回调执行错误会被捕获并记录
- 不影响主程序运行
