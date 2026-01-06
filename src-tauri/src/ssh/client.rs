//! SSH 客户端
//!
//! 使用 russh 建立 SSH 连接，支持密码和私钥认证。

use std::net::ToSocketAddrs;
use std::sync::Arc;

use russh::client::{Config, Handle, Handler};
use russh::keys::key::PublicKey;
use russh::{ChannelId, Disconnect};
use tokio::net::TcpStream;

use crate::utils::error::TerminalError;

use super::auth::AuthMethod;

/// SSH 客户端配置
#[derive(Debug, Clone)]
pub struct SshClientConfig {
    /// 远程主机地址
    pub host: String,
    /// 远程端口（默认 22）
    pub port: u16,
    /// 用户名
    pub user: String,
    /// 认证方式
    pub auth_method: AuthMethod,
    /// 连接超时（秒）
    pub connect_timeout: u64,
}

impl Default for SshClientConfig {
    fn default() -> Self {
        Self {
            host: String::new(),
            port: 22,
            user: String::new(),
            auth_method: AuthMethod::None,
            connect_timeout: 30,
        }
    }
}

/// SSH 客户端事件处理器
pub struct SshClientHandler {
    /// 是否已验证主机密钥
    host_key_verified: bool,
}

impl SshClientHandler {
    pub fn new() -> Self {
        Self {
            host_key_verified: false,
        }
    }
}

impl Default for SshClientHandler {
    fn default() -> Self {
        Self::new()
    }
}

/// 实现 russh 的 Handler trait
#[async_trait::async_trait]
impl Handler for SshClientHandler {
    type Error = TerminalError;

    /// 检查服务器公钥
    /// 
    /// 注意：在生产环境中应该实现 known_hosts 检查
    async fn check_server_key(
        &mut self,
        _server_public_key: &PublicKey,
    ) -> Result<bool, Self::Error> {
        // TODO: 实现 known_hosts 检查
        // 目前接受所有服务器密钥（不安全，仅用于开发）
        tracing::warn!("接受服务器密钥（未验证 known_hosts）");
        self.host_key_verified = true;
        Ok(true)
    }

    /// 处理通道数据
    async fn data(
        &mut self,
        _channel: ChannelId,
        _data: &[u8],
        _session: &mut russh::client::Session,
    ) -> Result<(), Self::Error> {
        // 数据处理在 SshSession 中进行
        Ok(())
    }

    /// 处理扩展数据（stderr）
    async fn extended_data(
        &mut self,
        _channel: ChannelId,
        _ext: u32,
        _data: &[u8],
        _session: &mut russh::client::Session,
    ) -> Result<(), Self::Error> {
        // stderr 数据处理在 SshSession 中进行
        Ok(())
    }

    /// 处理通道 EOF
    async fn channel_eof(
        &mut self,
        _channel: ChannelId,
        _session: &mut russh::client::Session,
    ) -> Result<(), Self::Error> {
        tracing::debug!("SSH 通道收到 EOF");
        Ok(())
    }

    /// 处理通道关闭
    async fn channel_close(
        &mut self,
        _channel: ChannelId,
        _session: &mut russh::client::Session,
    ) -> Result<(), Self::Error> {
        tracing::debug!("SSH 通道已关闭");
        Ok(())
    }
}

/// SSH 客户端
/// 
/// 封装 russh 客户端连接，提供连接、认证和断开功能。
pub struct SshClient {
    /// 客户端配置
    config: SshClientConfig,
    /// SSH 会话句柄
    handle: Option<Handle<SshClientHandler>>,
}

impl SshClient {
    /// 创建新的 SSH 客户端
    pub fn new(config: SshClientConfig) -> Self {
        Self {
            config,
            handle: None,
        }
    }

    /// 从连接参数创建 SSH 客户端
    pub fn from_params(
        host: String,
        port: Option<u16>,
        user: Option<String>,
        identity_file: Option<String>,
        password: Option<String>,
    ) -> Self {
        let auth_method = if let Some(key_path) = identity_file {
            AuthMethod::PrivateKey {
                path: key_path,
                passphrase: None,
            }
        } else if let Some(pwd) = password {
            AuthMethod::Password(pwd)
        } else {
            AuthMethod::None
        };

        let config = SshClientConfig {
            host,
            port: port.unwrap_or(22),
            user: user.unwrap_or_else(|| whoami::username()),
            auth_method,
            connect_timeout: 30,
        };

        Self::new(config)
    }

    /// 连接到远程服务器
    pub async fn connect(&mut self) -> Result<(), TerminalError> {
        tracing::info!(
            "连接到 SSH 服务器: {}@{}:{}",
            self.config.user,
            self.config.host,
            self.config.port
        );

        // 解析地址
        let addr = format!("{}:{}", self.config.host, self.config.port)
            .to_socket_addrs()
            .map_err(|e| {
                TerminalError::host_resolution_failed(
                    &self.config.host,
                    &e.to_string(),
                )
            })?
            .next()
            .ok_or_else(|| {
                TerminalError::host_resolution_failed(
                    &self.config.host,
                    "无法解析为有效地址",
                )
            })?;

        // 建立 TCP 连接
        let tcp = TcpStream::connect(addr).await.map_err(|e| {
            TerminalError::ssh_connection_failed(
                &self.config.host,
                self.config.port,
                &format!("TCP 连接失败: {}", e),
            )
        })?;

        // 创建 SSH 配置
        let ssh_config = Arc::new(Config::default());

        // 创建 SSH 客户端处理器
        let handler = SshClientHandler::new();

        // 建立 SSH 连接
        let handle = russh::client::connect_stream(ssh_config, tcp, handler)
            .await
            .map_err(|e| {
                TerminalError::ssh_connection_failed(
                    &self.config.host,
                    self.config.port,
                    &format!("SSH 握手失败: {}", e),
                )
            })?;

        self.handle = Some(handle);

        // 执行认证
        self.authenticate().await?;

        tracing::info!("SSH 连接成功: {}@{}", self.config.user, self.config.host);
        Ok(())
    }

