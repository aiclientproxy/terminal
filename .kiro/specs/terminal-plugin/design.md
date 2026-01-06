# 设计文档

## 概述

ProxyCast Terminal Plugin 是一个功能完整的终端插件，采用双层架构设计：Rust 后端负责 PTY/SSH 管理，React 前端负责终端渲染。前后端通过 JSON-RPC 协议进行通信，后端作为独立进程运行，通过 stdin/stdout 与前端交换数据。

### 技术栈

**后端（Rust）**
- portable-pty - PTY 管理
- russh - SSH 客户端
- tokio - 异步运行时
- serde/serde_json - 序列化

**前端（React + TypeScript）**
- xterm.js - 终端渲染
- React 18 - UI 框架
- Vite - 构建工具
- TailwindCSS - 样式

## 架构

### 整体架构图

```
┌────────────────────────────────────────────────────────────┐
│                    ProxyCast 主应用                         │
│  ┌──────────────────────────────────────────────────────┐  │
│  │           插件宿主环境（Plugin Host）                  │  │
│  │  ┌────────────────────────────────────────────────┐  │  │
│  │  │  React UI 前端 (IIFE Bundle)                   │  │  │
│  │  │  ┌──────────────┐  ┌────────────────────────┐  │  │  │
│  │  │  │ TerminalView │  │ xterm.js + Addons      │  │  │  │
│  │  │  │  - Tabs      │  │  - FitAddon            │  │  │  │
│  │  │  │  - Toolbar   │  │  - SearchAddon         │  │  │  │
│  │  │  │  - Dialogs   │  │  - WebLinksAddon       │  │  │  │
│  │  │  └──────┬───────┘  └────────────────────────┘  │  │  │
│  │  │         │                                       │  │  │
│  │  │         │ RPC Client (JSON-RPC over stdio)     │  │  │
│  │  └─────────┼───────────────────────────────────────┘  │  │
│  └────────────┼──────────────────────────────────────────┘  │
└───────────────┼─────────────────────────────────────────────┘
                │
                ▼
┌────────────────────────────────────────────────────────────┐
│           Rust CLI 后端 (独立进程)                          │
│  ┌──────────────────────────────────────────────────────┐  │
│  │  JSON-RPC Server (stdin/stdout)                      │  │
│  └────────┬────────────────────────┬────────────────────┘  │
│           │                        │                       │
│  ┌────────▼────────┐      ┌────────▼──────────┐           │
│  │  PTY Manager    │      │  SSH Manager      │           │
│  │  - Sessions     │      │  - Connections    │           │
│  │  - Shell Detect │      │  - Authentication │           │
│  │  - OSC Handler  │      │  - PTY Channels   │           │
│  └────────┬────────┘      └────────┬──────────┘           │
└───────────┼──────────────────────────┼──────────────────────┘
            │                          │
            ▼                          ▼
┌───────────────────────────────────────────────────────────┐
│                      操作系统层                            │
│  ┌──────────────┐  ┌──────────────┐  ┌─────────────────┐ │
│  │  本地 PTY     │  │  SSH 连接    │  │  WSL (可选)      │ │
│  │  (pty crate) │  │  (russh)     │  │                 │ │
│  └──────────────┘  └──────────────┘  └─────────────────┘ │
└───────────────────────────────────────────────────────────┘
```

### 数据流

```
用户输入流:
用户按键 → xterm.js onData → TermWrap.handleTermData() 
→ JSON-RPC: session.input → Rust 后端 → PTY stdin → Shell 进程

终端输出流:
Shell 进程 → PTY stdout → Rust 后端读取 
→ JSON-RPC Notification: terminal.output → RpcClient 
→ TermWrap.handleOutput() → terminal.write() → xterm.js 渲染
```

## 组件和接口

### JSON-RPC 方法

| 方法 | 描述 | 参数 | 返回值 |
|------|------|------|--------|
| `session.create` | 创建新会话 | `CreateSessionRequest` | `CreateSessionResponse` |
| `session.input` | 发送输入数据 | `InputRequest` | `null` |
| `session.resize` | 调整终端大小 | `ResizeRequest` | `null` |
| `session.close` | 关闭会话 | `{ session_id: string }` | `null` |
| `session.list` | 列出所有会话 | - | `SessionInfo[]` |
| `session.get` | 获取会话信息 | `{ session_id: string }` | `SessionInfo` |

