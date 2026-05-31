# doit — 以 Shell 为唯一工具的 AI Agent

[![Crates.io](https://img.shields.io/crates/v/doit-agent)](https://crates.io/crates/doit-agent)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

[English](README.md) | [中文](README.zh-CN.md)

**doit** 是一个命令行 AI Agent，只给 LLM 一个工具：Shell。没有内置的代码编辑器、文件浏览器、网络搜索函数——只有一个 `sh`。Agent 通过真实 PTY 执行 Shell 命令，看到原始输出（含 ANSI 转义码），反复迭代直到任务完成。

## 目录

- [设计哲学](#设计哲学)
- [功能特性](#功能特性)
- [环境要求](#环境要求)
- [安装](#安装)
- [快速开始](#快速开始)
- [使用模式](#使用模式)
- [内置命令](#内置命令)
- [配置系统](#配置系统)
- [会话管理](#会话管理)
- [Web 界面](#web-界面)
- [架构设计](#架构设计)
- [开发指南](#开发指南)
- [许可证](#许可证)

## 设计哲学

- **一切皆命令** — Agent 唯一的工具是执行 Shell 命令。它通过组合命令来创造自己的工作流。
- **真实终端体验** — Agent 看到的和人类看到的一致：原始 ANSI 输出、stderr 混排、退出码。
- **自我监管** — Agent 自行评估操作安全性。不设硬编码危险规则。不确定时主动向用户请求确认。
- **可审计** — 每次对话以人类可读的 JSONL 格式存储。可以回放、检查、验证每一个决策。

## 功能特性

### 交互式 REPL

启动与 Agent 的对话。Agent 询问任务，执行命令，通过流式 TUI 实时展示结果，并提出后续问题。

### 一次性任务

给 Agent 一个任务描述，它自主执行，逐回合显示进度，最后打印总结并退出。

### 子 Agent 模式

非交互式任务执行，适合作为其他 Agent 的工具使用。输出详细程度可配置。

### 会话恢复

所有对话以 JSONL 持久化。恢复任意历史会话，从上次中断的地方继续——Agent 看到与原来完全一致的上下文。

### Web 界面

可选的浏览器 UI，分栏布局：左侧对话历史，右侧详情视图。通过 WebSocket 实时流式传输 Agent 的思考过程和命令输出。

### 丰富的内置命令

Agent 可以调用结构化的 CLI 工具（glob、read、search、write、edit），提供高效、可预测的文件系统访问——但全部通过同一个 `sh` 接口调用。

## 环境要求

- **Rust nightly**（edition 2024）— 通过 [rustup](https://rustup.rs/) 安装：`rustup toolchain install nightly`
- [DeepSeek API Key](https://platform.deepseek.com/)（或任意 OpenAI 兼容端点）
- Web 界面需要：[bun](https://bun.sh/)（构建前端）

## 安装

### 一键安装（Linux & macOS）

```bash
curl -fsSL https://raw.githubusercontent.com/LaneSun/doit/main/install.sh | sh
```

自动下载最新的预编译二进制文件，安装到 `~/.local/bin/`（或 `/usr/local/bin`，如果有写权限）。

强制重新安装（即使已是最新版本）：

```bash
curl -fsSL https://raw.githubusercontent.com/LaneSun/doit/main/install.sh | sh -s -- --force
```

### 使用 Cargo

```bash
cargo install doit-agent
```

### 从源码构建

```bash
git clone https://github.com/LaneSun/doit.git
cd doit

# 构建前端（可选，不需要 Web UI 可跳过）
cd web && bun install && bun run build && cd ..

# 构建并安装
bash scripts/install.sh
```

或手动：

```bash
cd web && bun install && bun run build && cd ..
cargo build --release
cp target/release/doit ~/.local/bin/
```

### 不包含 Web UI

如果不需要 Web 界面，可以跳过前端构建：

```bash
mkdir -p web/build && touch web/build/index.html
cargo build --release
```

## 快速开始

1. 设置 API 密钥：

```bash
export DEEPSEEK_API_KEY=sk-your-key-here
```

或永久配置：

```bash
doit config set api.api_key sk-your-key-here
```

2. 启动交互式会话：

```bash
doit
```

3. 执行一次性任务：

```bash
doit run "列出所有 Rust 文件并统计行数"
```

4. 打开 Web 界面：

```bash
doit web
# 然后访问 http://localhost:3456
```

## 使用模式

### `doit` — 交互式 REPL

启动流式 TUI 会话。Agent 会询问你的需求，执行命令，反复迭代。按一次 Ctrl+C 中断当前命令（Agent 自行决定如何处理），按两次退出。

### `doit run <任务>` — 一次性执行

以逐回合模式执行单个任务。Agent 持续工作直到调用 `doit exit`，然后打印总结并退出。适合脚本和快速操作。

```
doit run "修复 src/ 下所有 clippy 警告"
```

### `doit task <任务>` — 子 Agent 模式

非交互式任务执行，设计用于被其他 Agent 作为工具调用。输出通过 `--verbosity` 控制。

```
doit task "运行测试套件并报告失败项"
```

### `doit resume <id>` — 恢复会话

恢复之前的任意对话。Agent 从 JSONL 重建完整上下文，从上次停止的地方继续。

```
doit resume abc12345
```

### `doit web` — Web 界面

启动本地 HTTP 服务器，提供浏览器 UI。界面包含对话面板（流式输出）、详情视图（查看命令和推理）、会话控制。

```
doit web
# 访问 http://localhost:3456
```

## 内置命令

Agent 通过其 `sh` 工具调用这些命令。它们专为 LLM 使用而设计——每个命令输出结构化、带行号的文本。

| 命令 | 用途 |
|------|------|
| `doit prompt <消息>` | 阻塞等待用户输入 |
| `doit exit <总结>` | 完成当前任务退出 |
| `doit exec [选项] -- <命令>` | 通过 PTY 执行 Shell 命令（含输出截断） |
| `doit glob <模式>` | 按 glob 模式匹配文件路径 |
| `doit read <文件> [--lines N:M]` | 读取文件内容（带行号） |
| `doit search <模式> [--include glob]` | 正则搜索（输出 文件:行号:内容） |
| `doit write <文件> [--append]` | 写入文件（从 stdin 读取） |
| `doit edit <文件> --lines N:M \| --regex pat --replace rep \| <<'DIFF'` | 结构化编辑 |
| `doit template <类型>` | 生成提示/模板文本 |
| `doit config [set\|get\|list]` | 查看或编辑配置 |

使用 `--skill` 参数查看任意命令面向 LLM 的文档。

## 配置系统

doit 使用分层配置系统（高优先级覆盖低优先级）：

1. CLI 参数（`--model`、`--config`）
2. 环境变量（`DOIT_API__BASE_URL` 等，双下划线嵌套映射）
3. `--config <path>` 指定的文件
4. `./doit.toml`（项目级）
5. `~/.config/doit/config.toml`（用户级）
6. 内置默认值

### 示例 `doit.toml`

```toml
[api]
base_url = "https://api.deepseek.com"
model = "deepseek-v4-pro"
api_key = "${DEEPSEEK_API_KEY}"
temperature = 0.7
max_tokens = 8192

[output]
truncate_chars = 2000
truncate_lines = 50

[locale]
lang = "zh-CN"
```

### API Key 安全

使用 `${ENV_VAR}` 语法引用环境变量——绝不将密钥以明文存储。

## 会话管理

每次对话存储在 `.doit/sessions/<id>/`：

```
.doit/sessions/abc12345/
├── conversation.jsonl    # 完整对话（JSONL 格式）
├── scratchpad.md         # Agent 草稿本（含模板）
└── logs/
    └── xyz98765.log      # 被截断的完整命令输出
```

- **确定性恢复**：存储的 `output` 是原始 ANSI 输出。恢复时剥离 ANSI 码重建 API 消息——Agent 看到与原来一致的上下文。
- **JSONL 格式**：每行一个完整的 JSON 对象。仅追加。人类可读，可用 grep 搜索。
- **会话 ID**：8 位随机标识符。`doit resume` 列出所有会话及时间戳。

## Web 界面

Web UI（`doit web`）提供：

- **分栏布局**：左侧对话列表，右侧详情视图，可拖拽调整宽度
- **流式输出**：Agent 的思考过程和回复通过 WebSocket 实时渲染
- **终端渲染**：命令输出在嵌入式 xterm.js 终端中显示（ANSI 完整保留）
- **Markdown 渲染**：思考和回复内容完整支持 GFM
- **可折叠条目**：思维块和命令可折叠，阅读更清爽
- **响应式**：1200px 以下单栏，以上双栏

前端为 SvelteKit SPA（使用 bun 构建，通过 rust-embed 在编译时嵌入）。

## 架构设计

### 协议

doit 使用 DeepSeek 原生 Function Calling，定义单一工具 `sh`。每个回合产生三种 Block：

```jsonl
{"seq":1,"role":"system","content":"你是 doit Agent..."}
{"seq":2,"role":"assistant","reasoning":"...","cmd":"ls -la","tool_call_id":"call_1"}
{"seq":3,"role":"tool","output":"原始 ANSI 输出","exit_code":0,"tool_call_id":"call_1"}
```

### 关键依赖

| 层 | Crate | 用途 |
|----|-------|------|
| CLI | `clap` (derive) | 声明式命令解析 |
| 异步 | `tokio` (full) | 多线程运行时 |
| 配置 | `figment` + `toml` | 分层 TOML + 环境变量插值 |
| 错误 | `miette` | 富诊断输出 |
| 国际化 | `rust-i18n` | 编译期翻译嵌入 |
| LLM | `reqwest` | OpenAI 兼容 API + 流式 |
| 终端 | `portable-pty` | 真实 PTY 命令执行 |
| Web | `axum` + `tower-http` | HTTP + WebSocket 服务 |
| 前端 | `rust-embed` | 嵌入式 SvelteKit SPA |
| 会话 | `serde_json` | JSONL 读写 |

### 设计决策

- **无硬编码能力**：Agent 的唯一工具是 `sh`。所有内置命令本质上是恰好随 doit 发行的普通 CLI 程序。
- **真实 PTY 执行**：命令在真实伪终端中运行。Agent 看到的原始 ANSI 输出、stderr、退出码与人类看到的完全一致。
- **隐式包装**：LLM 调用的所有命令被 `doit exec` 隐式包装，处理输出截断、日志记录和 ANSI 感知的截断边界。
- **流式 assistant 块**：推理和回复内容逐 token 流式追加到同一条 JSONL 条目，直到回合完成。

## 开发指南

### 环境

- Rust nightly：`rustup toolchain install nightly`
- bun（前端构建）：`curl -fsSL https://bun.sh/install | bash`

### 构建

```bash
# 完整构建（含 Web UI）
cd web && bun install && bun run build && cd ..
cargo build --release

# 仅 Rust（跳过前端）
cargo build
```

### 测试

```bash
cargo test
```

集成测试使用 `assert_cmd` + `insta` 快照测试。设置 `LANG=en_US.UTF-8` 以锁定英文输出。

### 提交规范

本项目使用 [jj](https://jj-vcs.github.io/jj/)（Jujutsu）进行版本控制。使用 `jjit commit` 生成 AI 辅助的约定式提交信息。

## 许可证

MIT 许可证。详见 [LICENSE](LICENSE)。
