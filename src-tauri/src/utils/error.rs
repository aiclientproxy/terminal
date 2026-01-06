//! 错误类型定义
//!
//! 定义终端插件的错误类型，提供描述性错误消息。
//! 
//! ## 功能
//! - 定义 TerminalError 枚举，涵盖所有可能的错误类型
//! - 实现错误转换（From trait）
//! - 提供错误分类和辅助方法
//! - 支持转换为 JSON-RPC 错误格式
//!
//! ## 需求覆盖
//! - 需求 10.1: PTY 创建失败时返回描述性错误消息
//! - 需求 10.2: SSH 连接失败时返回连接错误详情

use thiserror::Error;
use crate::rpc::types::JsonRpcError;

/// 终端错误类型
#[derive(Debug, Error)]
pub enum TerminalError {
    /// PTY 创建失败
    #[error("PTY 创建失败: {0}")]
    PtyCreationFailed(String),

    /// SSH 连接失败
    #[error("SSH 连接失败: {0}")]
    SshConnectionFailed(String),

    /// 会话不存在
    #[error("会话不存在: {0}")]
    SessionNotFound(String),

    /// 无效的请求
    #[error("无效的请求: {0}")]
    InvalidRequest(String),

    /// IO 错误
    #[error("IO 错误: {0}")]
    IoError(#[from] std::io::Error),

    /// 序列化错误
    #[error("序列化错误: {0}")]
    SerializationError(#[from] serde_json::Error),

    /// 认证失败
    #[error("认证失败: {0}")]
    AuthenticationFailed(String),

    /// 连接超时
    #[error("连接超时: {0}")]
    ConnectionTimeout(String),

    /// 会话已关闭
    #[error("会话已关闭: {0}")]
    SessionClosed(String),

    /// SSH 协议错误
    #[error("SSH 错误: {0}")]
    SshError(String),

    /// 通道错误
    #[error("通道错误: {0}")]
    ChannelError(String),

    /// 主机解析失败
    #[error("主机解析失败: {0}")]
    HostResolutionFailed(String),

    /// 私钥加载失败
    #[error("私钥加载失败: {0}")]
    PrivateKeyLoadFailed(String),
}

impl From<russh::Error> for TerminalError {
    fn from(err: russh::Error) -> Self {
        // 将 russh 错误转换为更友好的错误消息
        let message = match &err {
            russh::Error::Disconnect => "服务器断开连接".to_string(),
            russh::Error::NoCommonKexAlgo => "无法协商密钥交换算法".to_string(),
            russh::Error::NoCommonCipher => "无法协商加密算法".to_string(),
            russh::Error::NoCommonCompression => "无法协商压缩算法".to_string(),
            russh::Error::NoCommonMac => "无法协商 MAC 算法".to_string(),
            russh::Error::NoCommonKeyAlgo => "无法协商密钥算法".to_string(),
            _ => err.to_string(),
        };
        TerminalError::SshError(message)
    }
}

impl From<TerminalError> for JsonRpcError {
    fn from(err: TerminalError) -> Self {
        // 根据错误类型映射到适当的 JSON-RPC 错误码
        // 使用应用特定的错误码范围 (-32000 到 -32099)
        let code = match &err {
            TerminalError::SessionNotFound(_) => -32001,
            TerminalError::InvalidRequest(_) => -32602, // 使用标准的无效参数错误码
            TerminalError::SerializationError(_) => -32700, // 使用标准的解析错误码
            TerminalError::PtyCreationFailed(_) => -32010,
            TerminalError::SshConnectionFailed(_) => -32020,
            TerminalError::AuthenticationFailed(_) => -32021,
            TerminalError::ConnectionTimeout(_) => -32022,
            TerminalError::HostResolutionFailed(_) => -32023,
            TerminalError::PrivateKeyLoadFailed(_) => -32024,
            TerminalError::SshError(_) => -32025,
            TerminalError::ChannelError(_) => -32026,
            TerminalError::SessionClosed(_) => -32002,
            TerminalError::IoError(_) => -32603, // 使用标准的内部错误码
        };

        JsonRpcError {
            code,
            message: err.to_string(),
            data: Some(serde_json::json!({
                "error_type": err.error_type(),
                "error_code": err.code(),
                "recoverable": err.is_recoverable(),
            })),
        }
    }
}

impl TerminalError {
    /// 获取错误码
    pub fn code(&self) -> i32 {
        match self {
            TerminalError::PtyCreationFailed(_) => 1001,
            TerminalError::SshConnectionFailed(_) => 1002,
            TerminalError::SessionNotFound(_) => 1003,
            TerminalError::InvalidRequest(_) => 1004,
            TerminalError::IoError(_) => 1005,
            TerminalError::SerializationError(_) => 1006,
            TerminalError::AuthenticationFailed(_) => 1007,
            TerminalError::ConnectionTimeout(_) => 1008,
            TerminalError::SessionClosed(_) => 1009,
            TerminalError::SshError(_) => 1010,
            TerminalError::ChannelError(_) => 1011,
            TerminalError::HostResolutionFailed(_) => 1012,
            TerminalError::PrivateKeyLoadFailed(_) => 1013,
        }
    }

    /// 获取错误类型名称
    pub fn error_type(&self) -> &'static str {
        match self {
            TerminalError::PtyCreationFailed(_) => "pty_creation_failed",
            TerminalError::SshConnectionFailed(_) => "ssh_connection_failed",
            TerminalError::SessionNotFound(_) => "session_not_found",
            TerminalError::InvalidRequest(_) => "invalid_request",
            TerminalError::IoError(_) => "io_error",
            TerminalError::SerializationError(_) => "serialization_error",
            TerminalError::AuthenticationFailed(_) => "authentication_failed",
            TerminalError::ConnectionTimeout(_) => "connection_timeout",
            TerminalError::SessionClosed(_) => "session_closed",
            TerminalError::SshError(_) => "ssh_error",
            TerminalError::ChannelError(_) => "channel_error",
            TerminalError::HostResolutionFailed(_) => "host_resolution_failed",
            TerminalError::PrivateKeyLoadFailed(_) => "private_key_load_failed",
        }
    }

    /// 检查是否为可恢复错误
    pub fn is_recoverable(&self) -> bool {
        matches!(
            self,
            TerminalError::ConnectionTimeout(_)
                | TerminalError::HostResolutionFailed(_)
                | TerminalError::AuthenticationFailed(_)
        )
    }

    /// 检查是否为认证相关错误
    pub fn is_auth_error(&self) -> bool {
        matches!(
            self,
            TerminalError::AuthenticationFailed(_) | TerminalError::PrivateKeyLoadFailed(_)
        )
    }

    /// 检查是否为连接相关错误
    pub fn is_connection_error(&self) -> bool {
        matches!(
            self,
            TerminalError::SshConnectionFailed(_)
                | TerminalError::ConnectionTimeout(_)
                | TerminalError::HostResolutionFailed(_)
                | TerminalError::SshError(_)
        )
    }

    // ============ SSH 错误构造辅助方法 ============

    /// 创建 SSH 连接失败错误（包含主机信息）
    pub fn ssh_connection_failed(host: &str, port: u16, reason: &str) -> Self {
        TerminalError::SshConnectionFailed(format!(
            "无法连接到 {}:{} - {}",
            host, port, reason
        ))
    }

    /// 创建认证失败错误（包含认证方式）
    pub fn auth_failed(method: &str, reason: &str) -> Self {
        TerminalError::AuthenticationFailed(format!(
            "{}认证失败: {}",
            method, reason
        ))
    }

    /// 创建密码认证失败错误
    pub fn password_auth_failed(reason: &str) -> Self {
        Self::auth_failed("密码", reason)
    }

    /// 创建私钥认证失败错误
    pub fn key_auth_failed(key_path: &str, reason: &str) -> Self {
        TerminalError::AuthenticationFailed(format!(
            "私钥认证失败 ({}): {}",
            key_path, reason
        ))
    }

    /// 创建私钥加载失败错误
    pub fn key_load_failed(key_path: &str, reason: &str) -> Self {
        TerminalError::PrivateKeyLoadFailed(format!(
            "无法加载私钥 {}: {}",
            key_path, reason
        ))
    }

    /// 创建主机解析失败错误
    pub fn host_resolution_failed(host: &str, reason: &str) -> Self {
        TerminalError::HostResolutionFailed(format!(
            "无法解析主机 {}: {}",
            host, reason
        ))
    }

    /// 创建连接超时错误
    pub fn connection_timeout(host: &str, port: u16, timeout_secs: u64) -> Self {
        TerminalError::ConnectionTimeout(format!(
            "连接 {}:{} 超时 ({}秒)",
            host, port, timeout_secs
        ))
    }

    /// 创建通道错误
    pub fn channel_error(operation: &str, reason: &str) -> Self {
        TerminalError::ChannelError(format!(
            "{} 失败: {}",
            operation, reason
        ))
    }

    /// 创建会话关闭错误
    pub fn session_closed(session_id: &str, reason: &str) -> Self {
        TerminalError::SessionClosed(format!(
            "会话 {} 已关闭: {}",
            session_id, reason
        ))
    }
}

/// SSH 错误详情
/// 
/// 提供更详细的 SSH 错误信息，用于日志和调试。
#[derive(Debug, Clone)]
pub struct SshErrorDetails {
    /// 主机地址
    pub host: String,
    /// 端口
    pub port: u16,
    /// 用户名
    pub user: Option<String>,
    /// 认证方式
    pub auth_method: Option<String>,
    /// 错误消息
    pub message: String,
    /// 原始错误（如果有）
    pub cause: Option<String>,
}

impl SshErrorDetails {
    /// 创建新的错误详情
    pub fn new(host: &str, port: u16, message: &str) -> Self {
        Self {
            host: host.to_string(),
            port,
            user: None,
            auth_method: None,
            message: message.to_string(),
            cause: None,
        }
    }

