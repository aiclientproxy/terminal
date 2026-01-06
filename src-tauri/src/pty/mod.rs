//! PTY 管理模块
//!
//! 负责本地伪终端的创建和管理。

pub mod local;
pub mod manager;
pub mod output;
pub mod session;

pub use local::LocalPty;
pub use manager::PtyManager;
pub use output::{start_output_reader, OutputReaderConfig, OutputReaderHandle};
pub use session::PtySession;
