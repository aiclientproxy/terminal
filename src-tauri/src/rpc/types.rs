//! RPC 数据类型定义
//!
//! 定义 JSON-RPC 请求、响应和通知的数据结构。

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 终端尺寸
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TermSize {
    pub rows: u16,
    pub cols: u16,
}

impl Default for TermSize {
    fn default() -> Self {
        Self { rows: 24, cols: 80 }
    }
}

/// 连接类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ConnectionType {
    /// 本地 PTY 连接
    Local {
        #[serde(skip_serializing_if = "Option::is_none")]
        shell_path: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        cwd: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        env: Option<HashMap<String, String>>,
    },
    /// SSH 远程连接
    Ssh {
        host: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        port: Option<u16>,
        #[serde(skip_serializing_if = "Option::is_none")]
        user: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        identity_file: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        password: Option<String>,
    },
}

/// 会话状态
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SessionStatus {
    /// 初始化中
    Init,
    /// 连接中
    Connecting,
    /// 运行中
    Running,
    /// 已完成
    Done,
    /// 错误
    Error,
}

/// 会话信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    pub id: String,
    pub connection_type: ConnectionType,
    pub status: SessionStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    pub created_at: u64,
}

// ============ RPC 请求类型 ============

/// 创建会话请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSessionRequest {
    pub connection: ConnectionType,
    pub term_size: TermSize,
}

/// 创建会话响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSessionResponse {
    pub session_id: String,
}

/// 输入请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputRequest {
    pub session_id: String,
    /// Base64 编码的输入数据
    pub data: String,
}

/// 调整大小请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResizeRequest {
    pub session_id: String,
    pub term_size: TermSize,
}

/// 关闭会话请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloseSessionRequest {
    pub session_id: String,
}

/// 获取会话请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetSessionRequest {
    pub session_id: String,
}

// ============ RPC 通知类型 ============

/// 终端输出通知
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputNotification {
    pub session_id: String,
    /// Base64 编码的输出数据
    pub data: String,
}

/// 会话状态变更通知
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionStatusNotification {
    pub session_id: String,
    pub status: SessionStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
}

/// 会话标题变更通知
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionTitleNotification {
    pub session_id: String,
    pub title: String,
}

/// 工作目录变更通知
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionCwdNotification {
    pub session_id: String,
    pub cwd: String,
}

// ============ JSON-RPC 2.0 协议类型 ============

/// JSON-RPC 请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
    pub id: serde_json::Value,
}

/// JSON-RPC 响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
    pub id: serde_json::Value,
}

impl JsonRpcResponse {
    /// 创建成功响应
    pub fn success(id: serde_json::Value, result: serde_json::Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            result: Some(result),
            error: None,
            id,
        }
    }

    /// 创建错误响应
    pub fn error(id: serde_json::Value, error: JsonRpcError) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            result: None,
            error: Some(error),
            id,
        }
    }
}

/// JSON-RPC 错误
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

impl JsonRpcError {
    /// 解析错误 (-32700)
    pub fn parse_error(message: impl Into<String>) -> Self {
        Self {
            code: -32700,
            message: message.into(),
            data: None,
        }
    }

    /// 无效请求 (-32600)
    pub fn invalid_request(message: impl Into<String>) -> Self {
        Self {
            code: -32600,
            message: message.into(),
            data: None,
        }
    }

    /// 方法不存在 (-32601)
    pub fn method_not_found(method: impl Into<String>) -> Self {
        Self {
            code: -32601,
            message: format!("Method not found: {}", method.into()),
            data: None,
        }
    }

    /// 无效参数 (-32602)
    pub fn invalid_params(message: impl Into<String>) -> Self {
        Self {
            code: -32602,
            message: message.into(),
            data: None,
        }
    }

    /// 内部错误 (-32603)
    pub fn internal_error(message: impl Into<String>) -> Self {
        Self {
            code: -32603,
            message: message.into(),
            data: None,
        }
    }
}

/// JSON-RPC 通知
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcNotification {
    pub jsonrpc: String,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
}

