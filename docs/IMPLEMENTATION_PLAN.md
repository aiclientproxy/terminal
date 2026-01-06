# ProxyCast Terminal Plugin 完整实现计划

## 目录

- [项目概述](#项目概述)
- [架构设计](#架构设计)
- [项目结构](#项目结构)
- [核心接口定义](#核心接口定义)
- [Rust 后端实现](#rust-后端实现)
- [React 前端实现](#react-前端实现)
- [插件配置](#插件配置)
- [实现步骤](#实现步骤)
- [依赖清单](#依赖清单)

---

## 项目概述

将 Wave Terminal 的完整终端功能作为插件移植到 ProxyCast，采用与 kiro-provider 相同的插件架构。

### 核心功能

- ✅ 本地 PTY 终端支持
- ✅ SSH 远程连接
- ✅ 多标签页管理
- ✅ 终端搜索功能
- ✅ Shell 集成（OSC 序列）
- ✅ 剪贴板支持（OSC 52）
- ✅ 工作目录追踪（OSC 7）
- ✅ 主题自定义

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

---

## 架构设计

### 整体架构

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
┌─────────────────────────────────────────────────────────┐
│                     用户输入流                           │
└─────────────────────────────────────────────────────────┘
用户按键
    ↓
xterm.js onData
    ↓
TermWrap.handleTermData()
    ↓
JSON-RPC: session.input { session_id, data(base64) }
    ↓
Rust 后端接收
    ↓
PtyManager/SshManager 写入 PTY stdin
    ↓
Shell 进程

┌─────────────────────────────────────────────────────────┐
│                     终端输出流                           │
└─────────────────────────────────────────────────────────┘
Shell 进程输出
    ↓
PTY stdout
    ↓
Rust 后端读取
    ↓
JSON-RPC Notification: terminal.output { session_id, data(base64) }
    ↓
RpcClient 接收通知
    ↓
TermWrap.handleOutput()
    ↓
terminal.write()
    ↓
xterm.js 渲染
```

---

## 项目结构

```
terminal/
├── docs/
│   ├── IMPLEMENTATION_PLAN.md       # 本文档
│   ├── API.md                       # API 接口文档
│   └── ARCHITECTURE.md              # 架构详解
│
├── plugin.json                      # 插件元数据
├── config.json                      # 运行时配置
├── README.md                        # 项目说明
├── .gitignore
│
├── src-tauri/                       # Rust 后端
│   ├── Cargo.toml
│   ├── Cargo.lock
│   ├── build.rs                     # 构建脚本
│   └── src/
│       ├── main.rs                  # 入口点
│       ├── lib.rs                   # 库导出
│       │
│       ├── rpc/                     # JSON-RPC 层
│       │   ├── mod.rs
│       │   ├── server.rs            # RPC 服务器
│       │   ├── methods.rs           # RPC 方法注册
│       │   └── types.rs             # RPC 数据类型
│       │
│       ├── pty/                     # PTY 管理
│       │   ├── mod.rs
│       │   ├── manager.rs           # PTY 会话管理器
│       │   ├── local.rs             # 本地 PTY 实现
│       │   ├── session.rs           # 会话抽象
│       │   └── traits.rs            # PTY 接口定义
│       │
│       ├── ssh/                     # SSH 连接
│       │   ├── mod.rs
│       │   ├── client.rs            # SSH 客户端
│       │   ├── session.rs           # SSH 会话管理
│       │   ├── auth.rs              # SSH 认证
│       │   └── config.rs            # SSH 配置解析
│       │
│       ├── shell/                   # Shell 集成
│       │   ├── mod.rs
│       │   ├── detect.rs            # Shell 类型检测
│       │   ├── integration.rs       # Shell 集成文件
│       │   └── osc.rs               # OSC 序列处理
│       │
│       └── utils/                   # 工具模块
│           ├── mod.rs
│           ├── env.rs               # 环境变量
│           └── error.rs             # 错误类型
│
├── src/                             # React 前端
│   ├── package.json
│   ├── package-lock.json
│   ├── tsconfig.json
│   ├── vite.config.ts               # Vite 配置（IIFE 输出）
│   ├── postcss.config.js
│   ├── tailwind.config.js
│   │
│   └── src/
│       ├── index.tsx                # 入口点
│       ├── App.tsx                  # 主应用组件
│       ├── index.css                # 全局样式
│       │
│       ├── types/                   # TypeScript 类型
│       │   ├── index.ts
│       │   ├── rpc.ts               # RPC 类型定义
│       │   ├── terminal.ts          # 终端类型
│       │   └── plugin.d.ts          # ProxyCast 插件类型
│       │
│       ├── components/              # UI 组件
│       │   ├── Terminal/
│       │   │   ├── index.tsx        # 终端主组件
│       │   │   ├── TerminalView.tsx # 终端视图
│       │   │   ├── TerminalTabs.tsx # 标签页管理
│       │   │   ├── TerminalToolbar.tsx
│       │   │   └── styles.css
│       │   │
│       │   ├── Search/
│       │   │   ├── index.tsx        # 搜索组件
│       │   │   └── SearchBar.tsx
│       │   │
│       │   └── Dialogs/
│       │       ├── NewConnectionDialog.tsx  # 新建连接
│       │       └── SshConfigDialog.tsx      # SSH 配置
│       │
│       ├── lib/                     # 工具库
│       │   ├── termwrap.ts          # xterm.js 包装器
│       │   ├── fitaddon.ts          # 自定义 FitAddon
│       │   ├── rpc-client.ts        # JSON-RPC 客户端
│       │   ├── theme.ts             # 终端主题
│       │   └── utils.ts             # 工具函数
│       │
│       ├── store/                   # 状态管理
│       │   ├── index.ts
│       │   ├── terminal-store.ts    # 终端状态
│       │   └── session-store.ts     # 会话状态
│       │
│       └── hooks/                   # React Hooks
│           ├── useTerminal.ts       # 终端 Hook
│           ├── useRpc.ts            # RPC Hook
│           ├── useSession.ts        # 会话 Hook
│           └── useSearch.ts         # 搜索 Hook
│
└── plugin/
    └── dist/                        # 构建输出
        ├── backend/                 # Rust 编译产物
        │   ├── terminal-aarch64-apple-darwin
        │   ├── terminal-x86_64-apple-darwin
        │   ├── terminal-x86_64-unknown-linux-gnu
        │   └── terminal-x86_64-pc-windows-msvc.exe
        └── frontend/
            ├── index.js             # IIFE bundle
            └── styles.css
```

---

## 核心接口定义

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

### 数据类型

```typescript
// 连接类型
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

// 终端尺寸
interface TermSize {
  rows: number;
  cols: number;
}

// 会话状态
type SessionStatus = 'init' | 'connecting' | 'running' | 'done' | 'error';

// 会话信息
interface SessionInfo {
  id: string;
  connection_type: ConnectionType;
  status: SessionStatus;
  title?: string;
  cwd?: string;
  exit_code?: number;
  created_at: number;
}

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

---

## Rust 后端实现

### Cargo.toml

```toml
[package]
name = "proxycast-terminal"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "terminal-cli"
path = "src/main.rs"

[dependencies]
# JSON-RPC
jsonrpc-core = "18.0"
jsonrpc-derive = "18.0"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# PTY
portable-pty = "0.8"

# SSH
russh = "0.44"
russh-keys = "0.44"
ssh2-config = "0.2"

# Async runtime
tokio = { version = "1.0", features = ["full"] }
futures = "0.3"

# Utilities
thiserror = "1.0"
anyhow = "1.0"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
base64 = "0.21"
uuid = { version = "1.0", features = ["v4", "serde"] }
dirs = "5.0"
shellexpand = "3.0"

[target.'cfg(windows)'.dependencies]
winapi = { version = "0.3", features = ["processthreadsapi", "handleapi"] }

[target.'cfg(unix)'.dependencies]
whoami = "1.4"

[profile.release]
lto = true
codegen-units = 1
strip = true
opt-level = "z"
```

### 核心代码模板

#### src/main.rs

```rust
use anyhow::Result;
use tracing::info;
use tracing_subscriber;

mod rpc;
mod pty;
mod ssh;
mod shell;
mod utils;

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    info!("ProxyCast Terminal Plugin starting...");

    // 启动 JSON-RPC 服务器
    let mut server = rpc::server::RpcServer::new();
    server.run().await?;

    Ok(())
}
```

#### src/rpc/server.rs

```rust
use anyhow::Result;
use jsonrpc_core::{IoHandler, Params, Value};
use std::io::{BufRead, BufReader, Write};
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, error, info};

use super::types::*;
use crate::pty::manager::PtyManager;
use crate::ssh::manager::SshManager;

pub enum Notification {
    Output(OutputNotification),
    SessionStatus(SessionStatusNotification),
    Title(TitleNotification),
    Cwd(CwdNotification),
}

pub struct RpcServer {
    pty_manager: Arc<PtyManager>,
    ssh_manager: Arc<SshManager>,
    notification_rx: Option<mpsc::Receiver<Notification>>,
}

impl RpcServer {
    pub fn new() -> Self {
        let (notification_tx, notification_rx) = mpsc::channel(1024);

        Self {
            pty_manager: Arc::new(PtyManager::new(notification_tx.clone())),
            ssh_manager: Arc::new(SshManager::new(notification_tx)),
            notification_rx: Some(notification_rx),
        }
    }

    pub async fn run(&mut self) -> Result<()> {
        let mut io = IoHandler::new();
        self.register_methods(&mut io);

        let stdin = std::io::stdin();
        let mut stdout = std::io::stdout();
        let reader = BufReader::new(stdin.lock());

        // 启动通知发送任务
        if let Some(notification_rx) = self.notification_rx.take() {
            tokio::spawn(async move {
                Self::notification_loop(notification_rx).await;
            });
        }

        info!("RPC Server ready, listening on stdin...");

        // 主循环
        for line in reader.lines() {
            let line = match line {
                Ok(l) => l,
                Err(e) => {
                    error!("Failed to read line: {}", e);
                    break;
                }
            };

            if line.is_empty() {
                continue;
            }

            debug!("Received: {}", line);

            if let Some(response) = io.handle_request_sync(&line) {
                writeln!(stdout, "{}", response)?;
                stdout.flush()?;
            }
        }

        Ok(())
    }

    fn register_methods(&self, io: &mut IoHandler) {
        let pty = self.pty_manager.clone();
        let ssh = self.ssh_manager.clone();

        // session.create
        {
            let pty = pty.clone();
            let ssh = ssh.clone();
            io.add_method("session.create", move |params: Params| {
                let pty = pty.clone();
                let ssh = ssh.clone();
                async move {
                    let request: CreateSessionRequest = params.parse()?;

                    let result = match &request.connection {
                        ConnectionType::Local { .. } => {
                            pty.create_session(request).await
                        }
                        ConnectionType::Ssh { .. } => {
                            ssh.create_session(request).await
                        }
                    };

                    match result {
                        Ok(response) => Ok(serde_json::to_value(response)?),
                        Err(e) => Err(jsonrpc_core::Error::invalid_params(e.to_string())),
                    }
                }
            });
        }

        // session.input
        {
            let pty = pty.clone();
            let ssh = ssh.clone();
            io.add_method("session.input", move |params: Params| {
                let pty = pty.clone();
                let ssh = ssh.clone();
                async move {
                    let request: InputRequest = params.parse()?;

                    // 尝试 PTY，失败则尝试 SSH
                    if pty.send_input(request.clone()).await.is_ok() {
                        return Ok(Value::Null);
                    }

                    ssh.send_input(request).await
                        .map(|_| Value::Null)
                        .map_err(|e| jsonrpc_core::Error::invalid_params(e.to_string()))
                }
            });
        }

        // session.resize
        {
            let pty = pty.clone();
            let ssh = ssh.clone();
            io.add_method("session.resize", move |params: Params| {
                let pty = pty.clone();
                let ssh = ssh.clone();
                async move {
                    let request: ResizeRequest = params.parse()?;

                    if pty.resize(request.clone()).await.is_ok() {
                        return Ok(Value::Null);
                    }

                    ssh.resize(request).await
                        .map(|_| Value::Null)
                        .map_err(|e| jsonrpc_core::Error::invalid_params(e.to_string()))
                }
            });
        }

        // session.close
        {
            let pty = pty.clone();
            let ssh = ssh.clone();
            io.add_method("session.close", move |params: Params| {
                let pty = pty.clone();
                let ssh = ssh.clone();
                async move {
                    let session_id: String = params.parse()?;

                    if pty.close_session(&session_id).await.is_ok() {
                        return Ok(Value::Null);
                    }

                    ssh.close_session(&session_id).await
                        .map(|_| Value::Null)
                        .map_err(|e| jsonrpc_core::Error::invalid_params(e.to_string()))
                }
            });
        }

        // session.list
        {
            let pty = pty.clone();
            let ssh = ssh.clone();
            io.add_method("session.list", move |_params: Params| {
                let pty = pty.clone();
                let ssh = ssh.clone();
                async move {
                    let mut sessions = pty.list_sessions().await;
                    sessions.extend(ssh.list_sessions().await);
                    Ok(serde_json::to_value(sessions)?)
                }
            });
        }
    }

    async fn notification_loop(mut rx: mpsc::Receiver<Notification>) {
        let mut stdout = std::io::stdout();

        while let Some(notification) = rx.recv().await {
            let json = match notification {
                Notification::Output(n) => {
                    serde_json::json!({
                        "jsonrpc": "2.0",
                        "method": "terminal.output",
                        "params": n
                    })
                }
                Notification::SessionStatus(n) => {
                    serde_json::json!({
                        "jsonrpc": "2.0",
                        "method": "session.status",
                        "params": n
                    })
                }
                Notification::Title(n) => {
                    serde_json::json!({
                        "jsonrpc": "2.0",
                        "method": "session.title",
                        "params": n
                    })
                }
                Notification::Cwd(n) => {
                    serde_json::json!({
                        "jsonrpc": "2.0",
                        "method": "session.cwd",
                        "params": n
                    })
                }
            };

            if let Err(e) = writeln!(stdout, "{}", json) {
                error!("Failed to write notification: {}", e);
                break;
            }
            if let Err(e) = stdout.flush() {
                error!("Failed to flush stdout: {}", e);
                break;
            }
        }
    }
}
```

#### src/rpc/types.rs

```rust
use serde::{Deserialize, Serialize};

pub type SessionId = String;

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
        env: Option<std::collections::HashMap<String, String>>,
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
    pub id: SessionId,
    pub connection_type: ConnectionType,
    pub status: SessionStatus,
    pub title: Option<String>,
    pub cwd: Option<String>,
    pub exit_code: Option<i32>,
    pub created_at: u64,
}

#[derive(Debug, Deserialize)]
pub struct CreateSessionRequest {
    pub connection: ConnectionType,
    pub term_size: TermSize,
}

#[derive(Debug, Serialize)]
pub struct CreateSessionResponse {
    pub session_id: SessionId,
}

#[derive(Debug, Clone, Deserialize)]
pub struct InputRequest {
    pub session_id: SessionId,
    pub data: String, // base64
}

#[derive(Debug, Clone, Deserialize)]
pub struct ResizeRequest {
    pub session_id: SessionId,
    pub term_size: TermSize,
}

#[derive(Debug, Serialize)]
pub struct OutputNotification {
    pub session_id: SessionId,
    pub data: String, // base64
}

#[derive(Debug, Serialize)]
pub struct SessionStatusNotification {
    pub session_id: SessionId,
    pub status: SessionStatus,
    pub exit_code: Option<i32>,
}

#[derive(Debug, Serialize)]
pub struct TitleNotification {
    pub session_id: SessionId,
    pub title: String,
}

#[derive(Debug, Serialize)]
pub struct CwdNotification {
    pub session_id: SessionId,
    pub cwd: String,
}
```

#### src/pty/manager.rs

```rust
use anyhow::{Context, Result};
use portable_pty::{CommandBuilder, PtySize, PtySystem};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, error, info};
use uuid::Uuid;

use crate::rpc::server::Notification;
use crate::rpc::types::*;
use crate::shell::detect::detect_default_shell;

pub struct PtySession {
    pub id: SessionId,
    pub info: Arc<RwLock<SessionInfo>>,
}

pub struct PtyManager {
    sessions: Arc<RwLock<HashMap<SessionId, PtySession>>>,
    notification_tx: mpsc::Sender<Notification>,
}

impl PtyManager {
    pub fn new(notification_tx: mpsc::Sender<Notification>) -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            notification_tx,
        }
    }

    pub async fn create_session(
        &self,
        request: CreateSessionRequest,
    ) -> Result<CreateSessionResponse> {
        let session_id = Uuid::new_v4().to_string();

        let (shell_path, cwd, env) = match &request.connection {
            ConnectionType::Local { shell_path, cwd, env } => {
                (shell_path.clone(), cwd.clone(), env.clone())
            }
            _ => anyhow::bail!("Invalid connection type for PTY"),
        };

        let shell = shell_path.unwrap_or_else(|| detect_default_shell());

        let pty_system = portable_pty::native_pty_system();
        let pty_pair = pty_system
            .openpty(PtySize {
                rows: request.term_size.rows,
                cols: request.term_size.cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .context("Failed to open PTY")?;

        let mut cmd = CommandBuilder::new(&shell);

        if let Some(cwd) = &cwd {
            cmd.cwd(cwd);
        }

        cmd.env("TERM", "xterm-256color");
        cmd.env("COLORTERM", "truecolor");

        if let Some(env) = env {
            for (key, value) in env {
                cmd.env(key, value);
            }
        }

        let child = pty_pair.slave.spawn_command(cmd)?;
        let mut writer = pty_pair.master.take_writer()?;
        let mut reader = pty_pair.master.try_clone_reader()?;

        let session_info = Arc::new(RwLock::new(SessionInfo {
            id: session_id.clone(),
            connection_type: request.connection.clone(),
            status: SessionStatus::Running,
            title: None,
            cwd,
            exit_code: None,
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        }));

        let session = PtySession {
            id: session_id.clone(),
            info: session_info.clone(),
        };

        self.sessions.write().await.insert(session_id.clone(), session);

        // 启动输出读取任务
        let session_id_clone = session_id.clone();
        let notification_tx = self.notification_tx.clone();
        tokio::task::spawn_blocking(move || {
            let mut buf = [0u8; 4096];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        let data = base64::encode(&buf[..n]);
                        let _ = notification_tx.blocking_send(Notification::Output(
                            OutputNotification {
                                session_id: session_id_clone.clone(),
                                data,
                            },
                        ));
                    }
                    Err(_) => break,
                }
            }
        });

        // 启动进程监控任务
        let session_id_clone = session_id.clone();
        let notification_tx = self.notification_tx.clone();
        let session_info = session_info.clone();
        tokio::spawn(async move {
            let exit_status = child.wait();
            let exit_code = exit_status.ok().and_then(|s| s.exit_code());

            {
                let mut info = session_info.write().await;
                info.status = SessionStatus::Done;
                info.exit_code = exit_code;
            }

            let _ = notification_tx
                .send(Notification::SessionStatus(SessionStatusNotification {
                    session_id: session_id_clone,
                    status: SessionStatus::Done,
                    exit_code,
                }))
                .await;
        });

        info!("Created PTY session: {}", session_id);
        Ok(CreateSessionResponse { session_id })
    }

    pub async fn send_input(&self, request: InputRequest) -> Result<()> {
        // 实现发送输入逻辑
        // 需要存储 writer 到 session 中
        todo!("Implement send_input")
    }

    pub async fn resize(&self, request: ResizeRequest) -> Result<()> {
        // 实现调整大小逻辑
        todo!("Implement resize")
    }

    pub async fn close_session(&self, session_id: &str) -> Result<()> {
        let mut sessions = self.sessions.write().await;
        sessions.remove(session_id);
        info!("Closed session: {}", session_id);
        Ok(())
    }

    pub async fn list_sessions(&self) -> Vec<SessionInfo> {
        let sessions = self.sessions.read().await;
        let mut result = Vec::new();

        for session in sessions.values() {
            let info = session.info.read().await;
            result.push(info.clone());
        }

        result
    }
}
```

#### src/shell/detect.rs

```rust
pub fn detect_default_shell() -> String {
    #[cfg(unix)]
    {
        std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string())
    }

    #[cfg(windows)]
    {
        std::env::var("COMSPEC").unwrap_or_else(|_| "cmd.exe".to_string())
    }
}
```

---

## React 前端实现

### package.json

```json
{
  "name": "proxycast-terminal-ui",
  "version": "0.1.0",
  "type": "module",
  "scripts": {
    "dev": "vite",
    "build": "tsc && vite build",
    "preview": "vite preview"
  },
  "dependencies": {
    "@xterm/xterm": "^5.5.0",
    "@xterm/addon-fit": "^0.10.0",
    "@xterm/addon-search": "^0.15.0",
    "@xterm/addon-webgl": "^0.18.0",
    "@xterm/addon-web-links": "^0.11.0",
    "react": "^18.2.0",
    "react-dom": "^18.2.0"
  },
  "devDependencies": {
    "@types/react": "^18.2.0",
    "@types/react-dom": "^18.2.0",
    "@vitejs/plugin-react": "^4.2.0",
    "autoprefixer": "^10.4.16",
    "postcss": "^8.4.32",
    "tailwindcss": "^3.3.6",
    "typescript": "^5.3.3",
    "vite": "^5.0.10"
  },
  "peerDependencies": {
    "react": "^18.2.0",
    "react-dom": "^18.2.0"
  }
}
```

### vite.config.ts

```typescript
import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import path from 'path';

export default defineConfig({
  plugins: [react()],
  resolve: {
    alias: {
      '@': path.resolve(__dirname, 'src'),
    },
  },
  build: {
    outDir: '../plugin/dist/frontend',
    emptyOutDir: true,
    lib: {
      entry: path.resolve(__dirname, 'src/index.tsx'),
      name: 'ProxyCastTerminal',
      formats: ['iife'],
      fileName: () => 'index.js',
    },
    rollupOptions: {
      external: ['react', 'react-dom'],
      output: {
        globals: {
          react: 'React',
          'react-dom': 'ReactDOM',
        },
        assetFileNames: (assetInfo) => {
          if (assetInfo.name === 'style.css') return 'styles.css';
          return assetInfo.name;
        },
      },
    },
  },
});
```

### 核心代码模板

#### src/lib/rpc-client.ts

```typescript
import { EventEmitter } from 'events';

export interface RpcRequest {
  jsonrpc: '2.0';
  id: number;
  method: string;
  params?: any;
}

export interface RpcResponse {
  jsonrpc: '2.0';
  id: number;
  result?: any;
  error?: {
    code: number;
    message: string;
  };
}

export interface RpcNotification {
  jsonrpc: '2.0';
  method: string;
  params: any;
}

export class RpcClient extends EventEmitter {
  private requestId = 0;
  private pendingRequests = new Map<number, {
    resolve: (result: any) => void;
    reject: (error: Error) => void;
  }>();

  constructor(private sendMessage: (message: string) => void) {
    super();
  }

  async call(method: string, params?: any): Promise<any> {
    const id = ++this.requestId;
    const request: RpcRequest = {
      jsonrpc: '2.0',
      id,
      method,
      params,
    };

    return new Promise((resolve, reject) => {
      this.pendingRequests.set(id, { resolve, reject });
      this.sendMessage(JSON.stringify(request));
    });
  }

  handleMessage(message: string) {
    try {
      const json = JSON.parse(message);

      if ('id' in json) {
        // Response
        const response = json as RpcResponse;
        const pending = this.pendingRequests.get(response.id);

        if (pending) {
          this.pendingRequests.delete(response.id);

          if (response.error) {
            pending.reject(new Error(response.error.message));
          } else {
            pending.resolve(response.result);
          }
        }
      } else if ('method' in json) {
        // Notification
        const notification = json as RpcNotification;
        this.emit(notification.method, notification.params);
      }
    } catch (error) {
      console.error('Failed to parse RPC message:', error);
    }
  }
}
```

#### src/lib/termwrap.ts

```typescript
import { Terminal } from '@xterm/xterm';
import { FitAddon } from '@xterm/addon-fit';
import { SearchAddon } from '@xterm/addon-search';
import { WebLinksAddon } from '@xterm/addon-web-links';
import { WebglAddon } from '@xterm/addon-webgl';

export interface TermWrapOptions {
  onData: (data: string) => void;
  onResize: (rows: number, cols: number) => void;
  theme?: any;
  fontSize?: number;
  useWebGl?: boolean;
}

export class TermWrap {
  public terminal: Terminal;
  public fitAddon: FitAddon;
  public searchAddon: SearchAddon;
  private webLinksAddon: WebLinksAddon;
  private webglAddon?: WebglAddon;

  constructor(
    private container: HTMLElement,
    private options: TermWrapOptions
  ) {
    this.terminal = new Terminal({
      theme: options.theme,
      fontSize: options.fontSize || 14,
      fontFamily: 'Menlo, Monaco, "Courier New", monospace',
      cursorBlink: true,
      allowTransparency: true,
      scrollback: 10000,
    });

    this.fitAddon = new FitAddon();
    this.searchAddon = new SearchAddon();
    this.webLinksAddon = new WebLinksAddon();

    this.terminal.loadAddon(this.fitAddon);
    this.terminal.loadAddon(this.searchAddon);
    this.terminal.loadAddon(this.webLinksAddon);

    if (options.useWebGl) {
      try {
        this.webglAddon = new WebglAddon();
        this.terminal.loadAddon(this.webglAddon);
      } catch (e) {
        console.warn('WebGL addon failed to load:', e);
      }
    }

    this.terminal.open(container);
    this.fitAddon.fit();

    this.terminal.onData((data) => {
      this.options.onData(data);
    });

    this.terminal.onResize(({ rows, cols }) => {
      this.options.onResize(rows, cols);
    });

    // 监听窗口大小变化
    const resizeObserver = new ResizeObserver(() => {
      this.fit();
    });
    resizeObserver.observe(container);
  }

  write(data: Uint8Array | string) {
    if (data instanceof Uint8Array) {
      this.terminal.write(data);
    } else {
      this.terminal.write(data);
    }
  }

  fit() {
    this.fitAddon.fit();
  }

  focus() {
    this.terminal.focus();
  }

  dispose() {
    this.terminal.dispose();
  }
}
```

#### src/components/Terminal/index.tsx

```typescript
import React, { useEffect, useRef, useState } from 'react';
import { TermWrap } from '../../lib/termwrap';
import { useRpc } from '../../hooks/useRpc';
import '@xterm/xterm/css/xterm.css';

export interface TerminalProps {
  sessionId: string;
}

export const Terminal: React.FC<TerminalProps> = ({ sessionId }) => {
  const containerRef = useRef<HTMLDivElement>(null);
  const termWrapRef = useRef<TermWrap | null>(null);
  const { rpcClient } = useRpc();

  useEffect(() => {
    if (!containerRef.current || !rpcClient) return;

    const termWrap = new TermWrap(containerRef.current, {
      onData: (data) => {
        const encoded = btoa(data);
        rpcClient.call('session.input', {
          session_id: sessionId,
          data: encoded,
        });
      },
      onResize: (rows, cols) => {
        rpcClient.call('session.resize', {
          session_id: sessionId,
          term_size: { rows, cols },
        });
      },
      theme: {
        background: '#1e1e1e',
        foreground: '#d4d4d4',
      },
      fontSize: 14,
      useWebGl: true,
    });

    termWrapRef.current = termWrap;

    // 监听终端输出
    const handleOutput = (params: { session_id: string; data: string }) => {
      if (params.session_id === sessionId) {
        const decoded = atob(params.data);
        termWrap.write(decoded);
      }
    };

    rpcClient.on('terminal.output', handleOutput);

    return () => {
      rpcClient.off('terminal.output', handleOutput);
      termWrap.dispose();
    };
  }, [sessionId, rpcClient]);

  return (
    <div
      ref={containerRef}
      className="w-full h-full"
      style={{ backgroundColor: '#1e1e1e' }}
    />
  );
};
```

#### src/App.tsx

```typescript
import React, { useEffect, useState } from 'react';
import { Terminal } from './components/Terminal';
import { useRpc } from './hooks/useRpc';

export const App: React.FC = () => {
  const [sessions, setSessions] = useState<string[]>([]);
  const [activeSession, setActiveSession] = useState<string | null>(null);
  const { rpcClient, isReady } = useRpc();

  useEffect(() => {
    if (!isReady || !rpcClient) return;

    // 创建初始会话
    createSession();
  }, [isReady]);

  const createSession = async () => {
    if (!rpcClient) return;

    try {
      const result = await rpcClient.call('session.create', {
        connection: {
          type: 'local',
        },
        term_size: {
          rows: 24,
          cols: 80,
        },
      });

      setSessions((prev) => [...prev, result.session_id]);
      setActiveSession(result.session_id);
    } catch (error) {
      console.error('Failed to create session:', error);
    }
  };

  return (
    <div className="w-full h-screen bg-gray-900">
      {activeSession && <Terminal sessionId={activeSession} />}
    </div>
  );
};
```

---

## 插件配置

### plugin.json

```json
{
  "name": "terminal",
  "version": "0.1.0",
  "displayName": "Terminal",
  "description": "Full-featured terminal plugin with SSH support",
  "plugin_type": "tool",
  "entry": "terminal-cli",
  "min_proxycast_version": "0.31.0",
  "binary": {
    "binary_name": "terminal-cli",
    "platform_binaries": {
      "macos-arm64": "terminal-aarch64-apple-darwin",
      "macos-x64": "terminal-x86_64-apple-darwin",
      "linux-x64": "terminal-x86_64-unknown-linux-gnu",
      "windows-x64": "terminal-x86_64-pc-windows-msvc.exe"
    }
  },
  "ui": {
    "surfaces": ["tools"],
    "entry": "dist/frontend/index.js",
    "styles": "dist/frontend/styles.css"
  }
}
```

### config.json

```json
{
  "enabled": true,
  "settings": {
    "terminal": {
      "default_shell": null,
      "font_size": 14,
      "font_family": "Menlo, Monaco, Courier New, monospace",
      "scrollback": 10000,
      "use_webgl": true
    },
    "ssh": {
      "default_port": 22,
      "connect_timeout": 30,
      "keepalive_interval": 30
    }
  }
}
```

---

## 实现步骤

### 阶段 1: 项目初始化 (1-2 天)

**目标**: 搭建项目基础框架

- [ ] 创建项目目录结构
- [ ] 初始化 Rust 项目 (`cargo init --lib`)
- [ ] 配置 Cargo.toml 依赖
- [ ] 初始化前端项目 (`npm init`)
- [ ] 配置 package.json 和 vite.config.ts
- [ ] 创建 plugin.json 和 config.json
- [ ] 创建 .gitignore

**验收标准**:
- `cargo build` 成功
- `npm install` 成功
- 项目结构完整

---

### 阶段 2: Rust 后端 - RPC 框架 (2-3 天)

**目标**: 实现 JSON-RPC 服务器基础

- [ ] 实现 `src/rpc/types.rs` - 定义所有 RPC 数据类型
- [ ] 实现 `src/rpc/server.rs` - JSON-RPC 服务器框架
- [ ] 实现 stdin/stdout 通信
- [ ] 实现通知发送机制
- [ ] 编写单元测试

**验收标准**:
- 可以接收和响应 JSON-RPC 请求
- 可以发送通知到 stdout
- 所有类型定义完整

---

### 阶段 3: Rust 后端 - 本地 PTY (3-4 天)

**目标**: 实现本地终端功能

- [ ] 实现 `src/shell/detect.rs` - Shell 检测
- [ ] 实现 `src/pty/manager.rs` - PTY 会话管理
- [ ] 实现 PTY 创建和配置
- [ ] 实现输入/输出处理
- [ ] 实现终端大小调整
- [ ] 实现会话生命周期管理
- [ ] 注册 RPC 方法: `session.create`, `session.input`, `session.resize`

**验收标准**:
- 可以创建本地终端会话
- 可以发送输入并接收输出
- 可以调整终端大小
- 进程退出时正确通知

---

### 阶段 4: Rust 后端 - SSH 支持 (4-5 天)

**目标**: 实现 SSH 远程连接

- [ ] 实现 `src/ssh/config.rs` - SSH 配置解析
- [ ] 实现 `src/ssh/auth.rs` - SSH 认证（密钥/密码/Agent）
- [ ] 实现 `src/ssh/client.rs` - SSH 客户端
- [ ] 实现 `src/ssh/session.rs` - SSH 会话管理
- [ ] 实现 SSH PTY 通道
- [ ] 处理 SSH 连接状态

**验收标准**:
- 可以通过 SSH 连接远程主机
- 支持多种认证方式
- SSH 会话正常工作

---

### 阶段 5: React 前端 - 基础组件 (3-4 天)

**目标**: 实现终端 UI 基础

- [ ] 实现 `src/lib/rpc-client.ts` - RPC 客户端
- [ ] 实现 `src/lib/termwrap.ts` - xterm.js 包装器
- [ ] 实现 `src/lib/fitaddon.ts` - 自定义 FitAddon
- [ ] 实现 `src/hooks/useRpc.ts` - RPC Hook
- [ ] 实现 `src/components/Terminal/index.tsx` - 终端组件
- [ ] 配置 xterm.js 主题

**验收标准**:
- 终端可以正常显示
- 可以输入和输出
- FitAddon 正常工作

---

### 阶段 6: React 前端 - 多标签页 (2-3 天)

**目标**: 实现标签页管理

- [ ] 实现 `src/components/Terminal/TerminalTabs.tsx`
- [ ] 实现 `src/store/session-store.ts` - 会话状态管理
- [ ] 实现标签页切换
- [ ] 实现标签页关闭
- [ ] 实现新建标签页

**验收标准**:
- 可以创建多个终端标签
- 标签页切换正常
- 关闭标签页释放资源

---

### 阶段 7: React 前端 - 高级功能 (3-4 天)

**目标**: 实现搜索和 SSH 对话框

- [ ] 实现 `src/components/Search/index.tsx` - 搜索组件
- [ ] 集成 SearchAddon
- [ ] 实现 `src/components/Dialogs/NewConnectionDialog.tsx`
- [ ] 实现 SSH 连接表单
- [ ] 实现连接配置保存

**验收标准**:
- 搜索功能正常工作
- SSH 对话框可以创建连接
- 配置可以保存和加载

---

### 阶段 8: Shell 集成 (2-3 天)

**目标**: 实现 OSC 序列处理

- [ ] 实现 `src/shell/osc.rs` - OSC 序列解析
- [ ] 实现 OSC 7 (工作目录追踪)
- [ ] 实现 OSC 52 (剪贴板支持)
- [ ] 前端处理 OSC 通知
- [ ] 更新会话信息

**验收标准**:
- 工作目录变更可以追踪
- 剪贴板操作正常工作

---

### 阶段 9: 构建与打包 (2-3 天)

**目标**: 实现跨平台构建

- [ ] 编写构建脚本
- [ ] 配置交叉编译（macOS, Linux, Windows）
- [ ] 前端构建为 IIFE 格式
- [ ] 验证插件加载

**验收标准**:
- 所有平台二进制文件可以生成
- 前端正确打包
- 插件可以在 ProxyCast 中加载

---

### 阶段 10: 测试与优化 (3-4 天)

**目标**: 完善功能和性能

- [ ] 功能测试
- [ ] 性能测试
- [ ] 内存泄漏检查
- [ ] 错误处理完善
- [ ] 文档编写

**验收标准**:
- 所有功能正常工作
- 无明显性能问题
- 文档完整

---

## 依赖清单

### Rust 依赖

```toml
[dependencies]
# JSON-RPC
jsonrpc-core = "18.0"
jsonrpc-derive = "18.0"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# PTY
portable-pty = "0.8"

# SSH
russh = "0.44"
russh-keys = "0.44"
ssh2-config = "0.2"

# Async
tokio = { version = "1.0", features = ["full"] }
futures = "0.3"

# Utils
thiserror = "1.0"
anyhow = "1.0"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
base64 = "0.21"
uuid = { version = "1.0", features = ["v4", "serde"] }
dirs = "5.0"
shellexpand = "3.0"
```

### 前端依赖

```json
{
  "dependencies": {
    "@xterm/xterm": "^5.5.0",
    "@xterm/addon-fit": "^0.10.0",
    "@xterm/addon-search": "^0.15.0",
    "@xterm/addon-webgl": "^0.18.0",
    "@xterm/addon-web-links": "^0.11.0",
    "react": "^18.2.0",
    "react-dom": "^18.2.0"
  },
  "devDependencies": {
    "@types/react": "^18.2.0",
    "@types/react-dom": "^18.2.0",
    "@vitejs/plugin-react": "^4.2.0",
    "autoprefixer": "^10.4.16",
    "postcss": "^8.4.32",
    "tailwindcss": "^3.3.6",
    "typescript": "^5.3.3",
    "vite": "^5.0.10"
  }
}
```

---

## 参考资料

### Wave Terminal 源码参考

- `/frontend/app/view/term/term.tsx` - React 终端组件
- `/frontend/app/view/term/termwrap.ts` - xterm.js 包装器
- `/frontend/app/view/term/fitaddon.ts` - 自定义 FitAddon
- `/pkg/shellexec/shellexec.go` - PTY 管理（Go，需转换为 Rust）
- `/pkg/remote/conncontroller/conncontroller.go` - SSH 连接（Go，需转换为 Rust）

### kiro-provider 插件参考

- `/kiro-provider/plugin.json` - 插件配置示例
- `/kiro-provider/src-tauri/src/main.rs` - Rust CLI 入口
- `/kiro-provider/vite.config.ts` - Vite 构建配置

### 外部文档

- [xterm.js 文档](https://xtermjs.org/)
- [portable-pty 文档](https://docs.rs/portable-pty/)
- [russh 文档](https://docs.rs/russh/)
- [jsonrpc-core 文档](https://docs.rs/jsonrpc-core/)

---

## 总结

这是一个完整的终端插件实现计划，包含：

- **架构设计**: 清晰的双层架构（Rust 后端 + React 前端）
- **详细结构**: 完整的项目目录和文件组织
- **核心代码**: Rust 和 TypeScript 的代码模板
- **实现步骤**: 10 个阶段，约 25-35 天的开发周期

建议按照阶段顺序逐步实现，每个阶段完成后进行测试验证，确保质量。
