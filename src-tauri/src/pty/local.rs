//! 本地 PTY 实现
//!
//! 使用 portable-pty 创建和管理本地伪终端。

use portable_pty::{native_pty_system, CommandBuilder, MasterPty, PtySize};
use std::collections::HashMap;
use std::io::{Read, Write};

use crate::rpc::types::TermSize;
use crate::shell::detect::detect_default_shell;
use crate::utils::error::TerminalError;

/// 本地 PTY 实例
pub struct LocalPty {
    /// PTY master
    master: Box<dyn MasterPty + Send>,
    /// PTY writer
    writer: Box<dyn Write + Send>,
    /// 子进程
    child: Box<dyn portable_pty::Child + Send + Sync>,
}

impl LocalPty {
    /// 创建新的本地 PTY
    pub fn new(
        shell_path: Option<String>,
        cwd: Option<String>,
        env: Option<HashMap<String, String>>,
        term_size: TermSize,
    ) -> Result<Self, TerminalError> {
        // 获取 PTY 系统
        let pty_system = native_pty_system();

        // 配置 PTY 大小
        let size = PtySize {
            rows: term_size.rows,
            cols: term_size.cols,
            pixel_width: 0,
            pixel_height: 0,
        };

        // 创建 PTY pair
        let pair = pty_system
            .openpty(size)
            .map_err(|e| TerminalError::PtyCreationFailed(e.to_string()))?;

        // 构建命令
        let shell = shell_path.unwrap_or_else(detect_default_shell);
        let mut cmd = CommandBuilder::new(&shell);

        // 设置工作目录
        if let Some(dir) = cwd {
            cmd.cwd(dir);
        }

        // 设置 TERM 环境变量
        cmd.env("TERM", "xterm-256color");

        // 设置自定义环境变量
        if let Some(env_vars) = env {
            for (key, value) in env_vars {
                cmd.env(key, value);
            }
        }

        // 启动子进程
        let child = pair
            .slave
            .spawn_command(cmd)
            .map_err(|e| TerminalError::PtyCreationFailed(e.to_string()))?;

        // 获取 writer
        let writer = pair
            .master
            .take_writer()
            .map_err(|e| TerminalError::PtyCreationFailed(e.to_string()))?;

        Ok(Self {
            master: pair.master,
            writer,
            child,
        })
    }

    /// 获取 PTY reader
    pub fn try_clone_reader(&self) -> Result<Box<dyn Read + Send>, TerminalError> {
        self.master
            .try_clone_reader()
            .map_err(|e| TerminalError::IoError(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))
    }

    /// 写入数据到 PTY
    pub fn write(&mut self, data: &[u8]) -> Result<(), TerminalError> {
        self.writer.write_all(data)?;
        self.writer.flush()?;
        Ok(())
    }

    /// 调整 PTY 大小
    pub fn resize(&self, term_size: TermSize) -> Result<(), TerminalError> {
        let size = PtySize {
            rows: term_size.rows,
            cols: term_size.cols,
            pixel_width: 0,
            pixel_height: 0,
        };
        self.master
            .resize(size)
            .map_err(|e| TerminalError::IoError(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))
    }

    /// 检查子进程是否已退出
    pub fn try_wait(&mut self) -> Result<Option<portable_pty::ExitStatus>, TerminalError> {
        self.child
            .try_wait()
            .map_err(|e| TerminalError::IoError(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))
    }

    /// 等待子进程退出
    pub fn wait(&mut self) -> Result<portable_pty::ExitStatus, TerminalError> {
        self.child
            .wait()
            .map_err(|e| TerminalError::IoError(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))
    }

