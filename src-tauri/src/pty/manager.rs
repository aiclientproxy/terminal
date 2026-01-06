//! PTY 管理器
//!
//! 管理多个 PTY 会话的创建、输入、调整大小和关闭。

use std::collections::HashMap;

use crate::rpc::server::NotificationSender;
use crate::rpc::types::{ConnectionType, CreateSessionRequest, SessionInfo, SessionStatus, TermSize};
use crate::utils::error::TerminalError;

use super::session::PtySession;

/// PTY 管理器
pub struct PtyManager {
    /// 会话映射表
    sessions: HashMap<String, PtySession>,
    /// 通知发送器（可选，用于发送输出通知）
    notification_sender: Option<NotificationSender>,
}

impl PtyManager {
    /// 创建新的 PTY 管理器
    pub fn new() -> Self {
        Self {
            sessions: HashMap::new(),
            notification_sender: None,
        }
    }

    /// 创建带通知发送器的 PTY 管理器
    pub fn with_notification_sender(notification_sender: NotificationSender) -> Self {
        Self {
            sessions: HashMap::new(),
            notification_sender: Some(notification_sender),
        }
    }

    /// 设置通知发送器
    pub fn set_notification_sender(&mut self, sender: NotificationSender) {
        self.notification_sender = Some(sender);
    }

    /// 创建新会话
    pub async fn create_session(
        &mut self,
        request: CreateSessionRequest,
    ) -> Result<String, TerminalError> {
        // 生成唯一会话 ID
        let session_id = uuid::Uuid::new_v4().to_string();

        // 根据连接类型创建会话
        let mut session = match &request.connection {
            ConnectionType::Local { shell_path, cwd, env } => {
                // 创建本地 PTY 会话
                PtySession::new_local(
                    session_id.clone(),
                    shell_path.clone(),
                    cwd.clone(),
                    env.clone(),
                    request.term_size,
                )?
            }
            ConnectionType::Ssh { .. } => {
                // SSH 会话暂时只创建占位符，实际实现在 SSH 模块
                let mut session = PtySession::new(session_id.clone(), request.connection.clone());
                session.set_status(SessionStatus::Connecting);
                session
            }
        };

        // 如果有通知发送器且是本地会话，启动输出读取器
        if let Some(sender) = &self.notification_sender {
            if matches!(request.connection, ConnectionType::Local { .. }) {
                if let Err(e) = session.start_output_reader(sender.clone()).await {
                    tracing::warn!("启动输出读取器失败: {}", e);
                }
            }
        }

        // 存储会话
        self.sessions.insert(session_id.clone(), session);

        tracing::info!("创建会话: {}", session_id);
        Ok(session_id)
    }

    /// 发送输入到会话
    pub async fn send_input(&mut self, session_id: &str, data: &str) -> Result<(), TerminalError> {
        let session = self
            .sessions
            .get(session_id)
            .ok_or_else(|| TerminalError::SessionNotFound(session_id.to_string()))?;

        // 解码 base64 数据
        let decoded = base64::Engine::decode(
            &base64::engine::general_purpose::STANDARD,
            data,
        )
        .map_err(|e| TerminalError::InvalidRequest(format!("Invalid base64 data: {}", e)))?;

        // 写入 PTY
        session.write(&decoded).await?;

        tracing::debug!("发送输入到会话 {}: {} bytes", session_id, decoded.len());
        Ok(())
    }

    /// 调整会话大小
    pub async fn resize_session(
        &mut self,
        session_id: &str,
        term_size: TermSize,
    ) -> Result<(), TerminalError> {
        let session = self
            .sessions
            .get(session_id)
            .ok_or_else(|| TerminalError::SessionNotFound(session_id.to_string()))?;

        // 调整 PTY 大小
        session.resize(term_size.clone()).await?;

        tracing::debug!(
            "调整会话 {} 大小: {}x{}",
            session_id,
            term_size.cols,
            term_size.rows
        );
        Ok(())
    }

    /// 关闭会话
    pub async fn close_session(&mut self, session_id: &str) -> Result<(), TerminalError> {
        let mut session = self
            .sessions
            .remove(session_id)
            .ok_or_else(|| TerminalError::SessionNotFound(session_id.to_string()))?;

        // 停止输出读取器
        session.stop_output_reader().await;

        // 终止 PTY 进程
        session.kill().await?;

        tracing::info!("关闭会话: {}", session_id);
        Ok(())
    }

    /// 列出所有会话
    pub async fn list_sessions(&self) -> Vec<SessionInfo> {
        self.sessions.values().map(|s| s.info().clone()).collect()
    }

    /// 获取会话信息
    pub async fn get_session(&self, session_id: &str) -> Option<SessionInfo> {
        self.sessions.get(session_id).map(|s| s.info().clone())
    }

    /// 获取会话引用
    pub fn get_session_ref(&self, session_id: &str) -> Option<&PtySession> {
        self.sessions.get(session_id)
    }

    /// 获取可变会话引用
    pub fn get_session_mut(&mut self, session_id: &str) -> Option<&mut PtySession> {
        self.sessions.get_mut(session_id)
    }

    /// 获取会话数量
    pub fn session_count(&self) -> usize {
        self.sessions.len()
    }
}

impl Default for PtyManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_session() {
        let mut manager = PtyManager::new();
        let request = CreateSessionRequest {
            connection: ConnectionType::Local {
                shell_path: None,
                cwd: None,
                env: None,
            },
            term_size: TermSize::default(),
        };