### JSON-RPC 通知

| 通知 | 描述 | 参数 |
|------|------|------|
| `terminal.output` | 终端输出数据 | `OutputNotification` |
| `session.status` | 会话状态变更 | `SessionStatusNotification` |
| `session.title` | 会话标题变更 | `{ session_id: string, title: string }` |
| `session.cwd` | 工作目录变更 | `{ session_id: string, cwd: string }` |

### Rust 后端组件

```
src-tauri/src/
├── main.rs                  # 入口点
├── lib.rs                   # 库导出
├── rpc/                     # JSON-RPC 层
│   ├── mod.rs
│   ├── server.rs            # RPC 服务器
│   ├── methods.rs           # RPC 方法注册
│   └── types.rs             # RPC 数据类型
├── pty/                     # PTY 管理
│   ├── mod.rs
│   ├── manager.rs           # PTY 会话管理器
│   ├── local.rs             # 本地 PTY 实现
│   └── session.rs           # 会话抽象
├── ssh/                     # SSH 连接
│   ├── mod.rs
│   ├── client.rs            # SSH 客户端
│   ├── session.rs           # SSH 会话管理
│   └── auth.rs              # SSH 认证
├── shell/                   # Shell 集成
│   ├── mod.rs
│   ├── detect.rs            # Shell 类型检测
│   └── osc.rs               # OSC 序列处理
└── utils/                   # 工具模块
    ├── mod.rs
    └── error.rs             # 错误类型
```

### React 前端组件

```
src/
├── index.tsx                # 入口点
├── App.tsx                  # 主应用组件
├── types/                   # TypeScript 类型
│   ├── index.ts
│   ├── rpc.ts               # RPC 类型定义
│   └── terminal.ts          # 终端类型
├── components/              # UI 组件
│   ├── Terminal/
│   │   ├── index.tsx        # 终端主组件
│   │   ├── TerminalView.tsx # 终端视图
│   │   └── TerminalTabs.tsx # 标签页管理
│   ├── Search/
│   │   └── SearchBar.tsx    # 搜索组件
│   └── Dialogs/
│       └── NewConnectionDialog.tsx
├── lib/                     # 工具库
│   ├── termwrap.ts          # xterm.js 包装器
│   ├── rpc-client.ts        # JSON-RPC 客户端
│   └── theme.ts             # 终端主题
├── store/                   # 状态管理
│   └── session-store.ts     # 会话状态
└── hooks/                   # React Hooks
    ├── useTerminal.ts       # 终端 Hook
    └── useRpc.ts            # RPC Hook
```

## 数据模型

### 连接类型

```typescript
type ConnectionType =
  | {
      type: 'local';
      shell_path?: string;
      cwd?: string;
      env?: Record<string, string>;
    }
  | {
      type: 'ssh';
      host: string;
      port?: number;
      user?: string;
      identity_file?: string;
      password?: string;
    };
```

### 终端尺寸

```typescript
interface TermSize {
  rows: number;
  cols: number;
}
```

### 会话状态

```typescript
type SessionStatus = 'init' | 'connecting' | 'running' | 'done' | 'error';
```

### 会话信息

```typescript
interface SessionInfo {
  id: string;
  connection_type: ConnectionType;
  status: SessionStatus;
  title?: string;
  cwd?: string;
  exit_code?: number;
  created_at: number;
}
```

### RPC 请求/响应类型

```typescript
// 创建会话请求
interface CreateSessionRequest {
  connection: ConnectionType;
  term_size: TermSize;
}

// 创建会话响应
interface CreateSessionResponse {
  session_id: string;
}

// 输入请求
interface InputRequest {
  session_id: string;
  data: string; // base64 encoded
}

// 调整大小请求
interface ResizeRequest {
  session_id: string;
  term_size: TermSize;
}

// 输出通知
interface OutputNotification {
  session_id: string;
  data: string; // base64 encoded
}

// 状态通知
interface SessionStatusNotification {
  session_id: string;
  status: SessionStatus;
  exit_code?: number;
}
```

