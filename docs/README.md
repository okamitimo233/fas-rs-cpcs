# CPCS (fas-rs) 开发者文档

欢迎使用 fas-rs/cpcs 帧感知调度系统开发者文档。本文档面向需要理解系统架构、开发插件或贡献代码的开发者。

## 文档索引

### 架构文档

- [系统架构](architecture.md) - 系统整体架构、模块划分、核心数据流、FAS 原理

### 模块文档

- [核心框架](modules/framework.md) - Framework 模块概述、模块组织、关键类型
- [调度器](modules/scheduler.md) - 调度器架构、主循环、状态机、控制策略
- [插件系统](modules/extension.md) - 插件机制、加载流程、Lua 环境
- [配置系统](modules/config.md) - 配置文件结构、合并机制、热重载

### API 文档

- [插件 API](api/extension-api.md) - 插件 API 版本演进、回调函数、辅助函数

## 项目概述

fas-rs/cpcs 是 Android 平台的帧感知调度系统 (Frame Aware Scheduling)。系统运行在用户态，通过 eBPF 技术监视画面渲染，根据帧率动态调整 CPU 性能，实现功耗与性能的平衡。

### 核心特性

- **帧感知调度**：通过 eBPF 监测帧时间，实现精准的性能控制
- **CPCS 分析器**：基于 DAG (有向无环图) 的关键路径分析，智能分配 CPU 资源
- **插件系统**：支持 LuaJIT 插件扩展，提供灵活的自定义能力
- **多模式支持**：支持省电、均衡、性能、快速四种调度模式

### 技术栈

- **Rust** (Nightly, edition 2024)
- **eBPF** (通过 aya 框架)
- **LuaJIT** (插件系统)
- **Android 平台**

## 快速开始

### 项目结构

```
fas-rs/
├── src/                    # 主程序源码
│   ├── main.rs             # 入口点
│   ├── cpu_common/         # CPU 频率控制
│   └── framework/          # 核心框架
├── cpcs-analyzer/          # CPCS eBPF 分析器
├── cpcs-analyzer-common/   # 分析器共享代码
├── cpcs-analyzer-ebpf/     # eBPF 内核程序
├── third_party/            # 第三方库
│   └── frame-analyzer-patched/
└── xtask/                  # 构建任务
```

### 构建与运行

```bash
# 构建
cargo xtask build --release

# 运行
fas-rs run /path/to/games.toml
```

## 相关资源

- [用户手册](../README.md) - 安装与配置指南
- [贡献指南](../CONTRIBUTING.md) - 开发环境与贡献流程