impl JsonRpcNotification {
    /// 创建新通知
    pub fn new(method: impl Into<String>, params: serde_json::Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            method: method.into(),
            params: Some(params),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_term_size_default() {
        let size = TermSize::default();
        assert_eq!(size.rows, 24);
        assert_eq!(size.cols, 80);
    }

    #[test]
    fn test_connection_type_local_serialization() {
        let conn = ConnectionType::Local {
            shell_path: Some("/bin/zsh".to_string()),
            cwd: Some("/home/user".to_string()),
            env: None,
        };
        let json = serde_json::to_string(&conn).unwrap();
        assert!(json.contains("\"type\":\"local\""));
        assert!(json.contains("\"shell_path\":\"/bin/zsh\""));
    }

    #[test]
    fn test_connection_type_ssh_serialization() {
        let conn = ConnectionType::Ssh {
            host: "example.com".to_string(),
            port: Some(22),
            user: Some("root".to_string()),
            identity_file: None,
            password: None,
        };
        let json = serde_json::to_string(&conn).unwrap();
        assert!(json.contains("\"type\":\"ssh\""));
        assert!(json.contains("\"host\":\"example.com\""));
    }

    #[test]
    fn test_session_status_serialization() {
        assert_eq!(
            serde_json::to_string(&SessionStatus::Running).unwrap(),
            "\"running\""
        );
        assert_eq!(
            serde_json::to_string(&SessionStatus::Error).unwrap(),
            "\"error\""
        );
    }

    #[test]
    fn test_json_rpc_error_codes() {
        assert_eq!(JsonRpcError::parse_error("test").code, -32700);
        assert_eq!(JsonRpcError::invalid_request("test").code, -32600);
        assert_eq!(JsonRpcError::method_not_found("test").code, -32601);
        assert_eq!(JsonRpcError::invalid_params("test").code, -32602);
        assert_eq!(JsonRpcError::internal_error("test").code, -32603);
    }
}

/// Property-based tests for RPC types
/// Feature: terminal-plugin, Property 2: RPC 请求往返一致性
/// **验证: 需求 3.6**
#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    // Strategy for generating TermSize
    fn term_size_strategy() -> impl Strategy<Value = TermSize> {
        (1u16..500, 1u16..500).prop_map(|(rows, cols)| TermSize { rows, cols })
    }

    // Strategy for generating optional strings
    fn optional_string_strategy() -> impl Strategy<Value = Option<String>> {
        prop_oneof![
            Just(None),
            "[a-zA-Z0-9_/.-]{1,50}".prop_map(Some),
        ]
    }

    // Strategy for generating optional environment variables
    fn optional_env_strategy() -> impl Strategy<Value = Option<HashMap<String, String>>> {
        prop_oneof![
            Just(None),
            prop::collection::hash_map("[A-Z_]{1,20}", "[a-zA-Z0-9_]{1,30}", 0..5).prop_map(Some),
        ]
    }

    // Strategy for generating ConnectionType::Local
    fn local_connection_strategy() -> impl Strategy<Value = ConnectionType> {
        (
            optional_string_strategy(),
            optional_string_strategy(),
            optional_env_strategy(),
        )
            .prop_map(|(shell_path, cwd, env)| ConnectionType::Local {
                shell_path,
                cwd,
                env,
            })
    }

    // Strategy for generating ConnectionType::Ssh
    fn ssh_connection_strategy() -> impl Strategy<Value = ConnectionType> {
        (
            "[a-z0-9.-]{1,50}",
            prop::option::of(1u16..65535),
            optional_string_strategy(),
            optional_string_strategy(),
            optional_string_strategy(),
        )
            .prop_map(|(host, port, user, identity_file, password)| ConnectionType::Ssh {
                host,
                port,
                user,
                identity_file,
                password,
            })
    }

    // Strategy for generating ConnectionType
    fn connection_type_strategy() -> impl Strategy<Value = ConnectionType> {
        prop_oneof![local_connection_strategy(), ssh_connection_strategy(),]
    }

    // Strategy for generating SessionStatus
    fn session_status_strategy() -> impl Strategy<Value = SessionStatus> {
        prop_oneof![
            Just(SessionStatus::Init),
            Just(SessionStatus::Connecting),
            Just(SessionStatus::Running),
            Just(SessionStatus::Done),
            Just(SessionStatus::Error),
        ]
    }

    // Strategy for generating SessionInfo
    fn session_info_strategy() -> impl Strategy<Value = SessionInfo> {
        (
            "[a-f0-9-]{36}",
            connection_type_strategy(),
            session_status_strategy(),
            optional_string_strategy(),
            optional_string_strategy(),
            prop::option::of(-128i32..128),
            0u64..u64::MAX,
        )
            .prop_map(
                |(id, connection_type, status, title, cwd, exit_code, created_at)| SessionInfo {
                    id,
                    connection_type,
                    status,
                    title,
                    cwd,
                    exit_code,
                    created_at,
                },
            )
    }

    // Strategy for generating CreateSessionRequest
    fn create_session_request_strategy() -> impl Strategy<Value = CreateSessionRequest> {
        (connection_type_strategy(), term_size_strategy())
            .prop_map(|(connection, term_size)| CreateSessionRequest {
                connection,
                term_size,
            })
    }

    // Strategy for generating InputRequest
    fn input_request_strategy() -> impl Strategy<Value = InputRequest> {
        ("[a-f0-9-]{36}", "[A-Za-z0-9+/=]{0,100}")
            .prop_map(|(session_id, data)| InputRequest { session_id, data })
    }

    // Strategy for generating ResizeRequest
    fn resize_request_strategy() -> impl Strategy<Value = ResizeRequest> {
        ("[a-f0-9-]{36}", term_size_strategy())
            .prop_map(|(session_id, term_size)| ResizeRequest {
                session_id,
                term_size,
            })
    }

