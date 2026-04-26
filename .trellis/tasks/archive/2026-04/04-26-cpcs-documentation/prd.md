# brainstorm: 为 cpcs 系统编写文档

## Goal

为 cpcs（fas-rs）帧感知调度系统编写技术文档，帮助不同受众理解和使用该系统。

## What I already know

**项目概况：**
- fas-rs/cpcs 是 Android 帧感知调度系统 (Frame Aware Scheduling)
- 运行在用户态，通过 eBPF 监视画面渲染来控制 CPU 性能
- 兼容性强，灵活性高，支持插件系统

**核心模块结构：**
- `src/framework/`: 核心框架
  - `config/`: TOML 配置系统，支持配置合并
  - `scheduler/`: 调度器（主循环、策略控制、温控、topapp 追踪）
  - `extension/`: 插件系统（API v0-v4，支持 Lua 扩展）
  - `node/`: 模式节点控制（powersave/balance/performance/fast）
  - `pid_utils/`: 进程追踪工具
- `src/cpu_common/`: CPU 控制器（频率、温度监控）
- `cpcs-analyzer/`: eBPF 帧分析器
- `third_party/frame-analyzer-patched/`: 第三方帧分析器

**现有文档：**
- `README.md` / `README_EN.md`: 用户安装配置指南
- `.trellis/spec/backend/`: 开发指南（目录结构、错误处理、日志、质量标准）
- `webui/README.md`: Web UI 说明

**技术栈：**
- Rust (Nightly, edition 2024)
- eBPF (通过 cpcs-analyzer-ebpf)
- LuaJIT (插件系统)
- Android 平台

## Assumptions (temporary)

- 文档可能需要覆盖架构设计和开发者指南
- 可能需要使用 rustdoc 生成 API 文档
- 文档可能需要中英双语

## Open Questions

- ~~**目标受众**：文档是面向用户、开发者、还是两者都有？~~ → **开发者**
- ~~**文档范围**：需要哪些类型的文档？~~ → **架构文档 + API 文档**
- ~~**文档格式与位置**：Markdown 放在仓库内，还是需要外部文档站点？~~ → **Markdown，存放在 `docs/` 目录**
- ~~**语言**：是否需要中英双语版本？~~ → **仅中文**

（所有核心问题已解决）

## Requirements (evolving)

**目标受众：开发者**
- 需要理解系统架构的开发者
- 需要开发插件的开发者
- 需要贡献代码的社区成员

**文档范围：**
1. **架构文档**
   - 系统整体架构图
   - 模块划分与职责
   - 核心数据流（帧检测 → 调度决策 → CPU 控制）
   - FAS 原理与调度策略说明
   - 配置系统工作原理

2. **API 文档**
   - 各模块公开接口说明
   - 插件 API (v0-v4) 详细文档
   - Rust 文档注释（通过 rustdoc 生成）

3. **文档索引页**
   - `docs/README.md` 作为文档入口
   - 提供文档结构导航

## Acceptance Criteria (evolving)

- [ ] `docs/README.md` 文档索引页完成
- [ ] 架构文档覆盖所有核心模块（framework、cpu_common、analyzer）
- [ ] 插件 API (v0-v4) 文档完整，包含接口说明和使用示例
- [ ] 文档内容准确反映当前代码实现

## Technical Approach

**文档结构：**
```
docs/
├── README.md              # 文档索引
├── architecture.md        # 系统架构
├── modules/               # 模块详细文档
│   ├── framework.md       # 核心框架
│   ├── scheduler.md       # 调度器
│   ├── extension.md       # 插件系统
│   └── config.md          # 配置系统
└── api/
    └── extension-api.md   # 插件 API 文档
```

**实现方式：**
1. 通过阅读源码理解各模块职责和接口
2. 编写 Markdown 文档
3. 确保文档与代码一致

## Definition of Done (team quality bar)

- 文档内容准确完整
- 符合项目文档风格（参考现有 spec/backend 文档）
- 通过审阅确认

## Out of Scope (explicit)

- 用户手册（安装、配置指南已有 README.md）
- 贡献指南（开发环境搭建、PR 流程）
- 性能调优指南
- 插件开发教程（已有插件模板仓库）
- `.trellis/spec/` 相关内容（个人工作流，不提交上游）
- 中英双语版本

## Technical Notes

- 项目使用 Rust edition 2024，需要 nightly 工具链
- 插件系统支持 LuaJIT
- 配置文件位于 `/sdcard/Android/fas-rs/games.toml`
