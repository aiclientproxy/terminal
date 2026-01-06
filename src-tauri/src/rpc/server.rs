//! RPC 服务器实现
//!
//! 通过 stdin/stdout 实现 JSON-RPC 2.0 通信。

use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::{mpsc, Mutex};

use super::methods::RpcMethods;
use super::types::{JsonRpcNotification, JsonRpcRequest, JsonRpcResponse};

/// 通知发送器，可以克隆并在多个地方使用
#[derive(Clone)]
pub struct NotificationSender {
    tx: mpsc::UnboundedSender<JsonRpcNotification>,
}

impl NotificationSender {
    /// 创建新的通知发送器（用于测试）
    #[cfg(test)]
    pub fn new_for_test(tx: mpsc::UnboundedSender<JsonRpcNotification>) -> Self {
        Self { tx }
    }

    /// 发送通知
    pub fn send(&self, notification: JsonRpcNotification) -> Result<(), mpsc::error::SendError<JsonRpcNotification>> {
        self.tx.send(notification)
    }

    /// 发送终端输出通知
    pub fn send_output(&self, session_id: &str, data: &str) -> Result<(), mpsc::error::SendError<JsonRpcNotification>> {
        let notification = JsonRpcNotification {
            jsonrpc: "2.0".to_string(),
            method: "terminal.output".to_string(),
            params: Some(serde_json::json!({
                "session_id": session_id,
                "data": data
            })),
        };
        self.send(notification)
    }

    /// 发送会话状态变更通知
    pub fn send_status(&self, session_id: &str, status: &str, exit_code: Option<i32>) -> Result<(), mpsc::error::SendError<JsonRpcNotification>> {
        let mut params = serde_json::json!({
            "session_id": session_id,
            "status": status
        });
        if let Some(code) = exit_code {
            params["exit_code"] = serde_json::json!(code);
        }
        let notification = JsonRpcNotification {
            jsonrpc: "2.0".to_string(),
            method: "session.status".to_string(),
            params: Some(params),
        };
        self.send(notification)
    }

    /// 发送工作目录变更通知
    pub fn send_cwd(&self, session_id: &str, cwd: &str) -> Result<(), mpsc::error::SendError<JsonRpcNotification>> {
        let notification = JsonRpcNotification {
            jsonrpc: "2.0".to_string(),
            method: "session.cwd".to_string(),
            params: Some(serde_json::json!({
                "session_id": session_id,
                "cwd": cwd
            })),
        };
        self.send(notification)
    }

    /// 发送会话标题变更通知
    pub fn send_title(&self, session_id: &str, title: &str) -> Result<(), mpsc::error::SendError<JsonRpcNotification>> {
        let notification = JsonRpcNotification {
            jsonrpc: "2.0".to_string(),
            method: "session.title".to_string(),
            params: Some(serde_json::json!({
                "session_id": session_id,
                "title": title
            })),
        };
        self.send(notification)
    }

    /// 发送剪贴板内容通知
    pub fn send_clipboard(&self, session_id: &str, content: &str) -> Result<(), mpsc::error::SendError<JsonRpcNotification>> {
        let notification = JsonRpcNotification {
            jsonrpc: "2.0".to_string(),
            method: "session.clipboard".to_string(),
            params: Some(serde_json::json!({
                "session_id": session_id,
                "content": content
            })),
        };
        self.send(notification)
    }
}

/// RPC 服务器
pub struct RpcServer {
    methods: Arc<Mutex<RpcMethods>>,
    notification_rx: Arc<Mutex<mpsc::UnboundedReceiver<JsonRpcNotification>>>,
    notification_sender: NotificationSender,
}

impl RpcServer {
    /// 创建新的 RPC 服务器
    pub fn new() -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        let notification_sender = NotificationSender { tx };
        
        // 创建带通知发送器的 RpcMethods
        let methods = RpcMethods::with_notification_sender(notification_sender.clone());
        
