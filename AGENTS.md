## Version Control

This project uses **jj (Jujutsu)** instead of git for version control.

### Committing changes

Always prefer `jjit commit` over manual `jj describe` + `jj new`:

```bash
jjit commit
```

This automatically summarizes the working copy changes using LLM and creates a commit.

Useful flags:
- `--no-thinking` — hide the LLM thinking process
- `--show-prompt` — debug the prompt sent to LLM

### Viewing history

```bash
jj log
```

### Checking out revisions

You can use native jj commands:

```bash
jj checkout <revision>
# or
jj co <revision>
```

Or use the AI-powered `jjit goto` to find and checkout by description:

```bash
jjit goto
```

### Other common jj commands

```bash
jj status           # Show working copy status
jj diff             # Show changes in working copy
jj abandon <rev>    # Abandon a revision
jj squash           # Squash working copy into parent
jj split            # Split a revision into two
```

Note: jj automatically syncs with the underlying git repo, so git-compatible operations work seamlessly.

---

## Commit Rhythm

- Commit early and often
- Use `jjit commit` for all commits
- Each logical unit of work gets its own commit

---

## Project Philosophy

### 总体精神

- **极简主义**：能复用的逻辑就复用，不引入不必要的抽象。一切以 Shell 命令为核心。
- **不重复造轮子**：优先选用社区成熟 crate，遵循 Rust 生态最佳实践。
- **形式服从功能**：不考虑无实际用途的扩展性设计，只在确实需要的地方抽象。
- **一致性优先于便捷**：所有模式（交互/Run/Task）使用统一的底层机制，不搞特殊情况。

### 设计要求

- **向下兼容**：Block Schema、JSONL 格式、会话目录结构从第一天就要稳定。
- **确定性恢复**：每次存储的 `output` 是原始 ANSI 输出，重建 API 消息时剥离 ANSI。保证同一段对话恢复后的上下文与原始完全一致。
- **安全**：绝不硬编码 API Key 或敏感信息，支持环境变量引用。
- **可审计**：所有对话以 JSONL 存储，可阅读、可排查。

---

## Coding Workflow

### 四步协同流程

1. **提出方向** — Agent 先提出可能和建议的方向，交由用户讨论
2. **设计讨论** — 方向确定后，Agent 提出具体设计，用户审阅修改
3. **实现细节** — 设计确定后，Agent 提出具体实现细节，用户审阅
4. **编码实现** — 用户确认后，Agent 进行实际编码

决不跳过或合并步骤。

简化流程：当设计已确认时，可直接进行「编码实现」然后交由用户审计。

### 编码后讲解

每次编码完成后，讲解大致逻辑和关键决策，包含：
- 做了什么
- 为什么这样做
- 有哪些值得注意的细节

### 提交时机

- 用户审计完一阶段代码并明确确认后，才可 `jjit commit`
- 未获用户确认前绝不提交

### 编码阶段顺序

按依赖关系从下到上：

| 阶段 | 内容 | Crate 依赖 |
|------|------|-----------|
| 1 | 项目骨架 | Cargo.toml、目录结构、miette 错误类型 |
| 2 | 配置系统 | 多层 TOML 加载、环境变量覆盖、验证 |
| 3 | i18n | `locales/` 翻译文件、`rust-i18n` 加载 |
| 4 | CLI 解析 | clap derive 命令定义 |
| 5 | Block 定义 + JSONL 读写 | 会话持久化核心数据类型 |
| 6 | LLM 后端 | `async-openai` 封装、API 消息重建 |
| 7 | 会话管理 | 目录创建、ID 生成、加载/追加/恢复 |
| 8 | Shell 运行时 | `doit exec` 截断/日志、`doit prompt` 阻塞 |
| 9 | 内置子命令 | glob/read/search/write/edit |
| 10 | 系统提示生成 | `doit template system` |
| 11 | Agent 循环 | 核心调度：send → parse → execute → repeat |
| 12 | 流式 TUI + 中断 | 交互式展示、Ctrl+C 处理 |
| 13 | 测试 + 文档 | 集成测试、使用文档 |

### 每阶段开始前

- 讨论该阶段的实现计划
- 确认该阶段的交付标准
- 明确与其他阶段的接口边界
- 确定依赖的外部 crates

---

## Coding Conventions

### 子命令结构

每个内置子命令位于 `src/commands/<name>/`，包含：

```
src/commands/<name>/
├── mod.rs                        # clap Args + execute 函数
├── locales/
│   ├── en.toml                   # 英文翻译
│   └── zh-CN.toml                # 中文翻译
└── tests/
    └── integration/
        ├── main.rs               # 集成测试 (insta golden file)
        └── snapshots/            # 生成，不入版本控制
```

命令行注册流程：
1. 创建 `src/commands/<name>/` 目录和文件
2. `src/commands/mod.rs` 添加 `pub mod <name>;`
3. `src/cli.rs` 的 `Command` 枚举添加变体
4. `src/main.rs` 的 `match` 添加分发分支
5. `Cargo.toml` 添加 `[[test]]` 条目
6. `locales/en.toml` 和 `zh-CN.toml` 添加翻译 key

### i18n 规范

- 每个子命令的翻译 key 以其命令名为前缀：`prompt.skill`、`exit.skill`、`read.truncated`
- 子命令 locale 源文件位于 `src/commands/<name>/locales/`，`build.rs` 在编译时自动合并到 `locales/`
- `locales/` 目录由 build.rs 生成，已加入 `.gitignore`
- 基础翻译（`tracing.startup`）位于 `src/locales/`

### 测试规范

- 集成测试为主，使用 `assert_cmd` + `insta` golden file
- 测试文件位于各子命令的 `tests/integration/` 下
- 通用集成测试位于 `tests/integration/`
- 测试中固定 `LANG=en_US.UTF-8` 以锁定英文输出
- insta snapshots 由 `cargo insta accept` 管理

### 子命令约定

- 所有子命令接受 `--skill` 参数，输出 i18n 化的技能参考文本给 LLM
- 文件操作类子命令输出带行号的格式：`<行号>: <内容>`（冒号后有空格）
- 搜索类子命令输出格式：`<文件>:<行号>: <内容>`
- 大量输出需设置默认截断上限，并给出用户友好提示

---

## 设计文档

完整的设计文档位于 `DESIGN.md`。任何架构决策必须反映在设计文档中。

## 参考上下文

本项目目录：`/home/lanesun/Sync/code/sketch/doit`

会话持久化根目录：`.doit/sessions/`（相对于当前工作目录）
