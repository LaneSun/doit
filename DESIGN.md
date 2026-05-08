# doit — 设计文档

> doit 是一个以命令行为主的 AI Agent，借助 LLM 对 Terminal 操作的深度优势。
> 核心理念：**一切皆命令**。Agent 的工具箱被最小化为 Shell 原语 + 少量专用子命令。

---

## 1. CLI 接口

```
doit                          交互式 REPL 模式（流式 TUI）
doit init                     交互式初始化（输入 API Key）
doit run <任务描述>            单次执行，逐 Turn 展示，最终输出总结到 stdout
doit task <任务描述>           子 Agent / 非交互模式，仅输出 exit 传入的文本
doit resume <会话ID>          恢复交互式会话
doit prompt <消息>             阻塞等待用户输入
doit template <类型>          生成提示/模板文本
doit exec [参数] -- <命令>     Shell 执行包装（截断、日志）
```

---

## 2. 对话协议

整个对话上下文是一个格式化的 Shell 会话日志，使用类 XML 标签：

- `<cmd>$ ...</cmd>` — 待执行的命令
- `<output>...</output>` — 命令输出（stdout 与 stderr 合并）
- 同一 assistant 块内的多个 `<cmd>` 视为 `&&` 串联执行（不推荐，仅容错）
- `<cmd>` 之外的裸文本允许但不鼓励

### Turn 结构

```
==== system ====
{template system 的输出 — 纯文本，无标签}

==== assistant ====
<cmd>$ doit prompt '{template ask 的文本}'</cmd>    ← 交互模式下自动生成

==== user ====
<output>
[QUERY] {消息}
[INPUT] {用户输入}
</output>

==== user ====
<output>{草稿本命令输出}</output>

==== assistant ====
<cmd>$ ls</cmd>
<cmd>$ find . -name "*.rs"</cmd>   ← && 串联

==== user ====
<output>{命令结果}</output>

==== user ====
<output>{草稿本命令输出}</output>

==== assistant ====
{完成说明}
<cmd>$ doit exit "重构成功"</cmd>
```

- **交互模式**下，首个 assistant 块由系统自动生成（`prompt` 命令）。
- **非交互模式**下，首个 assistant 块由 LLM 给出。任务描述追加到 system 块内容尾部。

---

## 3. 内置子命令

### 核心

| 命令 | 说明 |
|------|------|
| `doit exec [--no-truncate] [--truncate-chars N] [--truncate-lines N] -- <命令>` | Shell 执行包装器。LLM 的所有外部命令默认被此命令隐式包裹。 |
| `doit prompt <消息>` | 阻塞等待用户输入。返回用户文本作为 output。空输入 → `<ENTER>` 标记。 |
| `doit exit [总结]` | 标记任务完成并退出。 |
| `doit task <描述>` | 启动子 Agent（非交互）。 |
| `doit template <类型>` | 生成模板/提示文本。 |

### 文件操作

| 命令 | 说明 |
|------|------|
| `doit glob <模式> [--cwd <目录>]` | 文件模式匹配，返回相对路径列表。 |
| `doit read <文件> [--lines N:M]` | 精准读取文件片段，带行号。 |
| `doit write <文件> [--append] [--mode 0o644]` | 原子文件写入（heredoc/stdin），先写临时文件再 rename。 |
| `doit search <模式> [--include "*.rs"] ...` | 内容搜索，带行号。 |
| `doit edit <文件> <模式> ...` | 结构化文件编辑（见下文）。自动以 `search` 格式输出替换结果。 |

### edit 三种模式

```
# 按行号替换
doit edit <文件> --lines N:M <<'EOF'
...
EOF

# 正则替换（词级，不需要上下文）
doit edit <文件> --regex "旧内容" --replace "新内容"

# Git diff 格式（多行结构化变更，前后至少 3 行上下文，固定使用 heredoc）
doit edit <文件> <<'DIFF'
@@ -10,6 +10,8 @@
 上下文行
-旧行
+新行
+新增行
 更多上下文
DIFF
```

子 Agent 模式（`doit task`）中，`doit prompt` 不出现在可用命令集中——LLM 根本看不到它。

---

## 4. 会话存储

每个会话以独立目录存储，包含一个 JSONL 文件：

```
.doit/sessions/<8字符ID>/
├── conversation.jsonl      # 逐 Block 的对话日志
├── scratchpad.md            # LLM 可编辑的草稿本文件
└── logs/                    # 截断溢出的完整日志
    ├── a1b2c3d4.log
    └── e5f6g7h8.log
```

### JSONL Schema

