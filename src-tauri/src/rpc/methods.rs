//! RPC 方法注册和调用
//!
//! 实现 JSON-RPC 方法的注册和分发。

use super::server::NotificationSender;
use super::types::{
    CloseSessionRequest, CreateSessionRequest, CreateSessionResponse, GetSessionRequest,
    InputRequest, JsonRpcError, JsonRpcResponse, ResizeRequest, SessionInfo,
};
use crate::pty::PtyManager;

/// RPC 方法处理器
pub struct RpcMethods {
    pty_manager: PtyManager,
}

impl RpcMethods {
    /// 创建新的方法处理器
    pub fn new() -> Self {
        Self {
            pty_manager: PtyManager::new(),
        }
    }

    /// 创建带通知发送器的方法处理器
    pub fn with_notification_sender(notification_sender: NotificationSender) -> Self {
        Self {
            pty_manager: PtyManager::with_notification_sender(notification_sender),
        }
    }

    /// 设置通知发送器
    pub fn set_notification_sender(&mut self, sender: NotificationSender) {
        self.pty_manager.set_notification_sender(sender);
    }

    /// 调用指定方法
    pub async fn call(
        &mut self,
        method: &str,
        params: Option<serde_json::Value>,
        id: serde_json::Value,
    ) -> JsonRpcResponse {
        match method {
            "session.create" => self.session_create(params, id).await,
            "session.input" => self.session_input(params, id).await,
            "session.resize" => self.session_resize(params, id).await,
            "session.close" => self.session_close(params, id).await,
            "session.list" => self.session_list(id).await,
            "session.get" => self.session_get(params, id).await,
            _ => JsonRpcResponse::error(id, JsonRpcError::method_not_found(method)),
        }
    }

    /// 创建会话
    async fn session_create(
        &mut self,
        params: Option<serde_json::Value>,
        id: serde_json::Value,
    ) -> JsonRpcResponse {
        let params = match params {
            Some(p) => p,
            None => {
                return JsonRpcResponse::error(id, JsonRpcError::invalid_params("缺少参数"));
            }
        };

        let request: CreateSessionRequest = match serde_json::from_value(params) {
            Ok(r) => r,
            Err(e) => {
                return JsonRpcResponse::error(
                    id,
                    JsonRpcError::invalid_params(format!("参数解析错误: {}", e)),
                );
            }
        };

        match self.pty_manager.create_session(request).await {
            Ok(session_id) => {
                let response = CreateSessionResponse { session_id };
                JsonRpcResponse::success(id, serde_json::to_value(response).unwrap())
            }
            Err(e) => JsonRpcResponse::error(id, JsonRpcError::internal_error(e.to_string())),
        }
    }

    /// 发送输入
    async fn session_input(
        &mut self,
        params: Option<serde_json::Value>,
        id: serde_json::Value,
    ) -> JsonRpcResponse {
        let params = match params {
            Some(p) => p,
            None => {
                return JsonRpcResponse::error(id, JsonRpcError::invalid_params("缺少参数"));
            }
        };

        let request: InputRequest = match serde_json::from_value(params) {
            Ok(r) => r,
            Err(e) => {
                return JsonRpcResponse::error(
                    id,
                    JsonRpcError::invalid_params(format!("参数解析错误: {}", e)),
                );
            }
        };

        match self.pty_manager.send_input(&request.session_id, &request.data).await {
            Ok(()) => JsonRpcResponse::success(id, serde_json::Value::Null),
            Err(e) => JsonRpcResponse::error(id, JsonRpcError::internal_error(e.to_string())),
        }
    }

    /// 调整大小
    async fn session_resize(
        &mut self,
        params: Option<serde_json::Value>,
        id: serde_json::Value,
    ) -> JsonRpcResponse {
        let params = match params {
            Some(p) => p,
            None => {
                return JsonRpcResponse::error(id, JsonRpcError::invalid_params("缺少参数"));
            }
        };

        let request: ResizeRequest = match serde_json::from_value(params) {
            Ok(r) => r,
            Err(e) => {
                return JsonRpcResponse::error(
                    id,
                    JsonRpcError::invalid_params(format!("参数解析错误: {}", e)),
                );
            }
        };

        match self
            .pty_manager
            .resize_session(&request.session_id, request.term_size)
            .await
        {
            Ok(()) => JsonRpcResponse::success(id, serde_json::Value::Null),
            Err(e) => JsonRpcResponse::error(id, JsonRpcError::internal_error(e.to_string())),
        }
    }

