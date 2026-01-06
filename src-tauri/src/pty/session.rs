//! PTY 会话
//!
//! 封装单个 PTY 会话的状态和操作。

use std::collections::HashMap;
use std::io::Read;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::Mutex;

use crate::rpc::server::NotificationSender;
use crate::rpc::types::{ConnectionType, SessionInfo, SessionStatus, TermSize};
use crate::utils::error::TerminalError;

use super::local::LocalPty;
use super::output::{start_output_reader, OutputReaderConfig, OutputReaderHandle};

/// PTY 会话
pub struct PtySession {
    /// 会话信息
    pub info: SessionInfo,
    /// 本地 PTY 实例（仅用于本地连接）
    local_pty: Option<Arc<Mutex<LocalPty>>>,
    /// 输出读取器句柄
    output_reader: Option<OutputReaderHandle>,
}

impl PtySession {
    /// 创建新会话（不启动 PTY）
    pub fn new(id: String, connection_type: ConnectionType) -> Self {
        let created_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Self {
            info: SessionInfo {
                id,
                connection_type,
                status: SessionStatus::Init,
                title: None,
                cwd: None,
                exit_code: None,
                created_at,
            },
            local_pty: None,
            output_reader: None,
        }
    }

    /// 创建并启动本地 PTY 会话
    pub fn new_local(
        id: String,
        shell_path: Option<String>,
        cwd: Option<String>,
        env: Option<HashMap<String, String>>,
        term_size: TermSize,
    ) -> Result<Self, TerminalError> {
        let created_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // 创建本地 PTY
        let local_pty = LocalPty::new(shell_path.clone(), cwd.clone(), env.clone(), term_size)?;

        Ok(Self {
            info: SessionInfo {
                id,
                connection_type: ConnectionType::Local {
                    shell_path,
                    cwd,
                    env,
                },
                status: SessionStatus::Running,
                title: None,
                cwd: None,
                exit_code: None,
                created_at,
            },
            local_pty: Some(Arc::new(Mutex::new(local_pty))),
            output_reader: None,
        })
    }

    /// 启动输出读取器
    /// 
    /// 开始异步读取 PTY 输出并通过通知发送到前端。
    pub async fn start_output_reader(
        &mut self,
        notification_sender: NotificationSender,
    ) -> Result<(), TerminalError> {
        if self.output_reader.is_some() {
            tracing::warn!("输出读取器已经在运行: {}", self.info.id);
            return Ok(());
        }

        let reader = self.try_clone_reader().await?;
        let handle = start_output_reader(
            self.info.id.clone(),
            reader,
            notification_sender,
            OutputReaderConfig::default(),
        );

        self.output_reader = Some(handle);
        tracing::info!("启动输出读取器: {}", self.info.id);
        Ok(())
    }

    /// 停止输出读取器
    pub async fn stop_output_reader(&mut self) {
        if let Some(handle) = self.output_reader.take() {
            handle.stop().await;
            tracing::info!("停止输出读取器: {}", self.info.id);
        }
    }

    /// 检查输出读取器是否已完成
    pub fn is_output_reader_finished(&self) -> bool {
        self.output_reader.as_ref().map_or(true, |h| h.is_finished())
    }

    /// 获取 PTY reader（用于读取输出）
    pub async fn try_clone_reader(&self) -> Result<Box<dyn Read + Send>, TerminalError> {
        if let Some(pty) = &self.local_pty {
            let pty = pty.lock().await;
            pty.try_clone_reader()
        } else {
            Err(TerminalError::SessionNotFound("No PTY available".to_string()))
        }
    }

    /// 写入数据到 PTY
    pub async fn write(&self, data: &[u8]) -> Result<(), TerminalError> {
        if let Some(pty) = &self.local_pty {
            let mut pty = pty.lock().await;
            pty.write(data)
        } else {
            Err(TerminalError::SessionNotFound("No PTY available".to_string()))
        }
    }

    /// 调整 PTY 大小
    pub async fn resize(&self, term_size: TermSize) -> Result<(), TerminalError> {
        if let Some(pty) = &self.local_pty {
            let pty = pty.lock().await;
            pty.resize(term_size)
        } else {
            Err(TerminalError::SessionNotFound("No PTY available".to_string()))
        }
    }

    /// 检查子进程是否已退出
    pub async fn try_wait(&self) -> Result<Option<portable_pty::ExitStatus>, TerminalError> {
        if let Some(pty) = &self.local_pty {
            let mut pty = pty.lock().await;
            pty.try_wait()
        } else {
            Err(TerminalError::SessionNotFound("No PTY available".to_string()))
        }
    }

    /// 终止 PTY 进程
    pub async fn kill(&self) -> Result<(), TerminalError> {
        if let Some(pty) = &self.local_pty {
            let mut pty = pty.lock().await;
            pty.kill()
        } else {
            Ok(()) // 没有 PTY 时直接返回成功
        }
    }

    /// 获取本地 PTY 引用
    pub fn local_pty(&self) -> Option<Arc<Mutex<LocalPty>>> {
        self.local_pty.clone()
    }

    /// 更新状态
    pub fn set_status(&mut self, status: SessionStatus) {
        self.info.status = status;
    }

    /// 设置退出码
    pub fn set_exit_code(&mut self, code: i32) {
        self.info.exit_code = Some(code);
    }

    /// 设置标题
    pub fn set_title(&mut self, title: String) {
        self.info.title = Some(title);
    }

    /// 设置工作目录
    pub fn set_cwd(&mut self, cwd: String) {
        self.info.cwd = Some(cwd);
    }

    /// 获取会话 ID
    pub fn id(&self) -> &str {
        &self.info.id
    }

    /// 获取会话信息
    pub fn info(&self) -> &SessionInfo {
        &self.info
    }
}
