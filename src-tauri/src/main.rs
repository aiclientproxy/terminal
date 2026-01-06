//! Terminal Plugin CLI 入口点
//!
//! 该程序作为独立进程运行，通过 stdin/stdout 与前端进行 JSON-RPC 通信。
//! 主要功能：
//! - 本地 PTY 终端会话管理
//! - SSH 远程连接管理
//! - JSON-RPC 2.0 协议通信

mod rpc;
mod pty;
mod ssh;
mod shell;
mod utils;

use tracing_subscriber::{fmt, prelude::*, EnvFilter};

use crate::rpc::server::RpcServer;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 初始化日志系统，输出到 stderr 避免干扰 JSON-RPC 通信
    tracing_subscriber::registry()
        .with(fmt::layer().with_writer(std::io::stderr))
        .with(EnvFilter::from_default_env().add_directive(tracing::Level::INFO.into()))
        .init();

    tracing::info!("Terminal Plugin 启动");

    // 创建并运行 RPC 服务器
    let server = RpcServer::new();
    server.run().await?;

    Ok(())
}
