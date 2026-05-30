# 后续任务规划

> 本文件记录已确认但尚未完成的较大任务。每个任务遵循「提方向 → 设计讨论 → 实现细节 → 编码」四步流程，逐项推进。

---

## 任务 1：分级配置系统（TOML）

### 目标

用 TOML 实现完整的多层级配置，覆盖：

- **API 配置**：base_url、api_key（支持 `${ENV}` 引用）等
- **模型配置**：model、temperature、max_tokens、thinking 开关等
- **交互式显示配置**：命令输出显隐、推理显隐、narration 显隐等
- **Prompt 配置**：覆盖内置系统提示（交互/任务两种模式）

### 优先级链（高 → 低，见 DESIGN.md）

1. CLI 参数（`--model`、`--no-truncate` 等）
2. 环境变量（`DOIT_API__BASE_URL`，双下划线表嵌套）
3. `--config <path>` 指定文件
4. `./doit.toml`（项目级，当前工作目录）
5. `~/.config/doit/config.toml`（用户级）
6. 内置默认值

### 当前硬编码点（需迁移到配置）

- `src/agent/mod.rs`：`DEFAULT_MODEL`、`Agent::from_env()`（api_key/base_url/model）
- `src/commands/exec/mod.rs`：`DEFAULT_CHARS=2000`、`DEFAULT_LINES=50`
- `src/agent/mod.rs`：`InteractiveView`/`StreamRender`（reasoning/content/narration/命令输出渲染）
- 系统提示模板（`template` 命令）

### 已定方案

- crate：`figment`（TOML+Env 分层）+ `directories`（XDG）+ `toml_edit`（无损写回）+ `toml`
- schema：`[api] [model] [output] [display] [prompt] [locale]`
- `[prompt]`：`append_*`（追加，保留命令注册表）+ `system_*`（完全覆盖逃生舱）
- 新增 `doit config` 子命令：`path` / `list` / `get <key>` / `set <key> <value> [--project]`

### 状态：已完成（待用户审计）

---

## 任务 2：Web 后端（SvelteKit + Tailwind）

### 目标

交互模式启动后输出一个 localhost 服务地址（`--host` 可开放到局域网），提供 UI 版 Agent 界面：

- 对话与命令条目，默认折叠；运行中的命令展开且支持交互式输入
- 元信息面板：上下文用量、模型、推理强度等
- 配置界面：修改项目级与用户级配置（复用任务 1 的配置系统）

### 关键架构问题（待讨论）

- Web 与终端交互模式的关系：Web 作为同一 Agent 循环的「另一个视图 + 输入源」（复用现有 `TurnView` 抽象），还是独立 headless 服务
- 传输：HTTP + WebSocket（事件广播 + 用户输入回传）
- 运行中命令的 PTY 如何桥接到浏览器（xterm.js ↔ PTY master），实现交互式输入——本任务最难、最新颖的部分
- 前端静态资源分发：`rust-embed` 嵌入二进制 vs 磁盘读取
- Rust 侧 HTTP 框架：`axum`（tokio 生态主流）

### 拟分阶段

1. 事件总线：把 Agent 的流式事件（reasoning/content/narration/命令/输出）抽象为可广播事件
2. axum 服务骨架 + WebSocket + 静态资源分发 + URL 输出
3. SvelteKit + Tailwind 前端：对话/命令折叠列表、元信息面板
4. 运行中命令 PTY ↔ xterm.js 交互式桥接
5. 配置编辑界面（依赖任务 1）

### 状态：方向待讨论（任务 1 完成后展开）
