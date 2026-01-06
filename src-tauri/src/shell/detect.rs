//! Shell 检测
//!
//! 检测系统默认 shell。

use std::env;

/// 检测系统默认 shell
pub fn detect_default_shell() -> String {
    #[cfg(unix)]
    {
        // Unix: 使用 SHELL 环境变量
        env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string())
    }

    #[cfg(windows)]
    {
        // Windows: 使用 COMSPEC 环境变量
        env::var("COMSPEC").unwrap_or_else(|_| "cmd.exe".to_string())
    }
}

/// 获取 shell 名称
pub fn get_shell_name(shell_path: &str) -> &str {
    std::path::Path::new(shell_path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("shell")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_default_shell() {
        let shell = detect_default_shell();
        assert!(!shell.is_empty());
    }

    #[test]
    fn test_get_shell_name() {
        assert_eq!(get_shell_name("/bin/zsh"), "zsh");
        assert_eq!(get_shell_name("/bin/bash"), "bash");
        assert_eq!(get_shell_name("/usr/local/bin/fish"), "fish");
        assert_eq!(get_shell_name("cmd.exe"), "cmd.exe");
    }
}
