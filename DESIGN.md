# doit Design Document

## 项目概述

**doit** 是一个以命令行工具为主的 AI Agent，基于"一切皆命令"的理念。设计核心：最小化 LLM 的工具箱，Agent 通过唯一的 `sh` 函数调用执行 Shell 命令来完成所有任务。

---

## 核心原则

- **一切皆命令**：Agent 只拥有一个工具 — 执行 Shell 命令。文件操作、网络请求、代码编辑、文本处理等一切能力通过 Shell 命令实现。
- **极简协议**：使用 DeepSeek Function Calling 原生机制，LLM 的每次响应是 `reasoning + tool_call` 的原子块。
- **真实终端体验**：LLM 看到的命令输出和人类终端用户看到的一致（含 stderr，含 ANSI 格式码）。
- **自我监管**：LLM 自行评估操作危险性并决定是否请求用户确认，不设硬编码安全规则。

---

## 技术栈

| 层 | 选型 | 说明 |
|---|------|------|
| CLI 框架 | clap (derive) | 声明式命令定义 |
| 异步运行时 | tokio (full) | 全部 I/O 异步 |
| 配置 | TOML (serde) | 多层 merge 覆盖 |
| 错误处理 | miette | 富诊断信息 |
| i18n | rust-i18n | 编译期嵌入 |
| HTTP 客户端 | async-openai | 后端 LLM API 调用 |
| 日志 | tracing + tracing-subscriber | 结构化日志 |
| 会话存储 | JSONL | 追加式持久化 |
| XDG 路径 | directories | 用户配置目录 |

---

## 对话协议

### 模型

使用 DeepSeek API 原生 Function Calling。Agent 定义单个工具 `sh`，`tool_choice` 设为 `required`。

**三种 Block（值源格式）：**

```
{seq:1, role:"system", content:"..."}
{seq:2, role:"assistant", reasoning:"...", cmd:"...", tool_call_id:"call_1"}
{seq:3, role:"tool", output:"...", exit_code:0, tool_call_id:"call_1"}
```

| Block | role | 说明 |
|-------|------|------|
| system_prompt | system | `doit template system` 输出。仅一个，在会话开头。含角色定义、协议规则、工具参考等。 |
| assistant | assistant | LLM 每次 API 响应。`reasoning` 为思维内容，`cmd` 为从 `tool_calls[0].function.arguments.command` 提取的命令字符串。 |
| tool | tool | 命令执行的原始输出。`output` 存原始 ANSI 输出，`exit_code` 存退出码，`tool_call_id` 关联对应请求。 |

### 交互模式完整流

```
Block 1: system ─ "你是 doit...\n请首先使用 doit prompt 向用户询问任务。"
                                                        API Call 1
                                                        messages: [system(1)]
Block 2: assistant ─ reasoning: "..."
                      cmd: "doit prompt '请描述你的任务'"
                                                        [阻塞等待用户输入]
Block 3: tool ─ output: "> 把 src/ 下的 js 改成 ts"
                                                        API Call 2
Block 4: assistant ─ reasoning: "..."
                      cmd: "find src/ -name '*.js'"
                                                        [doit exec 执行]
Block 5: tool ─ output: "src/a.js\nsrc/b.js"
...
Block N-1: assistant ─ reasoning: "任务完成"
                        cmd: "doit prompt '已完成，还有什么需要做的？'"
                                                        [阻塞等待]
            ├─ 用户输入新任务 → Block N: tool ─ output:"> ..."
            ├─ 用户 Enter 空  → Block N: tool ─ output:"> <ENTER>"
            └─ 用户 Ctrl+C   → 退出，不记录
```

### Run 模式 (`doit run "任务"`)

```
Block 1: system ─ "你是 doit...\n\n任务: {任务描述}\n\n请开始执行。"
Block 2: assistant ─ reasoning + cmd
...
Block N: assistant ─ reasoning + cmd: "doit exit '总结'"
Block N+1: tool ─ output: "总结"    → 进程退出
```

### Task 模式 (`doit task "任务"`)

与 Run 模式结构相同，差异仅在 system prompt 中不含 `prompt` 子命令的 `--help`。交互式专用的 `prompt` 在非交互模式下不可见。

### API 消息重建

存储的 Block 在每次 API 调用时重建为 OpenAI 兼容格式：

```
messages: [
  {role: "system", content: <system.content>},
  {role: "assistant", content: null, reasoning_content: <assistant.reasoning>,
   tool_calls: [{id: <assistant.tool_call_id>, type: "function",
                  function: {name: "sh", arguments: {command: <assistant.cmd>}}}]},
  {role: "tool", tool_call_id: <tool.tool_call_id>, content: <tool.output 剥除 ANSI>},
  ...
]
```

