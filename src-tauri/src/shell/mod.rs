//! Shell 集成模块
//!
//! 负责 Shell 检测和 OSC 序列处理。

pub mod detect;
pub mod osc;

pub use detect::detect_default_shell;
pub use osc::{ClipboardData, ClipboardSelection, OscHandler, OscParseResult, OscSequence};
