//! PTY 输出读取器
//!
//! 异步读取 PTY 输出并通过 JSON-RPC 通知发送到前端。
//! 支持检测和处理 OSC 序列（如工作目录变更、剪贴板操作）。

use std::io::Read;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use crate::rpc::server::NotificationSender;
use crate::rpc::types::SessionStatus;
use crate::shell::osc::{OscHandler, OscSequence};

/// 输出读取器配置
pub struct OutputReaderConfig {
    /// 读取缓冲区大小
    pub buffer_size: usize,
    /// 读取超时时间
    pub read_timeout: Duration,
    /// 是否启用 OSC 处理
    pub enable_osc_processing: bool,
    /// 剪贴板大小限制（字节）
    pub max_clipboard_size: usize,
}

impl Default for OutputReaderConfig {
    fn default() -> Self {
        Self {
            buffer_size: 4096,
            read_timeout: Duration::from_millis(100),
            enable_osc_processing: true,
            max_clipboard_size: 1024 * 1024, // 1MB
        }
    }
}

/// 输出读取器句柄
pub struct OutputReaderHandle {
    /// 停止信号发送器
    stop_tx: mpsc::Sender<()>,
    /// 任务句柄
    task_handle: JoinHandle<()>,
}

impl OutputReaderHandle {
    /// 停止输出读取器
    pub async fn stop(self) {
        // 发送停止信号
        let _ = self.stop_tx.send(()).await;
        // 等待任务完成
        let _ = self.task_handle.await;
    }

    /// 检查任务是否已完成
    pub fn is_finished(&self) -> bool {
        self.task_handle.is_finished()
    }
}

/// 处理 OSC 序列并发送相应通知
fn process_osc_sequences(
    session_id: &str,
    data: &str,
    osc_handler: &OscHandler,
    notification_sender: &NotificationSender,
) -> String {
    let (stripped_data, sequences) = osc_handler.strip_sequences(data);

    for sequence in sequences {
        match sequence {
            OscSequence::WorkingDirectory(cwd) => {
                tracing::debug!("检测到工作目录变更: {} -> {}", session_id, cwd);
                if let Err(e) = notification_sender.send_cwd(session_id, &cwd) {
                    tracing::error!("发送工作目录通知失败: {}", e);
                }
            }
            OscSequence::Clipboard(clipboard_data) => {
                tracing::debug!(
                    "检测到剪贴板操作: {} ({} bytes)",
                    session_id,
                    clipboard_data.content.len()
                );
                // 发送剪贴板通知
                if let Err(e) = notification_sender.send_clipboard(
                    session_id,
                    &clipboard_data.content,
                ) {
                    tracing::error!("发送剪贴板通知失败: {}", e);
                }
            }
            OscSequence::Unknown => {
                // 忽略未知序列
            }
        }
    }

    stripped_data
}