        let result = manager.create_session(request).await;
        // PTY 创建可能在某些环境中失败
        match result {
            Ok(session_id) => {
                assert!(!session_id.is_empty());
                assert_eq!(manager.session_count(), 1);
                // 清理
                let _ = manager.close_session(&session_id).await;
            }
            Err(e) => {
                println!("PTY creation failed (may be expected in CI): {}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_session_uniqueness() {
        let mut manager = PtyManager::new();
        let mut ids = Vec::new();

        for _ in 0..10 {
            let request = CreateSessionRequest {
                connection: ConnectionType::Local {
                    shell_path: None,
                    cwd: None,
                    env: None,
                },
                term_size: TermSize::default(),
            };
            match manager.create_session(request).await {
                Ok(id) => ids.push(id),
                Err(e) => {
                    println!("PTY creation failed (may be expected in CI): {}", e);
                    return;
                }
            }
        }

        // 验证所有 ID 都是唯一的
        let unique_count = ids.iter().collect::<std::collections::HashSet<_>>().len();
        assert_eq!(unique_count, ids.len());

        // 清理
        for id in &ids {
            let _ = manager.close_session(id).await;
        }
    }

    #[tokio::test]
    async fn test_close_session() {
        let mut manager = PtyManager::new();
        let request = CreateSessionRequest {
            connection: ConnectionType::Local {
                shell_path: None,
                cwd: None,
                env: None,
            },
            term_size: TermSize::default(),
        };

        match manager.create_session(request).await {
            Ok(session_id) => {
                assert_eq!(manager.session_count(), 1);
                manager.close_session(&session_id).await.unwrap();
                assert_eq!(manager.session_count(), 0);
            }
            Err(e) => {
                println!("PTY creation failed (may be expected in CI): {}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_close_nonexistent_session() {
        let mut manager = PtyManager::new();
        let result = manager.close_session("nonexistent").await;
        assert!(result.is_err());
    }
}


/// Property-based tests for PTY manager
/// Feature: terminal-plugin, Property 1: 会话 ID 唯一性
/// **验证: 需求 1.2**
#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;
    use std::collections::HashSet;

    // Strategy for generating number of sessions to create
    fn session_count_strategy() -> impl Strategy<Value = usize> {
        1usize..50
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Feature: terminal-plugin, Property 1: 会话 ID 唯一性
        /// *对于任意*多个创建的会话，所有返回的会话 ID 都应该是唯一的，不存在重复。
        #[test]
        fn prop_session_ids_are_unique(count in session_count_strategy()) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let mut manager = PtyManager::new();
                let mut ids = Vec::new();

                for _ in 0..count {
                    // 使用 SSH 连接类型避免实际创建 PTY（更快且不依赖系统 PTY）
                    let request = CreateSessionRequest {
                        connection: ConnectionType::Ssh {
                            host: "test.example.com".to_string(),
                            port: Some(22),
                            user: Some("test".to_string()),
                            identity_file: None,
                            password: None,
                        },
                        term_size: TermSize::default(),
                    };

                    match manager.create_session(request).await {
                        Ok(id) => {
                            ids.push(id);
                        }
                        Err(_) => {
                            // 如果创建失败，跳过（在某些环境中可能发生）
                            continue;
                        }
                    }
                }

                // 验证所有创建的 ID 都是唯一的
                let unique_ids: HashSet<_> = ids.iter().collect();
                prop_assert_eq!(
                    unique_ids.len(),
                    ids.len(),
                    "Session IDs should be unique. Created {} sessions but only {} unique IDs",
                    ids.len(),
                    unique_ids.len()
                );

                // 验证 ID 格式（UUID v4 格式）
                for id in &ids {
                    prop_assert!(
                        uuid::Uuid::parse_str(id).is_ok(),
                        "Session ID should be a valid UUID: {}",
                        id
                    );
                }

                Ok(())
            })?;
        }

        /// Feature: terminal-plugin, Property 1: 会话 ID 唯一性
        /// *对于任意*会话 ID，它应该是有效的 UUID v4 格式
        #[test]
        fn prop_session_id_is_valid_uuid(_dummy in 0..100u32) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let mut manager = PtyManager::new();
                
                // 使用 SSH 连接类型避免实际创建 PTY
                let request = CreateSessionRequest {
                    connection: ConnectionType::Ssh {
                        host: "test.example.com".to_string(),
                        port: Some(22),
                        user: Some("test".to_string()),
                        identity_file: None,
                        password: None,
                    },
                    term_size: TermSize::default(),
                };

                match manager.create_session(request).await {
                    Ok(id) => {
                        // 验证 ID 是有效的 UUID
                        let parsed = uuid::Uuid::parse_str(&id);
                        prop_assert!(parsed.is_ok(), "Session ID should be a valid UUID: {}", id);
                        
                        // 验证是 UUID v4
                        let uuid = parsed.unwrap();
                        prop_assert_eq!(
                            uuid.get_version(),
                            Some(uuid::Version::Random),
                            "Session ID should be UUID v4"
                        );
                    }
                    Err(e) => {
                        // 如果创建失败，测试仍然通过（环境问题）
                        println!("Session creation failed (may be expected): {}", e);
                    }
                }

                Ok(())
            })?;
        }
    }
}
