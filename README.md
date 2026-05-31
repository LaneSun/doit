# doit — A Shell-First AI Agent

[![Crates.io](https://img.shields.io/crates/v/doit)](https://crates.io/crates/doit)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

[English](README.md) | [中文](README.zh-CN.md)

**doit** is a command-line AI agent that gives the LLM exactly one tool: a shell. There are no built-in code editors, no file browsers, no web search functions — just `sh`. The agent executes shell commands via a real PTY, sees their raw output (ANSI and all), and iterates until the task is done.

## Table of Contents

- [Philosophy](#philosophy)
- [Features](#features)
- [Requirements](#requirements)
- [Installation](#installation)
- [Quick Start](#quick-start)
- [Usage Modes](#usage-modes)
- [Built-in Commands](#built-in-commands)
- [Configuration](#configuration)
- [Session Management](#session-management)
- [Web UI](#web-ui)
- [Architecture](#architecture)
- [Development](#development)
- [License](#license)

## Philosophy

- **One tool, all tasks** — the agent has no hard-coded capabilities. It invents its own workflows by composing shell commands.
- **Real terminal output** — the agent sees exactly what a human sees: raw ANSI output, stderr intermixed, exit codes.
- **Self-regulation** — the agent evaluates its own safety. No hard-coded danger rules. It asks the user for confirmation when uncertain.
- **Auditable** — every conversation is stored as human-readable JSONL. You can replay, inspect, and verify every decision.

## Features

### Interactive REPL

Start a conversation with the agent. It asks what you want, executes commands, shows you results in real-time via a streaming TUI, and asks follow-up questions.

### One-Shot Tasks

Give the agent a single task description and it runs autonomously, displaying progress turn-by-turn, then prints a summary and exits.

### Sub-Agent Mode

Non-interactive task execution suitable for use as a tool by other agents. Output verbosity is configurable.

### Session Resume

All conversations are persisted as JSONL. Resume any previous session and continue exactly where you left off — the agent sees the same context it had before.

### Web UI

An optional browser-based interface with a split-panel layout: conversation history on the left, detail view on the right. Real-time streaming of agent reasoning and command output via WebSocket.

### Rich Built-in Commands

The agent has access to structured CLI tools (glob, read, search, write, edit) that give it efficient, predictable access to the filesystem — but they are all invoked through the same `sh` interface.

## Requirements

- **Rust nightly** (edition 2024) — install via [rustup](https://rustup.rs/): `rustup toolchain install nightly`
- [DeepSeek API key](https://platform.deepseek.com/) (or any OpenAI-compatible endpoint)
- For the web UI: [bun](https://bun.sh/) (to build the frontend)

## Installation

### Using Cargo

```bash
cargo install doit
```

### From Source

```bash
git clone https://github.com/LaneSun/doit.git
cd doit

# Build the web frontend (optional, skip if you don't need the web UI)
cd web && bun install && bun run build && cd ..

# Build and install
bash scripts/install.sh
```

Or manually:

```bash
cd web && bun install && bun run build && cd ..
cargo build --release
cp target/release/doit ~/.local/bin/
```

### Without Web UI

If you don't need the web interface, you can build without the frontend:

```bash
# Create an empty placeholder so rust-embed doesn't fail
mkdir -p web/build && touch web/build/index.html
cargo build --release --no-default-features
```

## Quick Start

1. Set your API key:

```bash
export DEEPSEEK_API_KEY=sk-your-key-here
```

Or configure it permanently:

```bash
doit config set api.api_key sk-your-key-here
```

2. Start an interactive session:

```bash
doit
```

3. Run a one-shot task:

```bash
doit run "List all Rust files and count their lines"
```

4. Open the web UI:

```bash
doit web
# Then open http://localhost:3456
```

## Usage Modes

### `doit` — Interactive REPL

Starts a streaming TUI session. The agent will ask you what you want to do, execute commands, and iterate. Press Ctrl+C once to interrupt the current command (the agent decides how to handle it), twice to exit.

### `doit run <task>` — One-Shot Execution

Executes a single task with turn-by-turn display. The agent works until it calls `doit exit`, then prints the summary and exits. Useful for scripting and quick operations.

```
doit run "Fix all clippy warnings in src/"
```

### `doit task <task>` — Sub-Agent Mode

Non-interactive task execution designed for use as a tool by other agents. Output is controlled by `--verbosity`.

```
doit task "Run the test suite and report failures"
```

### `doit resume <id>` — Resume Session

Resume any previous conversation. The agent reconstructs the full context from JSONL and continues from where it stopped.

```
doit resume abc12345
```

### `doit web` — Web Interface

Launches a local HTTP server with a browser-based UI. The interface provides a conversation panel with streaming output, a detail view for inspecting commands and reasoning, and session controls.

```
doit web
# Open http://localhost:3456
```

## Built-in Commands

The agent invokes these through its `sh` tool. They are designed to be efficient and predictable for LLM use — each outputs structured, line-numbered text.

| Command | Purpose |
|---------|---------|
| `doit prompt <msg>` | Block and wait for user input |
| `doit exit <summary>` | Complete the current task |
| `doit exec [opts] -- <cmd>` | Execute a shell command via PTY (with output truncation) |
| `doit glob <pattern>` | Match file paths by glob pattern |
| `doit read <file> [--lines N:M]` | Read file content with line numbers |
| `doit search <pattern> [--include glob]` | Regex search with file:line:content output |
| `doit write <file> [--append]` | Write content to a file (reads from stdin) |
| `doit edit <file> --lines N:M | --regex pat --replace rep | <<'DIFF'` | Structured file editing |
| `doit template <type>` | Generate prompt/system templates |
| `doit config [set|get|list]` | View or edit configuration |

Run any command with `--skill` to see its LLM-facing documentation.

## Configuration

doit uses a layered configuration system (higher overrides lower):

1. CLI flags (`--model`, `--config`)
2. Environment variables (`DOIT_API__BASE_URL`, etc. — double-underscore nesting)
3. `--config <path>` file
4. `./doit.toml` (project-level)
5. `~/.config/doit/config.toml` (user-level)
6. Built-in defaults

### Example `doit.toml`

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

### API Key Security

Use `${ENV_VAR}` syntax to reference environment variables — never store API keys in plain text.

## Session Management

Every conversation is stored under `.doit/sessions/<id>/`:

```
.doit/sessions/abc12345/
├── conversation.jsonl    # Full conversation in JSONL format
├── scratchpad.md         # Agent scratchpad (with template)
└── logs/
    └── xyz98765.log      # Truncated command outputs
```

- **Deterministic recovery**: the stored `output` is the raw ANSI output. On resume, ANSI is stripped when rebuilding API messages — the agent sees the same context it originally had.
- **JSONL format**: each line is a self-contained JSON object. Append-only. Human-readable and grep-friendly.
- **Session IDs**: 8-character random identifiers. Listed by `doit resume` with timestamps.

## Web UI

The web UI (`doit web`) provides:

- **Split-panel layout**: conversation list (left) and detail view (right), resizable via drag
- **Streaming output**: agent reasoning and content render in real-time via WebSocket
- **Terminal rendering**: command outputs displayed in an embedded xterm.js terminal (ANSI preserved)
- **Markdown rendering**: reasoning and content rendered with full GFM support
- **Collapsible entries**: thinking blocks and commands can be collapsed for cleaner reading
- **Responsive**: single-panel below 1200px, dual-panel above

The frontend is a SvelteKit SPA (built with bun, embedded via rust-embed at compile time).

## Architecture

### Protocol

doit uses DeepSeek's native Function Calling with a single tool `sh`. Each turn produces three blocks:

```jsonl
{"seq":1,"role":"system","content":"You are doit..."}
{"seq":2,"role":"assistant","reasoning":"...","cmd":"ls -la","tool_call_id":"call_1"}
{"seq":3,"role":"tool","output":"raw ANSI output","exit_code":0,"tool_call_id":"call_1"}
```

### Key Crates

| Layer | Crate | Purpose |
|-------|-------|---------|
| CLI | `clap` (derive) | Declarative command parsing |
| Async | `tokio` (full) | Multi-threaded runtime |
| Config | `figment` + `toml` | Layered TOML with env var interpolation |
| Error | `miette` | Rich diagnostic output |
| i18n | `rust-i18n` | Compile-time translation embedding |
| LLM | `reqwest` | OpenAI-compatible API calls with streaming |
| Terminal | `portable-pty` | Real PTY for command execution |
| Web | `axum` + `tower-http` | HTTP + WebSocket server |
| Frontend | `rust-embed` | Embedded SvelteKit SPA |
| Session | `serde_json` | JSONL read/write |

### Design Decisions

- **No hard-coded capabilities**: the agent's only tool is `sh`. All built-in commands are ordinary CLI programs that happen to ship with doit.
- **Real PTY execution**: commands run in an actual pseudo-terminal. The agent sees raw ANSI output, stderr, and exit codes exactly as a human would.
- **Implicit wrapping**: all LLM-invoked commands are implicitly wrapped by `doit exec`, which handles output truncation, logging, and ANSI-preserving cut boundaries.
- **Streaming assistant blocks**: reasoning and content are streamed token-by-token and appended to the same JSONL entry until the turn completes.

## Development

### Requirements

- Rust nightly: `rustup toolchain install nightly`
- bun (for web frontend): `curl -fsSL https://bun.sh/install | bash`

### Building

```bash
# Full build (with web UI)
cd web && bun install && bun run build && cd ..
cargo build --release

# Rust-only build (skip frontend)
cargo build
```

### Testing

```bash
cargo test
```

Integration tests use `assert_cmd` + `insta` for snapshot testing. Set `LANG=en_US.UTF-8` for deterministic English output.

### Committing

This project uses [jj](https://jj-vcs.github.io/jj/) (Jujutsu) for version control. Use `jjit commit` for AI-generated conventional commits.

## License

MIT License. See [LICENSE](LICENSE) for details.