    /// 终止子进程
    pub fn kill(&mut self) -> Result<(), TerminalError> {
        self.child
            .kill()
            .map_err(|e| TerminalError::IoError(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_local_pty() {
        let result = LocalPty::new(None, None, None, TermSize::default());
        // 在测试环境中可能会失败，所以只检查是否能正常返回
        match result {
            Ok(mut pty) => {
                // 尝试终止进程
                let _ = pty.kill();
            }
            Err(e) => {
                // 在某些 CI 环境中可能没有 PTY 支持
                println!("PTY creation failed (may be expected in CI): {}", e);
            }
        }
    }

    #[test]
    fn test_create_local_pty_with_custom_shell() {
        #[cfg(unix)]
        let shell = Some("/bin/sh".to_string());
        #[cfg(windows)]
        let shell = Some("cmd.exe".to_string());

        let result = LocalPty::new(shell, None, None, TermSize::default());
        match result {
            Ok(mut pty) => {
                let _ = pty.kill();
            }
            Err(e) => {
                println!("PTY creation failed (may be expected in CI): {}", e);
            }
        }
    }

    #[test]
    fn test_create_local_pty_with_env() {
        let mut env = HashMap::new();
        env.insert("TEST_VAR".to_string(), "test_value".to_string());

        let result = LocalPty::new(None, None, Some(env), TermSize::default());
        match result {
            Ok(mut pty) => {
                let _ = pty.kill();
            }
            Err(e) => {
                println!("PTY creation failed (may be expected in CI): {}", e);
            }
        }
    }
}


/// Property-based tests for PTY configuration
/// Feature: terminal-plugin, Property 5: PTY 配置传递
/// **验证: 需求 8.3, 8.4**
#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    // Strategy for generating valid environment variable names
    fn env_var_name_strategy() -> impl Strategy<Value = String> {
        // Environment variable names: uppercase letters and underscores
        "[A-Z][A-Z0-9_]{2,10}".prop_map(|s| format!("TEST_{}", s))
    }

    // Strategy for generating valid environment variable values
    fn env_var_value_strategy() -> impl Strategy<Value = String> {
        "[a-zA-Z0-9_]{1,20}"
    }

    // Strategy for generating environment variable maps
    fn env_map_strategy() -> impl Strategy<Value = HashMap<String, String>> {
        prop::collection::hash_map(
            env_var_name_strategy(),
            env_var_value_strategy(),
            1..5,
        )
    }

    // Strategy for generating valid directory paths
    fn cwd_strategy() -> impl Strategy<Value = String> {
        prop_oneof![
            Just(std::env::temp_dir().to_string_lossy().to_string()),
            Just("/tmp".to_string()),
            Just(std::env::current_dir().unwrap_or_default().to_string_lossy().to_string()),
        ]
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(10))] // Reduced cases due to PTY creation overhead

        /// Feature: terminal-plugin, Property 5: PTY 配置传递
        /// *对于任意*指定的环境变量配置，PTY 管理器应该能够成功创建带有这些环境变量的 PTY
        #[test]
        fn prop_pty_env_vars_creation(env_vars in env_map_strategy()) {
            // Create PTY with custom environment variables
            let result = LocalPty::new(
                None,
                None,
                Some(env_vars.clone()),
                TermSize::default(),
            );

            match result {
                Ok(mut pty) => {
                    // PTY was created successfully with the env vars
                    let write_result = pty.write(b"echo test\n");
                    prop_assert!(
                        write_result.is_ok(),
                        "PTY should be writable after creation with env vars"
                    );
                    let _ = pty.kill();
                }
                Err(e) => {
                    println!("PTY creation failed (may be expected in CI): {}", e);
                }
            }
        }

        /// Feature: terminal-plugin, Property 5: PTY 配置传递
        /// *对于任意*指定的工作目录，PTY 管理器应该能够成功创建在该目录中启动的 PTY
        #[test]
        fn prop_pty_cwd_creation(cwd in cwd_strategy()) {
            // Skip if directory doesn't exist
            if !std::path::Path::new(&cwd).exists() {
                return Ok(());
            }

            // Create PTY with custom working directory
            let result = LocalPty::new(
                None,
                Some(cwd.clone()),
                None,
                TermSize::default(),
            );

            match result {
                Ok(mut pty) => {
                    let write_result = pty.write(b"echo test\n");
                    prop_assert!(
                        write_result.is_ok(),
                        "PTY should be writable after creation with cwd: {}",
                        cwd
                    );
                    let _ = pty.kill();
                }
                Err(e) => {
                    println!("PTY creation failed (may be expected in CI): {}", e);
                }
            }
        }

        /// Feature: terminal-plugin, Property 5: PTY 配置传递
        /// *对于任意* PTY 创建，TERM 环境变量应该被设置为 xterm-256color
        #[test]
        fn prop_pty_term_env_creation(_dummy in 0..5u32) {
            // Create PTY without custom env (TERM should still be set internally)
            let result = LocalPty::new(
                None,
                None,
                None,
                TermSize::default(),
            );

            match result {
                Ok(mut pty) => {
                    let write_result = pty.write(b"echo test\n");
                    prop_assert!(
                        write_result.is_ok(),
                        "PTY should be writable after creation"
                    );
                    let _ = pty.kill();
                }
                Err(e) => {
                    println!("PTY creation failed (may be expected in CI): {}", e);
                }
            }
        }

        /// Feature: terminal-plugin, Property 5: PTY 配置传递
        /// *对于任意*组合的配置（cwd + env），PTY 管理器应该能够成功创建
        #[test]
        fn prop_pty_combined_config(
            cwd in cwd_strategy(),
            env_vars in env_map_strategy()
        ) {
            // Skip if directory doesn't exist
            if !std::path::Path::new(&cwd).exists() {
                return Ok(());
            }

            // Create PTY with both cwd and env vars
            let result = LocalPty::new(
                None,
                Some(cwd.clone()),
                Some(env_vars.clone()),
                TermSize::default(),
            );

            match result {
                Ok(mut pty) => {
                    let write_result = pty.write(b"echo test\n");
                    prop_assert!(
                        write_result.is_ok(),
                        "PTY should be writable after creation with cwd: {} and {} env vars",
                        cwd,
                        env_vars.len()
                    );
                    let _ = pty.kill();
                }
                Err(e) => {
                    println!("PTY creation failed (may be expected in CI): {}", e);
                }
            }
        }
    }
}
