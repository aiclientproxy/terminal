//! SSH 会话
//!
//! 管理 SSH PTY 通道，处理输入/输出。

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use russh::client::Msg;
use russh::ChannelMsg;
use tokio::sync::{mpsc, Mutex, RwLock};

use crate::rpc::server::NotificationSender;
use crate::rpc::types::{ConnectionType, SessionInfo, SessionStatus, TermSize};
use crate::utils::error::TerminalError;

use super::client::SshClient;

/// SSH 通道包装器
///
/// 封装 russh Channel，提供线程安全的访问。
struct ChannelWrapper {
    /// 内部通道（使用 client::Msg 类型）
    inner: russh::Channel<Msg>,
}

impl ChannelWrapper {
    fn new(channel: russh::Channel<Msg>) -> Self {
        Self { inner: channel }
    }

    /// 发送数据
    async fn send_data(&self, data: &[u8]) -> Result<(), TerminalError> {
        self.inner.data(data).await.map_err(|e| {
            TerminalError::ChannelError(format!("发送数据失败: {}", e))
        })
    }

    /// 调整 PTY 大小
    async fn resize(&self, cols: u32, rows: u32) -> Result<(), TerminalError> {
        self.inner
            .window_change(cols, rows, 0, 0)
            .await
            .map_err(|e| {
                TerminalError::ChannelError(format!("调整大小失败: {}", e))
            })
    }

    /// 等待消息
    async fn wait(&mut self) -> Option<ChannelMsg> {
        self.inner.wait().await
    }

    /// 发送 EOF
    async fn eof(&self) -> Result<(), TerminalError> {
        self.inner.eof().await.map_err(|e| {
            TerminalError::ChannelError(format!("发送 EOF 失败: {}", e))
        })
    }

    /// 关闭通道
    async fn close(&self) -> Result<(), TerminalError> {
        self.inner.close().await.map_err(|e| {
            TerminalError::ChannelError(format!("关闭通道失败: {}", e))
        })
    }
}

/// SSH 会话
///
/// 封装 SSH 连接和 PTY 通道，提供终端交互功能。
pub struct SshSession {
    /// 会话 ID
    session_id: String,
    /// SSH 客户端
    client: SshClient,
    /// PTY 通道（共享访问）
    channel: Option<Arc<Mutex<ChannelWrapper>>>,
    /// 会话信息
    info: Arc<RwLock<SessionInfo>>,
    /// 输出读取任务句柄
    output_task: Option<tokio::task::JoinHandle<()>>,
    /// 停止信号发送器
    stop_tx: Option<mpsc::Sender<()>>,
}

impl SshSession {
    /// 创建新的 SSH 会话
    pub fn new(
        session_id: String,
        host: String,
        port: Option<u16>,
        user: Option<String>,
        identity_file: Option<String>,
        password: Option<String>,
    ) -> Self {
        let created_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let client = SshClient::from_params(
            host.clone(),
            port,
            user.clone(),
            identity_file.clone(),
            password.clone(),
        );

        let info = SessionInfo {
            id: session_id.clone(),
            connection_type: ConnectionType::Ssh {
                host,
                port,
                user,
                identity_file,
                password,
            },
            status: SessionStatus::Init,
            title: None,
            cwd: None,
            exit_code: None,
            created_at,
        };

        Self {
            session_id,
            client,
            channel: None,
            info: Arc::new(RwLock::new(info)),
            output_task: None,
            stop_tx: None,
        }
    }

    /// 连接并打开 PTY 通道
    pub async fn connect(&mut self, term_size: TermSize) -> Result<(), TerminalError> {
        // 更新状态为连接中
        {
            let mut info = self.info.write().await;
            info.status = SessionStatus::Connecting;
        }

        // 建立 SSH 连接
        self.client.connect().await?;

        // 获取会话句柄
        let handle = self.client.handle_mut().ok_or_else(|| {
            TerminalError::channel_error("打开会话", "无法获取 SSH 会话句柄")
        })?;

        // 打开会话通道
        let channel = handle.channel_open_session().await.map_err(|e| {
            TerminalError::channel_error("打开会话通道", &e.to_string())
        })?;

        // 请求 PTY
        channel
            .request_pty(
                false,                    // want_reply
                "xterm-256color",         // term
                term_size.cols as u32,    // col_width
                term_size.rows as u32,    // row_height
                0,                        // pix_width
                0,                        // pix_height
                &[],                      // terminal_modes
            )
            .await
            .map_err(|e| {
                TerminalError::channel_error("请求 PTY", &e.to_string())
            })?;

        // 请求 shell
        channel.request_shell(false).await.map_err(|e| {
            TerminalError::channel_error("请求 shell", &e.to_string())
        })?;

        // 包装通道
        self.channel = Some(Arc::new(Mutex::new(ChannelWrapper::new(channel))));

        // 更新状态为运行中
        {
            let mut info = self.info.write().await;
            info.status = SessionStatus::Running;
        }

        tracing::info!("SSH 会话已建立: {}", self.session_id);
        Ok(())
    }