```jsonl
{"type":"meta","id":"a1b2c3d4","model":"deepseek-v4-pro","cwd":"/home/user/project","created_at":1700000000}
{"seq":1,"role":"system","type":"content","raw":"...角色定义（纯文本）...","content":"...同上..."}
{"seq":2,"role":"assistant","type":"content","raw":"<cmd>$ doit prompt '...'</cmd>","content":"<cmd>$ doit prompt '...'</cmd>"}
{"seq":3,"role":"user","type":"content","raw":"<output>...</output>","content":"<output>...</output>"}
{"seq":4,"role":"user","type":"content","raw":"...草稿本输出...","content":"...草稿本输出..."}
{"seq":5,"role":"assistant","type":"content","raw":"<cmd>$ ls</cmd>","content":"<cmd>$ ls</cmd>"}
{"seq":6,"role":"user","type":"content","raw":"total 12\ndrwxr-xr-x ...","content":"total 12\ndrwxr-xr-x ..."}
```

- `seq`：全局递增序号，保证顺序不变
- `raw`：原始输出（保留 ANSI 格式码，截断后），用于终端回放
- `content`：清洗后纯文本（ANSI 已剥离，截断后），发给 LLM
- `type`：`content`（`thought` 等类型预留给未来思维链）
- Block 以 **Turn（角色轮次）** 为单位，非 `<cmd>`/`<output>` 为单位

---

## 5. 上下文渲染

发送给 LLM 时，从内存中的 Block 列表按 `seq` 顺序渲染：

- 按 role 分组 → 添加 `==== {role} ====\n{content}` 头
- 使用 `content` 字段（ANSI 已剥离）发给 LLM
- `raw` 用于恢复时的终端回放

---

## 6. 执行与截断

### doit exec

所有**非 `doit` 内置**的外部 Shell 命令被 `doit exec` 隐式包裹。`doit` 子命令**不**被包裹——它们自行处理截断（如需要）。

```
doit exec [--no-truncate] [--truncate-chars N] [--truncate-lines N] -- <shell 命令>
```

- 合并捕获 stdout + stderr
- 超出限制时进行头尾截断
- 完整输出写入会话 `logs/` 目录下的随机命名 `.log` 文件
- ANSI 感知切割（不切断转义序列；在边界插入 SGR 重置/恢复）

### 隐式包裹范围

| 命令类型 | 被 `doit exec` 包裹？ |
|---------|:-------------------:|
| 外部命令（`ls`、`find`、`cat`、`npm` 等） | 是（透明） |
| `doit` 子命令（`doit read`、`doit glob` 等） | 否（自身处理） |

可能产生大量输出的子命令（`doit read`、`doit search`、`doit glob`）需自行实现相同机制的截断。输出量有限的子命令（`doit edit`、`doit exit`、`doit prompt`）无需截断。

### 截断规则

- **限制**：头尾各 2000 字符 / 50 行，OR 触发（取先到达者）
- **方式**：头 2000/50 + 截断提示 + 尾 2000/50
- **截断提示**（附加在输出末尾）：
  ```
  ... [截断: 4523 行 / 34200 字符已省略, 完整输出见 .doit/sessions/<id>/logs/<logfile>.log]
  ```
- 用户不可配置（约定优先于配置）。LLM 可使用 `doit exec --no-truncate` 关闭截断。

### 交互式截断显示

命令实时执行时：
1. 达到截断阈值前：正常滚动（与裸 Shell 完全一致）
2. 触发截断时：头部冻结在 scrollback 中，中间省略行实时更新计数（`\r` 重绘），尾部为固定高度 viewport（ring buffer，整体重绘）
3. 最终状态 = LLM 所见完全一致

---

## 7. 草稿本

一种持久化的 LLM 上下文机制：

- 文件：`{会话目录}/scratchpad.md`（会话创建时含模板初始化）
- 每个 user 块（命令输出）之后，自动追加一个 user 块，内容为草稿本命令的输出（通过 `template scratchpad` 配置）
- LLM 可用标准 Shell 命令读写 scratchpad.md
- 默认模板命令：`cat scratchpad.md && echo '以上是草稿内容...'`
- 草稿本 Block 和其他 user block 一样存入 JSONL

---

## 8. 交互式干预（Ctrl+C）

| 操作 | 结果 |
|------|------|
| 命令执行中 **Ctrl+C** | 发送 SIGINT；将 `^C` 注入上下文；自动追加 `<cmd>$ doit prompt '命令被中断，请输入新指示，或 Ctrl+C 退出'</cmd>` |
| 输入文本 | 作为 `<output>` 捕获，发给 LLM，Agent 继续 |
| 输入回车（空） | `<ENTER>` 标记捕获，发给 LLM |
| prompt 等待中 **Ctrl+C** | 退出程序；**不记录**最后一个 output 块（干净的恢复点） |

---

## 9. 子 Agent 机制

- `doit task <描述>` — 启动非交互式子 Agent
- 子 Agent 的 CWD = 调用时刻的当前 Shell CWD（无特殊处理）
- 独立会话目录：`.doit/sessions/<父ID>/<子ID>/`（或独立树）
- 子 Agent 仅返回 `doit exit` 传入的文本
- 任务描述和结果建议通过 heredoc 传递
- 子 Agent **不能**使用 `doit prompt`（该命令不在可用范围内）
- 支持并发（Shell `&`）