    /// 执行认证
    async fn authenticate(&mut self) -> Result<(), TerminalError> {
        let handle = self.handle.as_mut().ok_or_else(|| {
            TerminalError::ssh_connection_failed(
                &self.config.host,
                self.config.port,
                "未建立连接",
            )
        })?;

        match &self.config.auth_method {
            AuthMethod::Password(password) => {
                tracing::debug!("使用密码认证");
                let auth_result = handle
                    .authenticate_password(&self.config.user, password)
                    .await
                    .map_err(|e| {
                        TerminalError::password_auth_failed(&format!(
                            "认证请求失败: {}",
                            e
                        ))
                    })?;

                if !auth_result {
                    return Err(TerminalError::password_auth_failed(
                        "密码被服务器拒绝",
                    ));
                }
            }
            AuthMethod::PrivateKey { path, passphrase } => {
                tracing::debug!("使用私钥认证: {}", path);
                
                // 加载私钥
                let key = super::auth::load_private_key(path, passphrase.as_deref())?;
                
                let auth_result = handle
                    .authenticate_publickey(&self.config.user, Arc::new(key))
                    .await
                    .map_err(|e| {
                        TerminalError::key_auth_failed(path, &format!(
                            "认证请求失败: {}",
                            e
                        ))
                    })?;

                if !auth_result {
                    return Err(TerminalError::key_auth_failed(
                        path,
                        "私钥被服务器拒绝",
                    ));
                }
            }
            AuthMethod::None => {
                tracing::debug!("尝试无认证连接");
                let auth_result = handle
                    .authenticate_none(&self.config.user)
                    .await
                    .map_err(|e| {
                        TerminalError::auth_failed("none", &format!(
                            "认证请求失败: {}",
                            e
                        ))
                    })?;

                if !auth_result {
                    return Err(TerminalError::AuthenticationFailed(
                        "服务器要求认证，请提供密码或私钥".to_string(),
                    ));
                }
            }
        }

        tracing::info!("SSH 认证成功");
        Ok(())
    }

    /// 获取 SSH 会话句柄
    pub fn handle(&self) -> Option<&Handle<SshClientHandler>> {
        self.handle.as_ref()
    }

    /// 获取可变 SSH 会话句柄
    pub fn handle_mut(&mut self) -> Option<&mut Handle<SshClientHandler>> {
        self.handle.as_mut()
    }

    /// 获取配置
    pub fn config(&self) -> &SshClientConfig {
        &self.config
    }

    /// 检查是否已连接
    pub fn is_connected(&self) -> bool {
        self.handle.is_some()
    }

    /// 断开连接
    pub async fn disconnect(&mut self) -> Result<(), TerminalError> {
        if let Some(handle) = self.handle.take() {
            tracing::info!("断开 SSH 连接: {}", self.config.host);
            handle
                .disconnect(Disconnect::ByApplication, "Client disconnecting", "en")
                .await
                .map_err(|e| {
                    TerminalError::SshConnectionFailed(format!("断开连接失败: {}", e))
                })?;
        }
        Ok(())
    }
}

impl Drop for SshClient {
    fn drop(&mut self) {
        if self.handle.is_some() {
            tracing::warn!("SSH 客户端被丢弃但未断开连接");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ssh_client_config_default() {
        let config = SshClientConfig::default();
        assert_eq!(config.port, 22);
        assert_eq!(config.connect_timeout, 30);
        assert!(config.host.is_empty());
        assert!(config.user.is_empty());
    }

    #[test]
    fn test_ssh_client_from_params_with_password() {
        let client = SshClient::from_params(
            "example.com".to_string(),
            Some(2222),
            Some("testuser".to_string()),
            None,
            Some("testpass".to_string()),
        );

        assert_eq!(client.config.host, "example.com");
        assert_eq!(client.config.port, 2222);
        assert_eq!(client.config.user, "testuser");
        assert!(matches!(client.config.auth_method, AuthMethod::Password(_)));
    }

    #[test]
    fn test_ssh_client_from_params_with_key() {
        let client = SshClient::from_params(
            "example.com".to_string(),
            None,
            Some("testuser".to_string()),
            Some("/path/to/key".to_string()),
            None,
        );

        assert_eq!(client.config.host, "example.com");
        assert_eq!(client.config.port, 22);
        assert!(matches!(
            client.config.auth_method,
            AuthMethod::PrivateKey { .. }
        ));
    }

    #[test]
    fn test_ssh_client_from_params_default_user() {
        let client = SshClient::from_params(
            "example.com".to_string(),
            None,
            None,
            None,
            Some("pass".to_string()),
        );

        // 应该使用当前用户名
        assert!(!client.config.user.is_empty());
    }

    #[test]
    fn test_ssh_client_not_connected_initially() {
        let client = SshClient::from_params(
            "example.com".to_string(),
            None,
            None,
            None,
            None,
        );

        assert!(!client.is_connected());
        assert!(client.handle().is_none());
    }
}
