//! RPC 通信模块
//!
//! 实现 JSON-RPC 2.0 协议的服务器端，通过 stdin/stdout 与前端通信。

pub mod server;
pub mod methods;
pub mod types;

pub use server::{RpcServer, NotificationSender};
pub use types::*;