    /// 启动输出读取器
    ///
    /// 开始异步读取 SSH 通道输出并通过通知发送到前端。
    pub async fn start_output_reader(
        &mut self,
        notification_sender: NotificationSender,
    ) -> Result<(), TerminalError> {
        let channel = self.channel.clone().ok_or_else(|| {
            TerminalError::ChannelError("通道未打开".to_string())
        })?;

        let session_id = self.session_id.clone();
        let info = self.info.clone();
        let (stop_tx, mut stop_rx) = mpsc::channel::<()>(1);

        // 启动输出读取任务
        let task = tokio::spawn(async move {
            tracing::info!("SSH 输出读取器启动: {}", session_id);

            loop {
                // 使用 select 来同时监听停止信号和通道消息
                tokio::select! {
                    biased;
                    
                    // 检查停止信号（优先级更高）
                    _ = stop_rx.recv() => {
                        tracing::info!("SSH 输出读取器收到停止信号: {}", session_id);
                        break;
                    }
                    
                    // 读取通道消息
                    msg = async {
                        let mut channel_guard = channel.lock().await;
                        channel_guard.wait().await
                    } => {
                        match msg {
                            Some(ChannelMsg::Data { data }) => {
                                // 发送输出通知（base64 编码）
                                let encoded = base64::Engine::encode(
                                    &base64::engine::general_purpose::STANDARD,
                                    &data,
                                );
                                if let Err(e) = notification_sender.send_output(&session_id, &encoded) {
                                    tracing::error!("发送输出通知失败: {}", e);
                                    break;
                                }
                            }
                            Some(ChannelMsg::ExtendedData { data, ext }) => {
                                // stderr 数据 (ext == 1)
                                tracing::debug!("SSH stderr (ext={}): {} bytes", ext, data.len());
                                let encoded = base64::Engine::encode(
                                    &base64::engine::general_purpose::STANDARD,
                                    &data,
                                );
                                if let Err(e) = notification_sender.send_output(&session_id, &encoded) {
                                    tracing::error!("发送 stderr 通知失败: {}", e);
                                    break;
                                }
                            }
                            Some(ChannelMsg::ExitStatus { exit_status }) => {
                                tracing::info!("SSH 进程退出: {} (code={})", session_id, exit_status);
                                
                                // 更新会话信息
                                {
                                    let mut info_guard = info.write().await;
                                    info_guard.status = SessionStatus::Done;
                                    info_guard.exit_code = Some(exit_status as i32);
                                }
                                
                                if let Err(e) = notification_sender.send_status(
                                    &session_id,
                                    "done",
                                    Some(exit_status as i32),
                                ) {
                                    tracing::error!("发送状态通知失败: {}", e);
                                }
                                break;
                            }
                            Some(ChannelMsg::Eof) => {
                                tracing::info!("SSH 通道 EOF: {}", session_id);
                                break;
                            }
                            Some(ChannelMsg::Close) => {
                                tracing::info!("SSH 通道关闭: {}", session_id);
                                break;
                            }
                            Some(other) => {
                                tracing::debug!("SSH 通道消息: {:?}", other);
                            }
                            None => {
                                tracing::info!("SSH 通道已断开: {}", session_id);
                                break;
                            }
                        }
                    }
                }
            }

            tracing::info!("SSH 输出读取器结束: {}", session_id);
        });

        self.output_task = Some(task);
        self.stop_tx = Some(stop_tx);

        Ok(())
    }

    /// 发送输入到 SSH 通道
    pub async fn send_input(&self, data: &[u8]) -> Result<(), TerminalError> {
        let channel = self.channel.as_ref().ok_or_else(|| {
            TerminalError::ChannelError("通道未打开".to_string())
        })?;

        let channel_guard = channel.lock().await;
        channel_guard.send_data(data).await?;

        tracing::debug!("发送 SSH 输入: {} bytes", data.len());
        Ok(())
    }