`reasoning` 内容携带进每次 API 请求以保留 LLM 的记忆完整性。

---

## 会话管理

### 目录结构

```
.doit/sessions/<8-char-id>/
├── conversation.jsonl       # 对话 Block 序列
├── scratchpad.md            # LLM 草稿本（含预设模板）
└── logs/
    └── xyz98765.log         # 溢出输出日志
```

### JSONL 存储

每行一个 JSON 对象，严格按 seq 递增。`raw` 输出（含 ANSI）直接存入 `output` 字段，在 API 消息重建时剥离 ANSI 得到 `content`。

**三种 Block 的 JSON Schema：**

```jsonl
// system
{"seq":1,"role":"system","content":"你是 doit Agent..."}

// assistant
{"seq":2,"role":"assistant","reasoning":"思考内容","cmd":"doit prompt '...'","tool_call_id":"call_1"}

// tool
{"seq":3,"role":"tool","output":"原始输出（含 ANSI）","exit_code":0,"tool_call_id":"call_1"}
```

必要字段：`tool_call_id` 关联 assistant 和对应 tool，`exit_code` 记录命令退出状态。

### 会话恢复 (`doit resume <id>`)

1. 加载 `conversation.jsonl`，校验最后一行完整性，丢弃不完整行
2. 清屏，按 seq 顺序回放所有 Block 的 `output` 到终端
3. 末块为 `assistant` → 提示用户确认是否继续执行未完成的命令
4. 执行命令，追加 tool block，继续 agent 循环

---

## CLI 命令结构

```
doit                        # 交互 REPL，流式 TUI
doit run <task>             # 单次执行，按 Turn 显示，最终总结 → stdout
doit task <task>            # 非交互执行，仅打印 exit 总结
doit resume <id>            # 恢复历史会话
```

### 核心子命令（LLM 可通过 sh 工具调用）

| 子命令 | 用途 | 可用模式 |
|--------|------|:------:|
| `doit prompt <msg>` | 阻塞等待用户输入 | 交互 |
| `doit exit <summary>` | 完成任务退出 | Run/Task |
| `doit exec [opts] -- <cmd>` | Shell 命令包装（截断/日志） | 全部 |
| `doit task <description>` | 子 Agent 执行 | 全部 |
| `doit template <type>` | 生成提示/模板文本 | 内部用 |
| `doit glob <pattern>` | 文件模式匹配 | 全部 |
| `doit read <file> [--lines N:M]` | 片段读取 + 行号 | 全部 |
| `doit search <pattern> [--include glob]` | 内容搜索 + 行号 | 全部 |
| `doit write <file> [--append]` | 安全写入 | 全部 |
| `doit edit <file> --lines N:M \| --regex pat --replace rep \| <<'DIFF'` | 结构化编辑 | 全部 |

### `doit edit` 三种模式

```
# 行号替换
doit edit <file> --lines 10:15 <<'EOF'
...
EOF

# 正则替换（词级）
doit edit <file> --regex "old" --replace "new"

# Git diff 格式（≥3 行上下文，固定 heredoc）
doit edit <file> <<'DIFF'
@@ -10,6 +10,8 @@
...
DIFF
```

替换成功后自动以 `search` 风格显示更改后的行。

### `doit read` 和 `doit search` 输出格式

输出每行前带 `<文件>:<行号>:` 前缀。

### `doit template` 类型

| type | 产出 | 用途 |
|------|------|------|
| `system` | 角色定义 + 协议规则 + 工具 `--help` | system block 的 content |
| `scratchpad` | 草稿本命令字符串 | 内部使用 |
| `ask` | 首次请求用户输入时的提示文本 | 交互模式 assistant 块的 cmd 参数 |

---

## 输出截断

### 规则

- 双限制：2000 字符 / 50 行，OR 触发，字符优先
- 截断方式：头尾保留（前 2000 字符/50 行 + 后 2000 字符/50 行）
- 溢出：完整输出写入 `logs/<8-char>.log`
- 截断通知置于输出末尾新行：`... [truncated: N lines / M chars omitted, full at .doit/sessions/<id>/logs/<log-id>.log]`
- 截断需 ANSI 感知：不在转义序列中间切割，截断边界插入 SGR 重置/恢复

### 交互式显示

`doit exec` 在交互模式下的终端行为：

1. **正常阶段**（输出 < 阈值）：与直接执行命令完全一致，自然滚动
2. **截断触发后**：头部冻结在 scrollback，截断行实时更新计数，尾部区域固定 50 行高度原地重绘（ring buffer）
3. **最终状态** = LLM 所见

### 隐式包装

