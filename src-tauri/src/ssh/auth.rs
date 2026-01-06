//! SSH 认证
//!
//! 支持密码和私钥认证方式。

use std::path::Path;

use russh_keys::key::KeyPair;

use crate::utils::error::TerminalError;

/// 认证方式
#[derive(Debug, Clone)]
pub enum AuthMethod {
    /// 无认证（用于测试或特殊配置）
    None,
    /// 密码认证
    Password(String),
    /// 私钥认证
    PrivateKey {
        /// 私钥文件路径
        path: String,
        /// 私钥密码（可选）
        passphrase: Option<String>,
    },
}

impl Default for AuthMethod {
    fn default() -> Self {
        Self::None
    }
}

/// 加载私钥文件
///
/// 支持 OpenSSH 格式和 PEM 格式的私钥。
///
/// # 参数
/// - `path`: 私钥文件路径
/// - `passphrase`: 私钥密码（如果私钥已加密）
///
/// # 返回
/// - `Ok(KeyPair)`: 成功加载的密钥对
/// - `Err(TerminalError)`: 加载失败的错误信息
pub fn load_private_key(path: &str, passphrase: Option<&str>) -> Result<KeyPair, TerminalError> {
    let path = expand_tilde(path);
    let key_path = Path::new(&path);

    // 检查文件是否存在
    if !key_path.exists() {
        return Err(TerminalError::key_load_failed(
            &path,
            "文件不存在",
        ));
    }

    // 检查文件权限（仅 Unix）
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(metadata) = std::fs::metadata(key_path) {
            let mode = metadata.permissions().mode();
            // 检查是否有组或其他用户的读取权限
            if mode & 0o077 != 0 {
                tracing::warn!(
                    "私钥文件 {} 权限过于宽松 (mode: {:o})，建议设置为 600",
                    path,
                    mode & 0o777
                );
            }
        }
    }

    // 读取私钥文件
    let key_data = std::fs::read_to_string(key_path).map_err(|e| {
        TerminalError::key_load_failed(&path, &format!("无法读取文件: {}", e))
    })?;

    // 解析私钥
    let key = if let Some(pass) = passphrase {
        russh_keys::decode_secret_key(&key_data, Some(pass)).map_err(|e| {
            TerminalError::key_load_failed(&path, &format!("解析失败（密码可能错误）: {}", e))
        })?
    } else {
        russh_keys::decode_secret_key(&key_data, None).map_err(|e| {
            // 检查是否是因为需要密码
            let err_str = e.to_string().to_lowercase();
            if err_str.contains("passphrase") || err_str.contains("encrypted") {
                TerminalError::key_load_failed(&path, "私钥已加密，需要提供密码")
            } else {
                TerminalError::key_load_failed(&path, &format!("解析失败: {}", e))
            }
        })?
    };

    tracing::debug!("成功加载私钥: {}", path);
    Ok(key)
}

/// 展开路径中的 ~ 为用户主目录
fn expand_tilde(path: &str) -> String {
    if path.starts_with("~/") {
        if let Some(home) = dirs::home_dir() {
            return path.replacen("~", &home.to_string_lossy(), 1);
        }
    }
    path.to_string()
}

/// 获取默认 SSH 私钥路径列表
///
/// 返回常见的 SSH 私钥文件路径，按优先级排序。
pub fn default_identity_files() -> Vec<String> {
    let mut paths = Vec::new();

    if let Some(home) = dirs::home_dir() {
        let ssh_dir = home.join(".ssh");
        
        // 常见的私钥文件名
        let key_names = [
            "id_ed25519",
            "id_ecdsa",
            "id_rsa",
            "id_dsa",
            "identity",
        ];

        for name in key_names {
            let key_path = ssh_dir.join(name);
            if key_path.exists() {
                paths.push(key_path.to_string_lossy().to_string());
            }
        }
    }

    paths
}

/// 尝试使用默认私钥进行认证
///
/// 按优先级尝试加载默认的 SSH 私钥文件。
pub fn try_load_default_key() -> Option<(String, KeyPair)> {
    for path in default_identity_files() {
        match load_private_key(&path, None) {
            Ok(key) => {
                tracing::info!("使用默认私钥: {}", path);
                return Some((path, key));
            }
            Err(e) => {
                tracing::debug!("无法加载私钥 {}: {}", path, e);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_method_default() {
        let method = AuthMethod::default();
        assert!(matches!(method, AuthMethod::None));
    }

    #[test]
    fn test_expand_tilde() {
        let expanded = expand_tilde("~/test/path");
        assert!(!expanded.starts_with("~/"));
        
        let no_tilde = expand_tilde("/absolute/path");
        assert_eq!(no_tilde, "/absolute/path");
    }

    #[test]
    fn test_load_nonexistent_key() {
        let result = load_private_key("/nonexistent/path/to/key", None);
        assert!(result.is_err());
        
        if let Err(TerminalError::PrivateKeyLoadFailed(msg)) = result {
            assert!(msg.contains("不存在") || msg.contains("文件不存在"));
        } else {
            panic!("Expected PrivateKeyLoadFailed error");
        }
    }

    #[test]
    fn test_default_identity_files() {
        // 这个测试只验证函数不会崩溃
        let paths = default_identity_files();
        // 路径列表可能为空（如果没有 SSH 密钥）
        for path in &paths {
            assert!(Path::new(path).exists());
        }
    }
}