/// 启动 PTY 输出读取器
///
/// 在后台任务中异步读取 PTY 输出，并通过 NotificationSender 发送到前端。
/// 当进程退出时，发送状态变更通知。
/// 
/// 如果启用了 OSC 处理，会自动检测并处理 OSC 序列：
/// - OSC 7: 发送工作目录变更通知
/// - OSC 52: 发送剪贴板内容通知
pub fn start_output_reader(
    session_id: String,
    reader: Box<dyn Read + Send>,
    notification_sender: NotificationSender,
    config: OutputReaderConfig,
) -> OutputReaderHandle {
    let (stop_tx, mut stop_rx) = mpsc::channel::<()>(1);

    // 创建 OSC 处理器
    let osc_handler = if config.enable_osc_processing {
        Some(OscHandler::new().with_max_clipboard_size(config.max_clipboard_size))
    } else {
        None
    };

    let task_handle = tokio::task::spawn_blocking(move || {
        let mut reader = reader;
        let mut buffer = vec![0u8; config.buffer_size];

        loop {
            // 检查是否收到停止信号
            if stop_rx.try_recv().is_ok() {
                tracing::debug!("输出读取器收到停止信号: {}", session_id);
                break;
            }

            // 尝试读取数据
            match reader.read(&mut buffer) {
                Ok(0) => {
                    // EOF - 进程已退出
                    tracing::info!("PTY 输出 EOF，进程已退出: {}", session_id);
                    
                    // 发送状态变更通知
                    if let Err(e) = notification_sender.send_status(
                        &session_id,
                        &serde_json::to_string(&SessionStatus::Done).unwrap().trim_matches('"'),
                        Some(0), // 默认退出码为 0
                    ) {
                        tracing::error!("发送状态通知失败: {}", e);
                    }
                    break;
                }
                Ok(n) => {
                    let data = &buffer[..n];
                    
                    // 尝试将数据转换为字符串以处理 OSC 序列
                    let output_data = if let Some(ref handler) = osc_handler {
                        // 尝试 UTF-8 解码
                        match std::str::from_utf8(data) {
                            Ok(text) => {
                                // 处理 OSC 序列
                                let processed = process_osc_sequences(
                                    &session_id,
                                    text,
                                    handler,
                                    &notification_sender,
                                );
                                processed.into_bytes()
                            }
                            Err(_) => {
                                // 非 UTF-8 数据，直接传递
                                data.to_vec()
                            }
                        }
                    } else {
                        data.to_vec()
                    };

                    // 如果处理后还有数据，编码为 base64 并发送
                    if !output_data.is_empty() {
                        let encoded = base64::Engine::encode(
                            &base64::engine::general_purpose::STANDARD,
                            &output_data,
                        );

                        tracing::trace!("读取 PTY 输出: {} bytes", output_data.len());

                        if let Err(e) = notification_sender.send_output(&session_id, &encoded) {
                            tracing::error!("发送输出通知失败: {}", e);
                            break;
                        }
                    }
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    // 非阻塞读取，没有数据可读，短暂休眠后继续
                    std::thread::sleep(Duration::from_millis(10));
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => {
                    // 被中断，继续读取
                    continue;
                }
                Err(e) => {
                    // 其他错误
                    tracing::error!("读取 PTY 输出错误: {}", e);
                    
                    // 发送错误状态通知
                    if let Err(send_err) = notification_sender.send_status(
                        &session_id,
                        &serde_json::to_string(&SessionStatus::Error).unwrap().trim_matches('"'),
                        None,
                    ) {
                        tracing::error!("发送错误状态通知失败: {}", send_err);
                    }
                    break;
                }
            }
        }

        tracing::debug!("输出读取器退出: {}", session_id);
    });

    OutputReaderHandle {
        stop_tx,
        task_handle,
    }
}

/// 进程退出监控器
/// 
/// 监控 PTY 子进程的退出状态，并在退出时发送通知。
pub struct ExitMonitor {
    /// 停止信号发送器
    stop_tx: mpsc::Sender<()>,
    /// 任务句柄
    task_handle: JoinHandle<()>,
}

impl ExitMonitor {
    /// 停止监控器
    pub async fn stop(self) {
        let _ = self.stop_tx.send(()).await;
        let _ = self.task_handle.await;
    }

    /// 检查任务是否已完成
    pub fn is_finished(&self) -> bool {
        self.task_handle.is_finished()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use tokio::sync::mpsc as tokio_mpsc;

    #[tokio::test]
    async fn test_output_reader_with_data() {
        // 创建测试数据
        let test_data = b"Hello, World!";
        let reader: Box<dyn Read + Send> = Box::new(Cursor::new(test_data.to_vec()));

        // 创建通知发送器
        let (tx, mut rx) = tokio_mpsc::unbounded_channel();
        let sender = NotificationSender::new_for_test(tx);

        // 启动输出读取器
        let handle = start_output_reader(
            "test-session".to_string(),
            reader,
            sender,
            OutputReaderConfig::default(),
        );

        // 等待一段时间让读取器处理数据
        tokio::time::sleep(Duration::from_millis(100)).await;

        // 检查是否收到输出通知
        let notification = rx.try_recv();
        assert!(notification.is_ok(), "Should receive output notification");

        let notif = notification.unwrap();
        assert_eq!(notif.method, "terminal.output");

        // 停止读取器
        handle.stop().await;
    }

    #[tokio::test]
    async fn test_output_reader_eof() {
        // 创建空数据（立即 EOF）
        let reader: Box<dyn Read + Send> = Box::new(Cursor::new(Vec::new()));

        // 创建通知发送器
        let (tx, mut rx) = tokio_mpsc::unbounded_channel();
        let sender = NotificationSender::new_for_test(tx);

        // 启动输出读取器
        let handle = start_output_reader(
            "test-session".to_string(),
            reader,
            sender,
            OutputReaderConfig::default(),
        );

        // 等待读取器完成
        tokio::time::sleep(Duration::from_millis(100)).await;

        // 检查是否收到状态通知
        let notification = rx.try_recv();
        assert!(notification.is_ok(), "Should receive status notification");

        let notif = notification.unwrap();
        assert_eq!(notif.method, "session.status");

        // 读取器应该已经完成
        assert!(handle.is_finished());
    }

    #[tokio::test]
    async fn test_output_reader_with_osc7() {
        // 创建包含 OSC 7 序列的测试数据
        let test_data = b"before\x1b]7;file://localhost/home/user\x07after";
        let reader: Box<dyn Read + Send> = Box::new(Cursor::new(test_data.to_vec()));

        // 创建通知发送器
        let (tx, mut rx) = tokio_mpsc::unbounded_channel();
        let sender = NotificationSender::new_for_test(tx);

        // 启动输出读取器（启用 OSC 处理）
        let handle = start_output_reader(
            "test-session".to_string(),
            reader,
            sender,
            OutputReaderConfig::default(),
        );

        // 等待一段时间让读取器处理数据
        tokio::time::sleep(Duration::from_millis(100)).await;

        // 收集所有通知
        let mut notifications = Vec::new();
        while let Ok(notif) = rx.try_recv() {
            notifications.push(notif);
        }

        // 应该收到工作目录通知
        let cwd_notif = notifications.iter().find(|n| n.method == "session.cwd");
        assert!(cwd_notif.is_some(), "Should receive cwd notification");
        
        let cwd_params = cwd_notif.unwrap().params.as_ref().unwrap();
        assert_eq!(cwd_params["cwd"], "/home/user");

        // 应该收到输出通知（不包含 OSC 序列）
        let output_notif = notifications.iter().find(|n| n.method == "terminal.output");
        assert!(output_notif.is_some(), "Should receive output notification");

        // 停止读取器
        handle.stop().await;
    }

    #[tokio::test]
    async fn test_output_reader_with_osc52() {
        // 创建包含 OSC 52 序列的测试数据
        // "Hello" in base64 is "SGVsbG8="
        let test_data = b"text\x1b]52;c;SGVsbG8=\x07more";
        let reader: Box<dyn Read + Send> = Box::new(Cursor::new(test_data.to_vec()));

        // 创建通知发送器
        let (tx, mut rx) = tokio_mpsc::unbounded_channel();
        let sender = NotificationSender::new_for_test(tx);

        // 启动输出读取器
        let handle = start_output_reader(
            "test-session".to_string(),
            reader,
            sender,
            OutputReaderConfig::default(),
        );

        // 等待一段时间让读取器处理数据
        tokio::time::sleep(Duration::from_millis(100)).await;

        // 收集所有通知
        let mut notifications = Vec::new();
        while let Ok(notif) = rx.try_recv() {
            notifications.push(notif);
        }

        // 应该收到剪贴板通知
        let clipboard_notif = notifications.iter().find(|n| n.method == "session.clipboard");
        assert!(clipboard_notif.is_some(), "Should receive clipboard notification");
        
        let clipboard_params = clipboard_notif.unwrap().params.as_ref().unwrap();
        assert_eq!(clipboard_params["content"], "Hello");

        // 停止读取器
        handle.stop().await;
    }

    #[tokio::test]
    async fn test_output_reader_osc_disabled() {
        // 创建包含 OSC 序列的测试数据
        let test_data = b"before\x1b]7;file://localhost/home/user\x07after";
        let reader: Box<dyn Read + Send> = Box::new(Cursor::new(test_data.to_vec()));

        // 创建通知发送器
        let (tx, mut rx) = tokio_mpsc::unbounded_channel();
        let sender = NotificationSender::new_for_test(tx);

        // 启动输出读取器（禁用 OSC 处理）
        let mut config = OutputReaderConfig::default();
        config.enable_osc_processing = false;
        
        let handle = start_output_reader(
            "test-session".to_string(),
            reader,
            sender,
            config,
        );

        // 等待一段时间让读取器处理数据
        tokio::time::sleep(Duration::from_millis(100)).await;

        // 收集所有通知
        let mut notifications = Vec::new();
        while let Ok(notif) = rx.try_recv() {
            notifications.push(notif);
        }

        // 不应该收到工作目录通知
        let cwd_notif = notifications.iter().find(|n| n.method == "session.cwd");
        assert!(cwd_notif.is_none(), "Should not receive cwd notification when OSC disabled");

        // 应该收到包含原始 OSC 序列的输出通知
        let output_notif = notifications.iter().find(|n| n.method == "terminal.output");
        assert!(output_notif.is_some(), "Should receive output notification");

        // 停止读取器
        handle.stop().await;
    }
}