    /// 关闭会话
    async fn session_close(
        &mut self,
        params: Option<serde_json::Value>,
        id: serde_json::Value,
    ) -> JsonRpcResponse {
        let params = match params {
            Some(p) => p,
            None => {
                return JsonRpcResponse::error(id, JsonRpcError::invalid_params("缺少参数"));
            }
        };

        let request: CloseSessionRequest = match serde_json::from_value(params) {
            Ok(r) => r,
            Err(e) => {
                return JsonRpcResponse::error(
                    id,
                    JsonRpcError::invalid_params(format!("参数解析错误: {}", e)),
                );
            }
        };

        match self.pty_manager.close_session(&request.session_id).await {
            Ok(()) => JsonRpcResponse::success(id, serde_json::Value::Null),
            Err(e) => JsonRpcResponse::error(id, JsonRpcError::internal_error(e.to_string())),
        }
    }

    /// 列出所有会话
    async fn session_list(&self, id: serde_json::Value) -> JsonRpcResponse {
        let sessions = self.pty_manager.list_sessions().await;
        JsonRpcResponse::success(id, serde_json::to_value(sessions).unwrap())
    }

    /// 获取会话信息
    async fn session_get(
        &self,
        params: Option<serde_json::Value>,
        id: serde_json::Value,
    ) -> JsonRpcResponse {
        let params = match params {
            Some(p) => p,
            None => {
                return JsonRpcResponse::error(id, JsonRpcError::invalid_params("缺少参数"));
            }
        };

        let request: GetSessionRequest = match serde_json::from_value(params) {
            Ok(r) => r,
            Err(e) => {
                return JsonRpcResponse::error(
                    id,
                    JsonRpcError::invalid_params(format!("参数解析错误: {}", e)),
                );
            }
        };

        match self.pty_manager.get_session(&request.session_id).await {
            Some(session) => JsonRpcResponse::success(id, serde_json::to_value(session).unwrap()),
            None => JsonRpcResponse::error(
                id,
                JsonRpcError::invalid_params(format!("会话不存在: {}", request.session_id)),
            ),
        }
    }
}

impl Default for RpcMethods {
    fn default() -> Self {
        Self::new()
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_method_not_found() {
        let mut methods = RpcMethods::new();
        let response = methods.call("unknown.method", None, serde_json::json!(1)).await;
        
        assert!(response.error.is_some());
        let error = response.error.unwrap();
        assert_eq!(error.code, -32601); // Method not found
    }

    #[tokio::test]
    async fn test_missing_params() {
        let mut methods = RpcMethods::new();
        let response = methods.call("session.create", None, serde_json::json!(1)).await;
        
        assert!(response.error.is_some());
        let error = response.error.unwrap();
        assert_eq!(error.code, -32602); // Invalid params
    }

    #[tokio::test]
    async fn test_invalid_params() {
        let mut methods = RpcMethods::new();
        let response = methods.call(
            "session.create",
            Some(serde_json::json!({"invalid": "params"})),
            serde_json::json!(1)
        ).await;
        
        assert!(response.error.is_some());
        let error = response.error.unwrap();
        assert_eq!(error.code, -32602); // Invalid params
    }
}

/// Property-based tests for RPC error responses
/// Feature: terminal-plugin, Property 3: RPC 错误响应格式
/// **验证: 需求 3.5**
#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    // Strategy for generating random method names (including invalid ones)
    fn method_name_strategy() -> impl Strategy<Value = String> {
        prop_oneof![
            // Valid method names
            Just("session.create".to_string()),
            Just("session.input".to_string()),
            Just("session.resize".to_string()),
            Just("session.close".to_string()),
            Just("session.list".to_string()),
            Just("session.get".to_string()),
            // Invalid method names
            "[a-z.]{1,30}".prop_map(|s| s),
        ]
    }

    // Strategy for generating invalid params
    fn invalid_params_strategy() -> impl Strategy<Value = Option<serde_json::Value>> {
        prop_oneof![
            Just(None),
            Just(Some(serde_json::json!({}))),
            Just(Some(serde_json::json!({"random": "value"}))),
            Just(Some(serde_json::json!(null))),
            Just(Some(serde_json::json!([1, 2, 3]))),
            Just(Some(serde_json::json!("string"))),
            Just(Some(serde_json::json!(123))),
        ]
    }