    /// 设置用户名
    pub fn with_user(mut self, user: &str) -> Self {
        self.user = Some(user.to_string());
        self
    }

    /// 设置认证方式
    pub fn with_auth_method(mut self, method: &str) -> Self {
        self.auth_method = Some(method.to_string());
        self
    }

    /// 设置原始错误
    pub fn with_cause(mut self, cause: &str) -> Self {
        self.cause = Some(cause.to_string());
        self
    }

    /// 转换为 TerminalError
    pub fn into_error(self) -> TerminalError {
        let mut msg = format!("{}:{}", self.host, self.port);
        
        if let Some(user) = &self.user {
            msg = format!("{}@{}", user, msg);
        }
        
        msg = format!("{} - {}", msg, self.message);
        
        if let Some(auth) = &self.auth_method {
            msg = format!("{} (认证方式: {})", msg, auth);
        }
        
        if let Some(cause) = &self.cause {
            msg = format!("{} [原因: {}]", msg, cause);
        }
        
        TerminalError::SshConnectionFailed(msg)
    }
}

impl std::fmt::Display for SshErrorDetails {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{} - {}", self.host, self.port, self.message)?;
        if let Some(user) = &self.user {
            write!(f, " (user: {})", user)?;
        }
        if let Some(cause) = &self.cause {
            write!(f, " [{}]", cause)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = TerminalError::SessionNotFound("test-id".to_string());
        assert_eq!(err.to_string(), "会话不存在: test-id");
    }