        Self {
            methods: Arc::new(Mutex::new(methods)),
            notification_rx: Arc::new(Mutex::new(rx)),
            notification_sender,
        }
    }

    /// 获取通知发送器
    pub fn notification_sender(&self) -> NotificationSender {
        self.notification_sender.clone()
    }

    /// 运行 RPC 服务器
    pub async fn run(&self) -> anyhow::Result<()> {
        let stdin = tokio::io::stdin();
        let stdout = Arc::new(Mutex::new(tokio::io::stdout()));
        let mut reader = BufReader::new(stdin);

        let mut line = String::new();

        // 启动通知发送任务
        let notification_rx = self.notification_rx.clone();
        let stdout_for_notifications = stdout.clone();
        let notification_task = tokio::spawn(async move {
            let mut rx = notification_rx.lock().await;
            while let Some(notification) = rx.recv().await {
                let mut stdout = stdout_for_notifications.lock().await;
                if let Ok(json) = serde_json::to_string(&notification) {
                    let _ = stdout.write_all(json.as_bytes()).await;
                    let _ = stdout.write_all(b"\n").await;
                    let _ = stdout.flush().await;
                }
            }
        });

        loop {
            line.clear();
            let bytes_read = reader.read_line(&mut line).await?;

            if bytes_read == 0 {
                // EOF，退出
                tracing::info!("stdin 关闭，退出");
                break;
            }

            let line_trimmed = line.trim();
            if line_trimmed.is_empty() {
                continue;
            }

            // 解析 JSON-RPC 请求
            let response = self.handle_request(line_trimmed).await;

            // 发送响应
            let response_json = serde_json::to_string(&response)?;
            let mut stdout = stdout.lock().await;
            stdout.write_all(response_json.as_bytes()).await?;
            stdout.write_all(b"\n").await?;
            stdout.flush().await?;
        }

        // 取消通知任务
        notification_task.abort();

        Ok(())
    }

    /// 处理单个请求
    async fn handle_request(&self, line: &str) -> JsonRpcResponse {
        // 解析 JSON
        let request: JsonRpcRequest = match serde_json::from_str(line) {
            Ok(req) => req,
            Err(e) => {
                return JsonRpcResponse::error(
                    serde_json::Value::Null,
                    super::types::JsonRpcError::parse_error(format!("JSON 解析错误: {}", e)),
                );
            }
        };

        // 验证 JSON-RPC 版本
        if request.jsonrpc != "2.0" {
            return JsonRpcResponse::error(
                request.id,
                super::types::JsonRpcError::invalid_request("无效的 JSON-RPC 版本"),
            );
        }

        // 调用方法
        let mut methods = self.methods.lock().await;
        methods.call(&request.method, request.params, request.id).await
    }

    /// 发送通知（用于异步事件）- 直接发送，不经过通道
    pub async fn send_notification(&self, notification: JsonRpcNotification) -> anyhow::Result<()> {
        self.notification_sender.send(notification)
            .map_err(|e| anyhow::anyhow!("发送通知失败: {}", e))
    }
}

impl Default for RpcServer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_notification_sender_clone() {
        let server = RpcServer::new();
        let sender1 = server.notification_sender();
        let sender2 = sender1.clone();
        
        // Both senders should be able to send
        assert!(sender1.send_output("test-session", "dGVzdA==").is_ok());
        assert!(sender2.send_status("test-session", "running", None).is_ok());
    }

    #[test]
    fn test_notification_sender_output() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let sender = NotificationSender { tx };
        
        sender.send_output("session-123", "SGVsbG8=").unwrap();
        
        let notification = rx.try_recv().unwrap();
        assert_eq!(notification.method, "terminal.output");
        assert!(notification.params.is_some());
        
        let params = notification.params.unwrap();
        assert_eq!(params["session_id"], "session-123");
        assert_eq!(params["data"], "SGVsbG8=");
    }

    #[test]
    fn test_notification_sender_status() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let sender = NotificationSender { tx };
        
        sender.send_status("session-123", "done", Some(0)).unwrap();
        
        let notification = rx.try_recv().unwrap();
        assert_eq!(notification.method, "session.status");
        
        let params = notification.params.unwrap();
        assert_eq!(params["session_id"], "session-123");
        assert_eq!(params["status"], "done");
        assert_eq!(params["exit_code"], 0);
    }

    #[test]
    fn test_notification_sender_cwd() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let sender = NotificationSender { tx };
        
        sender.send_cwd("session-123", "/home/user").unwrap();
        
        let notification = rx.try_recv().unwrap();
        assert_eq!(notification.method, "session.cwd");
        
        let params = notification.params.unwrap();
        assert_eq!(params["session_id"], "session-123");
        assert_eq!(params["cwd"], "/home/user");
    }

    #[test]
    fn test_notification_sender_title() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let sender = NotificationSender { tx };
        
        sender.send_title("session-123", "vim").unwrap();
        
        let notification = rx.try_recv().unwrap();
        assert_eq!(notification.method, "session.title");
        
        let params = notification.params.unwrap();
        assert_eq!(params["session_id"], "session-123");
        assert_eq!(params["title"], "vim");
    }
}
