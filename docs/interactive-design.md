# Interactive 模式设计报告

## 架构概览

```
用户终端 (raw mode, no echo)
    │
    ├─ stdin (逐字节) ──→ [agent loop] ──→ PTY master writer ──→ [子进程] stdin
    │
    └─ stdout ←── [PTY proxy read thread] ←── PTY master reader ←── [子进程] stdout/stderr
                                                      │
                                                 capture buffer → LLM context
```

## Agent Loop 流程

```
1. 创建 Session（.doit/sessions/<8-char-id>/）
2. 生成 System Prompt（调用 doit template system --interactive）
3. 进入循环:

loop {
    ┌─ 3a. 构建 API messages（Block → ChatMessage）
    ├─ 3b. 调用 DeepSeek API
    │     model: deepseek-v4-pro
    │     thinking: enabled
    │     无 tool_choice（强 prompt 引导工具调用）
    │
    ├─ 3c. 解析响应:
    │     ├─ 有 tool_calls → 提取 cmd + reasoning
    │     └─ 无 tool_calls → is_prompt=true，content 转为 prompt 内容
    │
    ├─ 3d. 显示命令（println! 到终端）:
    │     - is_prompt:  heredoc 格式显示
    │     - tool_call:  "$ <cmd>" 格式
    │
    ├─ 3e. 追加 Assistant Block
    │
    ├─ 3f. 检查 doit exit → 退出循环
    │
    ├─ 3g. 执行命令:
    │     - doit prompt → 直接构造 CommandBuilder，参数传 content
    │     - doit 子命令 → 解析参数，直接构造 CommandBuilder
    │     - 外部命令   → sh -c 执行
    │
    └─ 3h. 追加结果 Block:
          - is_prompt → User Block
          - tool_call → Tool Block
}
```

## PTY 代理机制

### 组件

| 组件 | 职责 |
|------|------|
| PTY slave | 子进程 stdin/stdout/stderr 终端 |
| PTY master reader (读线程) | 读取子进程输出 → 写终端 + 写 capture buffer |
| PTY master writer (主线程) | 读 stdin → 写入 PTY master（传给子进程） |
| Self-pipe (signal pipe) | 读线程 EOF 时唤醒主线程 |
| capture buffer (Arc<Mutex<Vec<u8>>>) | 保存完整 PTY 输出 → LLM 上下文 |

### 数据流

```
                    ┌──────────────────┐
  stdin ──(poll)──→│  Main Thread     │──→ PTY master writer
                    │  poll(stdin, sig)│
                    └──────────────────┘

                    ┌──────────────────┐
  PTY master reader →│ Read Thread     │──→ stdout (用户终端)
  (EOF → signal pipe)│                  │──→ capture buffer (LLM)
                    └──────────────────┘
```

### 退出机制

- 子进程退出 → PTY slave 关闭 → master reader 返回 EOF
- 读线程写 1 字节到 signal pipe
- 主线程 poll() 检测到 signal pipe 可读 → 退出循环
- Ctrl+C: `Interrupted` 错误 → break

## 终端 Raw Mode

PTY 执行期间将终端设为 raw mode:

```
tcgetattr(stdin) → 保存 original
c_lflag &= ~(ECHO | ICANON)   // 关闭回显和行缓冲
VMIN = 1, VTIME = 0            // 逐字节读取
tcsetattr(TCSANOW, raw)
// ... PTY 执行 ...
tcsetattr(TCSANOW, original)   // 恢复
```

作用：终端不本地回显输入，所有显示通过 PTY 代理统一管理。

## 命令执行路径

### 1. is_prompt（LLM 返回 content 而非 tool_calls）

```rust
pty_exec(&["doit", "prompt", content])
```
- 直接通过 CommandBuilder 传参，不经过 shell
- 避免 LLM 多行内容中的特殊字符被 shell 误解析

### 2. doit 子命令（`cmd.starts_with("doit ")`）

```rust
pty_exec_direct(prog, args)
```
- 解析命令字符串为参数数组
- 通过 CommandBuilder 直接执行

### 3. 外部命令

```rust
pty_exec_shell("sh -c 'cmd'")
```
- 保留管道、重定向等 shell 特性

## Block 结构

```rust
enum Block {
    System { seq, content }                          // system prompt
    User { seq, content }                            // user input (is_prompt 结果)
    Assistant { seq, reasoning, cmd, tool_call_id?, content? }  // LLM 响应
    Tool { seq, output, exit_code, tool_call_id }    // 命令执行结果
}
```

### is_prompt 时的 Block 配对

```
Assistant { content: Some("LLM 文本"), tool_call_id: None, cmd: "doit prompt" }
User { content: "> 用户输入内容" }
```

API 重建时：Assistant.message.content = "LLM文本", tool_calls = None, User = role:user。

### tool_call 时的 Block 配对

```
Assistant { content: None, tool_call_id: Some("call_1"), cmd: "ls /" }
Tool { output: "...", tool_call_id: "call_1" }
```

API 重建时：Assistant.message.tool_calls = [...], Tool = role:tool。

## API 消息重建 (render.rs)

```
System → role: "system"
User   → role: "user"
Assistant:
  - tool_call_id 存在 → tool_calls (name:"sh", arguments:{command:...})
  - tool_call_id 为 None，content 存在 → content 文本
  - reasoning_content 始终携带
Tool:
  - output 剥离 ANSI → content
  - tool_call_id 匹配
```

## API 调用参数

```
model: deepseek-v4-pro
thinking: {type: "enabled"}
无 tool_choice
tools: [{type: "function", function: {name: "sh", ...}}]
```

## Session 管理

```
.doit/sessions/<8-char-id>/
├── conversation.jsonl   ← 追加写入，每次 append
├── scratchpad.md        ← 创建时生成 TODO 模板
└── logs/                ← 预留（截断溢出）
```

## 当前代码结构

```
src/agent/mod.rs          ← Agent struct + run_loop + PTY 代理
src/backend/deepseek.rs   ← DeepSeek API client
src/block/{mod,jsonl,render}.rs  ← Block 定义/JSONL/API 渲染
src/session/mod.rs        ← Session 创建/加载/追加
src/commands/interactive/mod.rs  ← CLI 入口 → Agent::run_interactive
```

## 已知问题

1. **终端显示不一致**: `println!` 的 heredoc 显示与 PTY 代理输出可能交错（多线程 stdout 竞争）
2. **raw mode 恢复**: 异常退出时可能未恢复终端设置
3. **外部命令的 shell 转义**: 无 do it 前缀的命令通过 `sh -c` 执行，仍存在转义风险
4. **doit prompt 的 `> ` 提示符**: 在 PTY 内部被捕获但终端显示可能不完整
