//! 工具模块
//!
//! 提供错误类型、状态管理和通用工具函数。

pub mod error;
pub mod state;

pub use error::TerminalError;
pub use state::{SessionStateManager, StateTransitionResult};