    // Strategy for generating OutputNotification
    fn output_notification_strategy() -> impl Strategy<Value = OutputNotification> {
        ("[a-f0-9-]{36}", "[A-Za-z0-9+/=]{0,100}")
            .prop_map(|(session_id, data)| OutputNotification { session_id, data })
    }

    // Strategy for generating SessionStatusNotification
    fn session_status_notification_strategy() -> impl Strategy<Value = SessionStatusNotification> {
        (
            "[a-f0-9-]{36}",
            session_status_strategy(),
            prop::option::of(-128i32..128),
        )
            .prop_map(|(session_id, status, exit_code)| SessionStatusNotification {
                session_id,
                status,
                exit_code,
            })
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Feature: terminal-plugin, Property 2: RPC 请求往返一致性
        /// *对于任意*有效的 TermSize，序列化后再反序列化应产生等价对象
        #[test]
        fn prop_term_size_roundtrip(size in term_size_strategy()) {
            let json = serde_json::to_string(&size).unwrap();
            let deserialized: TermSize = serde_json::from_str(&json).unwrap();
            prop_assert_eq!(size, deserialized);
        }

        /// Feature: terminal-plugin, Property 2: RPC 请求往返一致性
        /// *对于任意*有效的 ConnectionType，序列化后再反序列化应产生等价对象
        #[test]
        fn prop_connection_type_roundtrip(conn in connection_type_strategy()) {
            let json = serde_json::to_string(&conn).unwrap();
            let deserialized: ConnectionType = serde_json::from_str(&json).unwrap();
            prop_assert_eq!(conn, deserialized);
        }

        /// Feature: terminal-plugin, Property 2: RPC 请求往返一致性
        /// *对于任意*有效的 SessionStatus，序列化后再反序列化应产生等价对象
        #[test]
        fn prop_session_status_roundtrip(status in session_status_strategy()) {
            let json = serde_json::to_string(&status).unwrap();
            let deserialized: SessionStatus = serde_json::from_str(&json).unwrap();
            prop_assert_eq!(status, deserialized);
        }

        /// Feature: terminal-plugin, Property 2: RPC 请求往返一致性
        /// *对于任意*有效的 CreateSessionRequest，序列化后再反序列化应产生等价对象
        #[test]
        fn prop_create_session_request_roundtrip(req in create_session_request_strategy()) {
            let json = serde_json::to_string(&req).unwrap();
            let deserialized: CreateSessionRequest = serde_json::from_str(&json).unwrap();
            prop_assert_eq!(req.term_size, deserialized.term_size);
            prop_assert_eq!(req.connection, deserialized.connection);
        }

        /// Feature: terminal-plugin, Property 2: RPC 请求往返一致性
        /// *对于任意*有效的 InputRequest，序列化后再反序列化应产生等价对象
        #[test]
        fn prop_input_request_roundtrip(req in input_request_strategy()) {
            let json = serde_json::to_string(&req).unwrap();
            let deserialized: InputRequest = serde_json::from_str(&json).unwrap();
            prop_assert_eq!(req.session_id, deserialized.session_id);
            prop_assert_eq!(req.data, deserialized.data);
        }

        /// Feature: terminal-plugin, Property 2: RPC 请求往返一致性
        /// *对于任意*有效的 ResizeRequest，序列化后再反序列化应产生等价对象
        #[test]
        fn prop_resize_request_roundtrip(req in resize_request_strategy()) {
            let json = serde_json::to_string(&req).unwrap();
            let deserialized: ResizeRequest = serde_json::from_str(&json).unwrap();
            prop_assert_eq!(req.session_id, deserialized.session_id);
            prop_assert_eq!(req.term_size, deserialized.term_size);
        }

        /// Feature: terminal-plugin, Property 2: RPC 请求往返一致性
        /// *对于任意*有效的 OutputNotification，序列化后再反序列化应产生等价对象
        #[test]
        fn prop_output_notification_roundtrip(notif in output_notification_strategy()) {
            let json = serde_json::to_string(&notif).unwrap();
            let deserialized: OutputNotification = serde_json::from_str(&json).unwrap();
            prop_assert_eq!(notif.session_id, deserialized.session_id);
            prop_assert_eq!(notif.data, deserialized.data);
        }

        /// Feature: terminal-plugin, Property 2: RPC 请求往返一致性
        /// *对于任意*有效的 SessionStatusNotification，序列化后再反序列化应产生等价对象
        #[test]
        fn prop_session_status_notification_roundtrip(notif in session_status_notification_strategy()) {
            let json = serde_json::to_string(&notif).unwrap();
            let deserialized: SessionStatusNotification = serde_json::from_str(&json).unwrap();
            prop_assert_eq!(notif.session_id, deserialized.session_id);
            prop_assert_eq!(notif.status, deserialized.status);
            prop_assert_eq!(notif.exit_code, deserialized.exit_code);
        }
    }
}