    #[test]
    fn test_error_codes() {
        assert_eq!(TerminalError::PtyCreationFailed("".to_string()).code(), 1001);
        assert_eq!(TerminalError::SshConnectionFailed("".to_string()).code(), 1002);
        assert_eq!(TerminalError::SessionNotFound("".to_string()).code(), 1003);
        assert_eq!(TerminalError::AuthenticationFailed("".to_string()).code(), 1007);
        assert_eq!(TerminalError::ChannelError("".to_string()).code(), 1011);
    }

    #[test]
    fn test_error_types() {
        assert_eq!(
            TerminalError::SshConnectionFailed("".to_string()).error_type(),
            "ssh_connection_failed"
        );
        assert_eq!(
            TerminalError::AuthenticationFailed("".to_string()).error_type(),
            "authentication_failed"
        );
    }

    #[test]
    fn test_is_recoverable() {
        assert!(TerminalError::ConnectionTimeout("".to_string()).is_recoverable());
        assert!(TerminalError::AuthenticationFailed("".to_string()).is_recoverable());
        assert!(!TerminalError::SessionNotFound("".to_string()).is_recoverable());
    }

    #[test]
    fn test_is_auth_error() {
        assert!(TerminalError::AuthenticationFailed("".to_string()).is_auth_error());
        assert!(TerminalError::PrivateKeyLoadFailed("".to_string()).is_auth_error());
        assert!(!TerminalError::SshConnectionFailed("".to_string()).is_auth_error());
    }