    /// 调整 PTY 大小
    pub async fn resize(&self, term_size: TermSize) -> Result<(), TerminalError> {
        let channel = self.channel.as_ref().ok_or_else(|| {
            TerminalError::ChannelError("通道未打开".to_string())
        })?;

        let channel_guard = channel.lock().await;
        channel_guard.resize(term_size.cols as u32, term_size.rows as u32).await?;

        tracing::debug!(
            "调整 SSH PTY 大小: {}x{}",
            term_size.cols,
            term_size.rows
        );
        Ok(())
    }

    /// 关闭会话
    pub async fn close(&mut self) -> Result<(), TerminalError> {
        tracing::info!("关闭 SSH 会话: {}", self.session_id);

        // 发送停止信号
        if let Some(stop_tx) = self.stop_tx.take() {
            let _ = stop_tx.send(()).await;
        }

        // 关闭通道
        if let Some(channel) = self.channel.take() {
            let channel_guard = channel.lock().await;
            let _ = channel_guard.eof().await;
            let _ = channel_guard.close().await;
        }

        // 等待输出任务结束
        if let Some(task) = self.output_task.take() {
            // 设置超时，避免无限等待
            let _ = tokio::time::timeout(
                std::time::Duration::from_secs(5),
                task,
            ).await;
        }

        // 断开 SSH 连接
        self.client.disconnect().await?;

        // 更新状态
        {
            let mut info = self.info.write().await;
            info.status = SessionStatus::Done;
        }

        Ok(())
    }

    /// 获取会话 ID
    pub fn id(&self) -> &str {
        &self.session_id
    }

    /// 获取会话信息（异步）
    pub async fn info(&self) -> SessionInfo {
        self.info.read().await.clone()
    }

    /// 获取会话信息引用
    pub fn info_ref(&self) -> Arc<RwLock<SessionInfo>> {
        self.info.clone()
    }

    /// 设置状态
    pub async fn set_status(&self, status: SessionStatus) {
        let mut info = self.info.write().await;
        info.status = status;
    }

    /// 检查是否已连接
    pub async fn is_connected(&self) -> bool {
        let info = self.info.read().await;
        self.client.is_connected() && info.status == SessionStatus::Running
    }
}

impl Drop for SshSession {
    fn drop(&mut self) {
        if self.client.is_connected() {
            tracing::warn!("SSH 会话被丢弃但未关闭: {}", self.session_id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ssh_session_new() {
        let session = SshSession::new(
            "test-session-id".to_string(),
            "example.com".to_string(),
            Some(22),
            Some("testuser".to_string()),
            None,
            Some("password".to_string()),
        );

        assert_eq!(session.id(), "test-session-id");
    }

    #[tokio::test]
    async fn test_ssh_session_info() {
        let session = SshSession::new(
            "test-id".to_string(),
            "host.example.com".to_string(),
            Some(2222),
            Some("user".to_string()),
            Some("/path/to/key".to_string()),
            None,
        );

        let info = session.info().await;
        assert_eq!(info.id, "test-id");
        assert_eq!(info.status, SessionStatus::Init);
        
        if let ConnectionType::Ssh { host, port, user, identity_file, password } = &info.connection_type {
            assert_eq!(host, "host.example.com");
            assert_eq!(*port, Some(2222));
            assert_eq!(*user, Some("user".to_string()));
            assert_eq!(*identity_file, Some("/path/to/key".to_string()));
            assert!(password.is_none());
        } else {
            panic!("Expected SSH connection type");
        }
    }

    #[tokio::test]
    async fn test_ssh_session_not_connected_initially() {
        let session = SshSession::new(
            "test-id".to_string(),
            "example.com".to_string(),
            None,
            None,
            None,
            None,
        );

        assert!(!session.is_connected().await);
    }

    #[tokio::test]
    async fn test_ssh_session_send_input_without_channel() {
        let session = SshSession::new(
            "test-id".to_string(),
            "example.com".to_string(),
            None,
            None,
            None,
            None,
        );

        let result = session.send_input(b"test").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_ssh_session_resize_without_channel() {
        let session = SshSession::new(
            "test-id".to_string(),
            "example.com".to_string(),
            None,
            None,
            None,
            None,
        );

        let result = session.resize(TermSize { rows: 24, cols: 80 }).await;
        assert!(result.is_err());
    }
}