---

## 10. 会话恢复

1. 加载 `conversation.jsonl`
2. 校验最后一行完整性；丢弃不完整行并警告
3. 清屏
4. 按 `seq` 顺序回放所有 Block（`raw` 字段显示到终端，包含所有角色包括 system）
5. 如果最后一个完整 Block 是 `doit prompt` 且无对应 output：
   - 使用相同消息重新执行 `doit prompt`
   - 用户回到等待状态
6. 继续 Agent 循环

---

## 11. LLM 后端

### 抽象

- 基于 Trait 的后端抽象
- OpenAI 兼容 API 格式（chat completions）
- 使用 `async-openai` crate 进行 HTTP 传输
- 无状态：每次 `send` 携带完整消息列表
- 默认模型：`deepseek-v4-pro`

### API 消息映射

- 每个 Block 的 `content`（附带 role）→ API 消息（`system` / `user` / `assistant`）
- `==== role ====` 标记**不**发送到 API，仅存在于内部格式
- 系统提示作为 API role `system` 发送

### 流式响应

后续处理（与思维链交互）。

---

## 12. 模板系统

```
doit template system       # 角色定义 + 协议规则 + 子命令 --help（纯文本）
doit template ask          # 首次请求用户输入时的 prompt 消息文本
doit template scratchpad   # 读取并显示草稿本的命令
```

- 所有模板内置 + i18n，可通过配置覆盖
- `template system` 输出 → system block 的 content（纯文本，无标签）
- `template ask` 输出 → 用作 `doit prompt` 的消息参数（放入 assistant block）
- `template scratchpad` 输出 → 命令字符串，被 shell 执行后其输出放入尾部 user block

### 模式差异

| 模板 | 交互模式 | 非交互 (`doit task`) |
|------|:------:|:------------------:|
| `system` | 核心规则 + 全部子命令 --help | 核心规则 + 仅非阻塞子命令 |
| `ask` | 包含（自动生成的 assistant block） | 不使用 |
| `scratchpad` | 包含 | 包含 |

---

## 13. 系统提示内容

`template system` 输出涵盖：

1. **角色定义** — Agent 身份与能力描述
2. **协议说明** — `<cmd>/<output>` 格式、`&&` 语义
3. **执行规则** — `doit exec` 隐式包裹、截断机制、日志位置
4. **安全规则** — 自我评估危险性；使用 `doit prompt` 文本确认；不引入专用确认命令
5. **草稿本说明** — 用途、位置、编辑方法
6. **动态工具参考** — 所有可用内置子命令的 `--help` 输出

---

## 14. 配置系统

### 优先级（从高到低）

1. CLI 参数（`--model`、`--no-truncate` 等）
2. 环境变量（`DOIT_API__BASE_URL` → `api.base_url`）
3. `--config <路径>` 指定文件
4. `./doit.toml`（项目根目录）
5. `~/.config/doit/config.toml`（用户全局）
6. 内置默认值

### 合并策略

深层 merge：每个 section 独立合并，叶子值覆盖。

### 可配置项

```toml
[api]
base_url = "https://api.deepseek.com"
model = "deepseek-v4-pro"
api_key = "${DOIT_API_KEY}"       # 环境变量引用
temperature = 0.7
max_tokens = 8192

[output]
truncate_chars = 2000              # 头尾各
truncate_lines = 50                # 头尾各

[locale]
lang = "zh-CN"

[system_prompt]                    # 可选覆盖
# template = "..."

[templates]                        # 可选模板覆盖
# system = "..."
# ask = "..."
# scratchpad = "..."
```

---

## 15. 国际化（i18n）

- Crate：`rust-i18n`（编译期嵌入）
- 语言：`en`、`zh-CN`
- 翻译文件：`locales/{lang}.toml`（crate 根目录）
- 范围：系统提示、截断通知、草稿本提示、交互提示、CLI 帮助、错误信息

---

## 16. 错误处理

- Crate：`miette`（丰富诊断输出）
- 所有应用错误为 `miette::Report`
- 模块级错误类型使用 `thiserror` 定义结构化错误

---

## 17. 技术栈汇总

| 关注点 | Crate |
|--------|-------|
| CLI | `clap` (derive) |
| 异步运行时 | `tokio` (full) |
| 配置 | `serde` + `toml` |
| HTTP | `async-openai` |
| 错误处理 | `miette` |
| 国际化 | `rust-i18n` |
| 日志 | `tracing` + `tracing-subscriber` |
| XDG 路径 | `directories` |
| 时间戳 | `chrono` |
| 会话 ID | `uuid` (v4) |
| ANSI 处理 | `strip-ansi-escapes` |

---

## 18. 实现要点

- 初始化向导：简单的交互式 API Key 输入（`doit init`）
- `doit edit` 正则替换和 diff 格式匹配由开发者自行设计实现

## 19. 未来讨论

- 思维链（流式响应 + thought block）