    #[test]
    fn test_is_connection_error() {
        assert!(TerminalError::SshConnectionFailed("".to_string()).is_connection_error());
        assert!(TerminalError::ConnectionTimeout("".to_string()).is_connection_error());
        assert!(!TerminalError::AuthenticationFailed("".to_string()).is_connection_error());
    }

    #[test]
    fn test_ssh_connection_failed_helper() {
        let err = TerminalError::ssh_connection_failed("example.com", 22, "连接被拒绝");
        assert!(err.to_string().contains("example.com"));
        assert!(err.to_string().contains("22"));
        assert!(err.to_string().contains("连接被拒绝"));
    }

    #[test]
    fn test_auth_failed_helper() {
        let err = TerminalError::password_auth_failed("密码错误");
        assert!(err.to_string().contains("密码"));
        assert!(err.to_string().contains("密码错误"));
    }

    #[test]
    fn test_key_auth_failed_helper() {
        let err = TerminalError::key_auth_failed("/path/to/key", "密钥格式无效");
        assert!(err.to_string().contains("/path/to/key"));
        assert!(err.to_string().contains("密钥格式无效"));
    }

    #[test]
    fn test_connection_timeout_helper() {
        let err = TerminalError::connection_timeout("example.com", 22, 30);
        assert!(err.to_string().contains("example.com"));
        assert!(err.to_string().contains("30"));
    }

    #[test]
    fn test_ssh_error_details() {
        let details = SshErrorDetails::new("example.com", 22, "连接失败")
            .with_user("root")
            .with_auth_method("password")
            .with_cause("Connection refused");
        
        let err = details.into_error();
        let msg = err.to_string();
        
        assert!(msg.contains("example.com"));
        assert!(msg.contains("22"));
        assert!(msg.contains("root"));
        assert!(msg.contains("password"));
        assert!(msg.contains("Connection refused"));
    }

    #[test]
    fn test_ssh_error_details_display() {
        let details = SshErrorDetails::new("example.com", 22, "连接失败")
            .with_user("root")
            .with_cause("timeout");
        
        let display = format!("{}", details);
        assert!(display.contains("example.com:22"));
        assert!(display.contains("root"));
        assert!(display.contains("timeout"));
    }

    #[test]
    fn test_terminal_error_to_json_rpc_error() {
        // 测试会话不存在错误
        let err = TerminalError::SessionNotFound("test-id".to_string());
        let rpc_err: JsonRpcError = err.into();
        assert_eq!(rpc_err.code, -32001);
        assert!(rpc_err.message.contains("test-id"));
        assert!(rpc_err.data.is_some());

        // 测试无效请求错误
        let err = TerminalError::InvalidRequest("bad params".to_string());
        let rpc_err: JsonRpcError = err.into();
        assert_eq!(rpc_err.code, -32602);

        // 测试 PTY 创建失败错误
        let err = TerminalError::PtyCreationFailed("no pty".to_string());
        let rpc_err: JsonRpcError = err.into();
        assert_eq!(rpc_err.code, -32010);

        // 测试 SSH 连接失败错误
        let err = TerminalError::SshConnectionFailed("connection refused".to_string());
        let rpc_err: JsonRpcError = err.into();
        assert_eq!(rpc_err.code, -32020);

        // 测试认证失败错误
        let err = TerminalError::AuthenticationFailed("wrong password".to_string());
        let rpc_err: JsonRpcError = err.into();
        assert_eq!(rpc_err.code, -32021);
        
        // 验证 data 字段包含额外信息
        let data = rpc_err.data.unwrap();
        assert_eq!(data["error_type"], "authentication_failed");
        assert_eq!(data["recoverable"], true);
    }

    #[test]
    fn test_json_rpc_error_data_contains_metadata() {
        let err = TerminalError::ConnectionTimeout("10s".to_string());
        let rpc_err: JsonRpcError = err.into();
        
        let data = rpc_err.data.unwrap();
        assert_eq!(data["error_type"], "connection_timeout");
        assert_eq!(data["error_code"], 1008);
        assert_eq!(data["recoverable"], true);
    }
}