### Rust 数据结构

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TermSize {
    pub rows: u16,
    pub cols: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ConnectionType {
    Local {
        shell_path: Option<String>,
        cwd: Option<String>,
        env: Option<HashMap<String, String>>,
    },
    Ssh {
        host: String,
        port: Option<u16>,
        user: Option<String>,
        identity_file: Option<String>,
        password: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SessionStatus {
    Init,
    Connecting,
    Running,
    Done,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    pub id: String,
    pub connection_type: ConnectionType,
    pub status: SessionStatus,
    pub title: Option<String>,
    pub cwd: Option<String>,
    pub exit_code: Option<i32>,
    pub created_at: u64,
}
```

## 正确性属性

*正确性属性是系统在所有有效执行中应保持为真的特征或行为——本质上是关于系统应该做什么的形式化陈述。属性作为人类可读规范和机器可验证正确性保证之间的桥梁。*

### 属性 1: 会话 ID 唯一性

*对于任意*多个创建的会话，所有返回的会话 ID 都应该是唯一的，不存在重复。

**验证: 需求 1.2**

### 属性 2: RPC 请求往返一致性

*对于任意*有效的 RPC 请求对象，将其序列化为 JSON 字符串后再反序列化，应该产生与原始对象等价的对象。

**验证: 需求 3.6**

### 属性 3: RPC 错误响应格式

*对于任意*无效的 JSON-RPC 请求，服务器返回的错误响应应该符合 JSON-RPC 2.0 规范，包含 error 字段和正确的错误码。

**验证: 需求 3.5**

### 属性 4: OSC 序列处理健壮性

*对于任意* OSC 序列输入（包括有效和无效的），OSC 处理器应该：
- 对于有效的 OSC 7 序列，正确解析出目录路径
- 对于有效的 OSC 52 序列，正确解码 base64 内容
- 对于无效序列，安全地忽略而不崩溃

**验证: 需求 7.1, 7.2, 7.3**

### 属性 5: PTY 配置传递

*对于任意*指定的工作目录和环境变量配置，PTY 管理器创建的 shell 进程应该：
- 在指定的工作目录中启动
- 包含所有指定的环境变量

**验证: 需求 8.3, 8.4**

### 属性 6: 错误状态一致性

*对于任意*会话遇到的错误，系统应该：
- 将会话状态更新为 'error'
- 记录错误信息
- 继续处理其他会话而不崩溃

**验证: 需求 10.4, 10.5**

## 错误处理

### 错误类型定义

```rust
#[derive(Debug, thiserror::Error)]
pub enum TerminalError {
    #[error("PTY 创建失败: {0}")]
    PtyCreationFailed(String),
    
    #[error("SSH 连接失败: {0}")]
    SshConnectionFailed(String),
    
    #[error("会话不存在: {0}")]
    SessionNotFound(String),
    
    #[error("无效的请求: {0}")]
    InvalidRequest(String),
    
    #[error("IO 错误: {0}")]
    IoError(#[from] std::io::Error),
    
    #[error("序列化错误: {0}")]
    SerializationError(#[from] serde_json::Error),
}
```

### 错误处理策略

1. **PTY 错误**: 返回描述性错误消息，更新会话状态为 'error'
2. **SSH 错误**: 返回连接错误详情，包括主机、端口和认证方式
3. **RPC 错误**: 返回 JSON-RPC 2.0 标准错误响应
4. **意外错误**: 记录日志，继续运行，不影响其他会话

## 测试策略

### 双重测试方法

本项目采用单元测试和属性测试相结合的方式：

- **单元测试**: 验证特定示例、边界情况和错误条件
- **属性测试**: 验证跨所有输入的通用属性

### 属性测试配置

- 使用 `proptest` 库进行 Rust 属性测试
- 使用 `fast-check` 库进行 TypeScript 属性测试
- 每个属性测试最少运行 100 次迭代
- 每个测试必须引用设计文档中的属性编号

### 测试标签格式

```
Feature: terminal-plugin, Property {number}: {property_text}
```

### 测试覆盖范围

| 组件 | 单元测试 | 属性测试 |
|------|----------|----------|
| RPC 类型序列化 | ✓ | ✓ (属性 2) |
| RPC 错误处理 | ✓ | ✓ (属性 3) |
| 会话 ID 生成 | ✓ | ✓ (属性 1) |
| OSC 解析器 | ✓ | ✓ (属性 4) |
| PTY 配置 | ✓ | ✓ (属性 5) |
| 错误状态管理 | ✓ | ✓ (属性 6) |
| Shell 检测 | ✓ | - |
| 前端组件 | ✓ | - |