LLM 通过 `sh` 工具调用的命令在运行时被 `doit exec` 隐式包装。LLM 在系统提示中被告知此行为。如需全量输出可显式调用 `doit exec --no-truncate -- <cmd>`。

---

## LLM 后端

### 抽象层

单一 trait `ChatBackend`，`async-openai` 实现。仅支持 OpenAI 兼容格式接口，默认模型 `deepseek-v4-pro`。

- 无状态：每次 `send` 携带完整 messages 列表
- 单后端：不支持多后端 fallback
- 流式：单次请求返回 reasoning + tool_calls

### API 配置

| 项 | 说明 |
|---|------|
| base_url | OpenAI 兼容端点 |
| model | 默认 `deepseek-v4-pro` |
| api_key | 通过配置系统提供 |
| tool_choice | 固定 `required: {type: "function", function: {name: "sh"}}` |

---

## 系统提示 (`doit template system`)

### 来源

内置硬编码 + i18n，支持多层覆盖（见配置系统）。不同模式下生成内容有差异。

### 内容结构

1. **角色定义**：Agent 身份和能力描述
2. **协议说明**：Function Calling 机制、`sh` 工具定义
3. **执行规则**：`doit exec` 隐式包装、截断机制、日志位置
4. **安全规则**：危险操作自我评估 + 文本确认 + `doit prompt` 获取许可
5. **草稿本说明**：`scratchpad.md` 的用途和位置
6. **模式声明**：交互 / Run / Task
7. **工具参考**：所有可见子命令的 `--help` 输出，以工具描述形式嵌入

### 模式差异

| 子命令工具参考 | 交互 | Run | Task |
|---------------|:---:|:---:|:---:|
| `prompt` | ✓ | ✗ | ✗ |
| `exit` | ✗ | ✓ | ✓ |
| 其他全部 (exec/glob/read/search/write/edit/task) | ✓ | ✓ | ✓ |

### 交互模式尾部注入

```
请首先使用 doit prompt 向用户询问他们想要完成的任务。
```

### Run/Task 模式尾部注入

```
任务: {任务描述}

请开始执行。
```

---

## 中断处理 (Ctrl+C)

1. 用户按 Ctrl+C → 当前命令收到 SIGINT，`^C` 混入命令输出
2. tool block 返回含 `^C` 的输出和 `exit_code:130`
3. 发送给 LLM，由 LLM 自行决定：调用 `doit prompt` 询问用户，或恢复原命令，或其他操作
4. 如果在 `doit prompt` 阻塞中按 Ctrl+C → 等同于输入空，`<ENTER>` 返回 LLM
5. 第二次 Ctrl+C（非 prompt 阻塞状态）→ 退出程序，不记录最后一个 tool block

---

## 配置系统

### 优先级链（高到低）

1. CLI 参数 (`--model`, `--no-truncate` 等)
2. 环境变量 (`DOIT_API__BASE_URL` 等，双下划线嵌套映射)
3. `--config <path>` 指定的配置文件
4. `./doit.toml`（当前工作目录）
5. `~/.config/doit/config.toml`（用户配置目录）
6. 内置默认值

### 配置项

```toml
[api]
base_url = "https://api.deepseek.com"
model = "deepseek-v4-pro"
api_key = "${DOIT_API_KEY}"
temperature = 0.7
max_tokens = 8192

[output]
truncate_chars = 2000
truncate_lines = 50

[locale]
lang = "zh-CN"

[system_prompt]
# template = "..."    # 可选覆盖内置系统提示
```

### API Key 安全

支持 `${ENV_VAR}` 语法引用环境变量，避免明文存储密钥。

### 合并策略

深层 merge：每个 section 独立合并，子键逐项覆盖。

---

## i18n

- 使用 `rust-i18n`，编译期嵌入
- 翻译文件位于 `locales/`，支持 `en` 和 `zh-CN`
- 翻译内容：系统提示各段落、截断通知、草稿本提示、交互提示、CLI 帮助、错误信息

---

## 安全模型

LLM 自主评估每步操作的危险等级。高危险操作时 LLM 应在 reasoning 中表达担忧，并在 cmd 中先调用 `doit prompt "确认文本"` 获取用户许可后再执行。

绝不在代码层面硬编码危险操作规则。

---

## 会话草稿本

每个 session 目录下自动创建 `scratchpad.md`（含预设模板）。LLM 可通过 Shell 命令自由读写。无自动注入——草稿本是普通文件，查看即一条命令。系统提示中说明其用途和位置。

---

## 待讨论

- 流式交互 TUI 具体设计
- LLM 后端 trait 精确接口
- 子命令详细实现规格
- 错误码体系
- 测试策略