    // Strategy for generating request IDs
    fn request_id_strategy() -> impl Strategy<Value = serde_json::Value> {
        prop_oneof![
            (1i64..1000).prop_map(|n| serde_json::json!(n)),
            "[a-z0-9-]{1,20}".prop_map(|s| serde_json::json!(s)),
            Just(serde_json::Value::Null),
        ]
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Feature: terminal-plugin, Property 3: RPC 错误响应格式
        /// *对于任意*无效的方法名，服务器应返回包含 error 字段的响应，错误码为 -32601
        #[test]
        fn prop_unknown_method_returns_error(
            method in "[a-z]{5,15}\\.[a-z]{5,15}",
            id in request_id_strategy()
        ) {
            // Skip known valid methods
            let valid_methods = ["session.create", "session.input", "session.resize", 
                                 "session.close", "session.list", "session.get"];
            if valid_methods.contains(&method.as_str()) {
                return Ok(());
            }

            let rt = tokio::runtime::Runtime::new().unwrap();
            let response = rt.block_on(async {
                let mut methods = RpcMethods::new();
                methods.call(&method, None, id.clone()).await
            });

            // Response must have error field
            prop_assert!(response.error.is_some(), "Response should have error for unknown method");
            
            let error = response.error.unwrap();
            // Error code must be -32601 (Method not found)
            prop_assert_eq!(error.code, -32601, "Error code should be -32601 for method not found");
            
            // Response must have correct jsonrpc version
            prop_assert_eq!(response.jsonrpc, "2.0", "jsonrpc version should be 2.0");
            
            // Response must not have result
            prop_assert!(response.result.is_none(), "Response should not have result for error");
        }

        /// Feature: terminal-plugin, Property 3: RPC 错误响应格式
        /// *对于任意*缺少必需参数的请求，服务器应返回包含 error 字段的响应，错误码为 -32602
        #[test]
        fn prop_missing_params_returns_error(
            method in prop_oneof![
                Just("session.create"),
                Just("session.input"),
                Just("session.resize"),
                Just("session.close"),
                Just("session.get"),
            ],
            id in request_id_strategy()
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            let response = rt.block_on(async {
                let mut methods = RpcMethods::new();
                methods.call(method, None, id.clone()).await
            });

            // Response must have error field
            prop_assert!(response.error.is_some(), "Response should have error for missing params");
            
            let error = response.error.unwrap();
            // Error code must be -32602 (Invalid params)
            prop_assert_eq!(error.code, -32602, "Error code should be -32602 for invalid params");
            
            // Response must have correct jsonrpc version
            prop_assert_eq!(response.jsonrpc, "2.0", "jsonrpc version should be 2.0");
        }

        /// Feature: terminal-plugin, Property 3: RPC 错误响应格式
        /// *对于任意*无效参数格式的请求，服务器应返回包含 error 字段的响应，错误码为 -32602
        #[test]
        fn prop_invalid_params_returns_error(
            method in prop_oneof![
                Just("session.create"),
                Just("session.input"),
                Just("session.resize"),
                Just("session.close"),
                Just("session.get"),
            ],
            params in invalid_params_strategy(),
            id in request_id_strategy()
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            let response = rt.block_on(async {
                let mut methods = RpcMethods::new();
                methods.call(method, params, id.clone()).await
            });

            // Response must have error field (either missing params or invalid params)
            prop_assert!(response.error.is_some(), "Response should have error for invalid params");
            
            let error = response.error.unwrap();
            // Error code must be -32602 (Invalid params) or -32603 (Internal error for session not found)
            prop_assert!(
                error.code == -32602 || error.code == -32603,
                "Error code should be -32602 or -32603, got {}",
                error.code
            );
            
            // Response must have correct jsonrpc version
            prop_assert_eq!(response.jsonrpc, "2.0", "jsonrpc version should be 2.0");
        }

        /// Feature: terminal-plugin, Property 3: RPC 错误响应格式
        /// *对于任意*错误响应，都应该符合 JSON-RPC 2.0 规范
        #[test]
        fn prop_error_response_format(
            code in prop_oneof![
                Just(-32700i32), // Parse error
                Just(-32600),    // Invalid request
                Just(-32601),    // Method not found
                Just(-32602),    // Invalid params
                Just(-32603),    // Internal error
            ],
            message in "[a-zA-Z0-9 ]{1,50}",
            id in request_id_strategy()
        ) {
            let error = JsonRpcError {
                code,
                message: message.clone(),
                data: None,
            };
            let response = JsonRpcResponse::error(id.clone(), error);

            // Verify response structure
            prop_assert_eq!(&response.jsonrpc, "2.0");
            prop_assert!(response.result.is_none());
            prop_assert!(response.error.is_some());
            
            let err = response.error.as_ref().unwrap();
            prop_assert_eq!(err.code, code);
            prop_assert_eq!(&err.message, &message);
            
            // Verify serialization produces valid JSON
            let json = serde_json::to_string(&response);
            prop_assert!(json.is_ok(), "Response should serialize to valid JSON");
            
            let json_str = json.unwrap();
            prop_assert!(json_str.contains("\"jsonrpc\":\"2.0\""));
            prop_assert!(json_str.contains("\"error\""));
            let code_str = format!("\"code\":{}", code);
            prop_assert!(json_str.contains(&code_str), "JSON should contain error code");
        }
    }
}
