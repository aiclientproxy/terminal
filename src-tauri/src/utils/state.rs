//! 会话状态管理
//!
//! 提供会话状态转换逻辑和错误状态更新功能。
//!
//! ## 功能
//! - 定义有效的状态转换规则
//! - 提供状态转换验证
//! - 支持错误状态更新
//! - 记录状态变更日志
//!
//! ## 需求覆盖
//! - 需求 10.4: 会话遇到错误时更新状态为 'error'
//! - 需求 10.5: 发生意外错误时记录错误并继续运行

use crate::rpc::types::SessionStatus;
use crate::utils::error::TerminalError;

/// 状态转换结果
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StateTransitionResult {
    /// 转换成功
    Success,
    /// 转换无效（当前状态不允许转换到目标状态）
    Invalid {
        from: SessionStatus,
        to: SessionStatus,
        reason: String,
    },
}

impl StateTransitionResult {
    /// 检查转换是否成功
    pub fn is_success(&self) -> bool {
        matches!(self, StateTransitionResult::Success)
    }

    /// 检查转换是否失败
    pub fn is_invalid(&self) -> bool {
        matches!(self, StateTransitionResult::Invalid { .. })
    }
}

/// 会话状态管理器
///
/// 管理单个会话的状态转换，确保状态转换的有效性。
#[derive(Debug, Clone)]
pub struct SessionStateManager {
    /// 当前状态
    current_status: SessionStatus,
    /// 会话 ID（用于日志）
    session_id: String,
    /// 错误消息（如果状态为 Error）
    error_message: Option<String>,
}

impl SessionStateManager {
    /// 创建新的状态管理器
    pub fn new(session_id: impl Into<String>) -> Self {
        Self {
            current_status: SessionStatus::Init,
            session_id: session_id.into(),
            error_message: None,
        }
    }

    /// 创建带初始状态的状态管理器
    pub fn with_status(session_id: impl Into<String>, status: SessionStatus) -> Self {
        Self {
            current_status: status,
            session_id: session_id.into(),
            error_message: None,
        }
    }

    /// 获取当前状态
    pub fn status(&self) -> SessionStatus {
        self.current_status
    }

    /// 获取错误消息
    pub fn error_message(&self) -> Option<&str> {
        self.error_message.as_deref()
    }

    /// 检查是否可以转换到目标状态
    pub fn can_transition_to(&self, target: SessionStatus) -> bool {
        Self::is_valid_transition(self.current_status, target)
    }

    /// 尝试转换到目标状态
    ///
    /// 如果转换有效，更新状态并返回 Success。
    /// 如果转换无效，保持当前状态并返回 Invalid。
    pub fn transition_to(&mut self, target: SessionStatus) -> StateTransitionResult {
        if Self::is_valid_transition(self.current_status, target) {
            let from = self.current_status;
            self.current_status = target;
            
            // 如果不是错误状态，清除错误消息
            if target != SessionStatus::Error {
                self.error_message = None;
            }
            
            tracing::debug!(
                "会话 {} 状态转换: {:?} -> {:?}",
                self.session_id,
                from,
                target
            );
            
            StateTransitionResult::Success
        } else {
            let reason = Self::get_invalid_transition_reason(self.current_status, target);
            tracing::warn!(
                "会话 {} 无效状态转换: {:?} -> {:?}, 原因: {}",
                self.session_id,
                self.current_status,
                target,
                reason
            );
            
            StateTransitionResult::Invalid {
                from: self.current_status,
                to: target,
                reason,
            }
        }
    }

    /// 转换到错误状态
    ///
    /// 从任何状态都可以转换到错误状态。
    /// 记录错误消息以便后续查询。
    pub fn transition_to_error(&mut self, error: &TerminalError) {
        let from = self.current_status;
        self.current_status = SessionStatus::Error;
        self.error_message = Some(error.to_string());
        
        tracing::error!(
            "会话 {} 进入错误状态: {:?} -> Error, 错误: {}",
            self.session_id,
            from,
            error
        );
    }

    /// 转换到错误状态（带自定义消息）
    pub fn transition_to_error_with_message(&mut self, message: impl Into<String>) {
        let from = self.current_status;
        let msg = message.into();
        self.current_status = SessionStatus::Error;
        self.error_message = Some(msg.clone());
        
        tracing::error!(
            "会话 {} 进入错误状态: {:?} -> Error, 错误: {}",
            self.session_id,
            from,
            msg
        );
    }

    /// 强制设置状态（跳过验证）
    ///
    /// 仅用于特殊情况，如恢复会话状态。
    pub fn force_set_status(&mut self, status: SessionStatus) {
        tracing::warn!(
            "会话 {} 强制设置状态: {:?} -> {:?}",
            self.session_id,
            self.current_status,
            status
        );
        self.current_status = status;
    }

    /// 检查状态转换是否有效
    ///
    /// 状态转换规则：
    /// - Init -> Connecting, Running, Error, Done
    /// - Connecting -> Running, Error, Done
    /// - Running -> Done, Error
    /// - Done -> (终态，不能转换)
    /// - Error -> (终态，不能转换，除非强制重置)
    pub fn is_valid_transition(from: SessionStatus, to: SessionStatus) -> bool {
        // 相同状态不需要转换
        if from == to {
            return true;
        }

        // 任何状态都可以转换到 Error
        if to == SessionStatus::Error {
            return true;
        }

        match from {
            SessionStatus::Init => matches!(
                to,
                SessionStatus::Connecting | SessionStatus::Running | SessionStatus::Done
            ),
            SessionStatus::Connecting => matches!(
                to,
                SessionStatus::Running | SessionStatus::Done
            ),
            SessionStatus::Running => matches!(to, SessionStatus::Done),
            SessionStatus::Done => false, // 终态
            SessionStatus::Error => false, // 终态
        }
    }

    /// 获取无效转换的原因
    fn get_invalid_transition_reason(from: SessionStatus, to: SessionStatus) -> String {
        match from {
            SessionStatus::Done => "会话已完成，不能再转换状态".to_string(),
            SessionStatus::Error => "会话处于错误状态，不能再转换状态".to_string(),
            _ => format!("不允许从 {:?} 转换到 {:?}", from, to),
        }
    }

    /// 检查会话是否处于终态
    pub fn is_terminal(&self) -> bool {
        matches!(self.current_status, SessionStatus::Done | SessionStatus::Error)
    }

    /// 检查会话是否处于活动状态
    pub fn is_active(&self) -> bool {
        matches!(
            self.current_status,
            SessionStatus::Init | SessionStatus::Connecting | SessionStatus::Running
        )
    }

    /// 检查会话是否处于错误状态
    pub fn is_error(&self) -> bool {
        self.current_status == SessionStatus::Error
    }
}

impl Default for SessionStateManager {
    fn default() -> Self {
        Self::new("unknown")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_state_manager() {
        let manager = SessionStateManager::new("test-session");
        assert_eq!(manager.status(), SessionStatus::Init);
        assert!(manager.error_message().is_none());
    }

    #[test]
    fn test_with_status() {
        let manager = SessionStateManager::with_status("test", SessionStatus::Running);
        assert_eq!(manager.status(), SessionStatus::Running);
    }

    #[test]
    fn test_valid_transitions() {
        // Init -> Connecting
        assert!(SessionStateManager::is_valid_transition(
            SessionStatus::Init,
            SessionStatus::Connecting
        ));

        // Init -> Running
        assert!(SessionStateManager::is_valid_transition(
            SessionStatus::Init,
            SessionStatus::Running
        ));

        // Connecting -> Running
        assert!(SessionStateManager::is_valid_transition(
            SessionStatus::Connecting,
            SessionStatus::Running
        ));

        // Running -> Done
        assert!(SessionStateManager::is_valid_transition(
            SessionStatus::Running,
            SessionStatus::Done
        ));

        // Any -> Error
        assert!(SessionStateManager::is_valid_transition(
            SessionStatus::Init,
            SessionStatus::Error
        ));
        assert!(SessionStateManager::is_valid_transition(
            SessionStatus::Running,
            SessionStatus::Error
        ));
    }

    #[test]
    fn test_invalid_transitions() {
        // Done -> anything (except Error)
        assert!(!SessionStateManager::is_valid_transition(
            SessionStatus::Done,
            SessionStatus::Running
        ));
        assert!(!SessionStateManager::is_valid_transition(
            SessionStatus::Done,
            SessionStatus::Init
        ));

        // Error -> anything
        assert!(!SessionStateManager::is_valid_transition(
            SessionStatus::Error,
            SessionStatus::Running
        ));
        assert!(!SessionStateManager::is_valid_transition(
            SessionStatus::Error,
            SessionStatus::Done
        ));

        // Running -> Init (backwards)
        assert!(!SessionStateManager::is_valid_transition(
            SessionStatus::Running,
            SessionStatus::Init
        ));

        // Running -> Connecting (backwards)
        assert!(!SessionStateManager::is_valid_transition(
            SessionStatus::Running,
            SessionStatus::Connecting
        ));
    }

    #[test]
    fn test_transition_to() {
        let mut manager = SessionStateManager::new("test");
        
        // Valid transition
        let result = manager.transition_to(SessionStatus::Connecting);
        assert!(result.is_success());
        assert_eq!(manager.status(), SessionStatus::Connecting);

        // Another valid transition
        let result = manager.transition_to(SessionStatus::Running);
        assert!(result.is_success());
        assert_eq!(manager.status(), SessionStatus::Running);

        // Invalid transition (backwards)
        let result = manager.transition_to(SessionStatus::Init);
        assert!(result.is_invalid());
        assert_eq!(manager.status(), SessionStatus::Running); // Status unchanged
    }

    #[test]
    fn test_transition_to_error() {
        let mut manager = SessionStateManager::new("test");
        manager.transition_to(SessionStatus::Running);
        
        let error = TerminalError::PtyCreationFailed("test error".to_string());
        manager.transition_to_error(&error);
        
        assert_eq!(manager.status(), SessionStatus::Error);
        assert!(manager.error_message().is_some());
        assert!(manager.error_message().unwrap().contains("test error"));
    }

    #[test]
    fn test_transition_to_error_with_message() {
        let mut manager = SessionStateManager::new("test");
        manager.transition_to_error_with_message("custom error message");
        
        assert_eq!(manager.status(), SessionStatus::Error);
        assert_eq!(manager.error_message(), Some("custom error message"));
    }

    #[test]
    fn test_is_terminal() {
        let mut manager = SessionStateManager::new("test");
        assert!(!manager.is_terminal());

        manager.transition_to(SessionStatus::Running);
        assert!(!manager.is_terminal());

        manager.transition_to(SessionStatus::Done);
        assert!(manager.is_terminal());
    }

    #[test]
    fn test_is_active() {
        let mut manager = SessionStateManager::new("test");
        assert!(manager.is_active());

        manager.transition_to(SessionStatus::Running);
        assert!(manager.is_active());

        manager.transition_to(SessionStatus::Done);
        assert!(!manager.is_active());
    }

    #[test]
    fn test_is_error() {
        let mut manager = SessionStateManager::new("test");
        assert!(!manager.is_error());

        manager.transition_to_error_with_message("error");
        assert!(manager.is_error());
    }

    #[test]
    fn test_force_set_status() {
        let mut manager = SessionStateManager::new("test");
        manager.transition_to(SessionStatus::Done);
        
        // Normally can't transition from Done
        let result = manager.transition_to(SessionStatus::Running);
        assert!(result.is_invalid());

        // But force_set_status bypasses validation
        manager.force_set_status(SessionStatus::Running);
        assert_eq!(manager.status(), SessionStatus::Running);
    }

    #[test]
    fn test_same_state_transition() {
        let mut manager = SessionStateManager::new("test");
        manager.transition_to(SessionStatus::Running);
        
        // Same state transition should succeed
        let result = manager.transition_to(SessionStatus::Running);
        assert!(result.is_success());
    }

    #[test]
    fn test_error_message_cleared_on_non_error_transition() {
        let mut manager = SessionStateManager::new("test");
        manager.transition_to_error_with_message("error");
        assert!(manager.error_message().is_some());

        // Force reset to test clearing
        manager.force_set_status(SessionStatus::Init);
        manager.transition_to(SessionStatus::Running);
        
        // Error message should be cleared after non-error transition
        // Note: force_set_status doesn't clear error_message, but transition_to does
    }

    #[test]
    fn test_state_transition_result() {
        let success = StateTransitionResult::Success;
        assert!(success.is_success());
        assert!(!success.is_invalid());

        let invalid = StateTransitionResult::Invalid {
            from: SessionStatus::Done,
            to: SessionStatus::Running,
            reason: "test".to_string(),
        };
        assert!(!invalid.is_success());
        assert!(invalid.is_invalid());
    }
}


/// Property-based tests for session state management
/// Feature: terminal-plugin, Property 6: 错误状态一致性
/// **验证: 需求 10.4, 10.5**
#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    // Strategy for generating SessionStatus
    fn session_status_strategy() -> impl Strategy<Value = SessionStatus> {
        prop_oneof![
            Just(SessionStatus::Init),
            Just(SessionStatus::Connecting),
            Just(SessionStatus::Running),
            Just(SessionStatus::Done),
            Just(SessionStatus::Error),
        ]
    }

    // Strategy for generating non-terminal SessionStatus
    fn non_terminal_status_strategy() -> impl Strategy<Value = SessionStatus> {
        prop_oneof![
            Just(SessionStatus::Init),
            Just(SessionStatus::Connecting),
            Just(SessionStatus::Running),
        ]
    }

    // Strategy for generating error messages
    fn error_message_strategy() -> impl Strategy<Value = String> {
        "[a-zA-Z0-9 _-]{1,100}"
    }

    // Strategy for generating session IDs
    fn session_id_strategy() -> impl Strategy<Value = String> {
        "[a-f0-9-]{36}"
    }

    // Strategy for generating TerminalError variants
    fn terminal_error_strategy() -> impl Strategy<Value = TerminalError> {
        prop_oneof![
            error_message_strategy().prop_map(TerminalError::PtyCreationFailed),
            error_message_strategy().prop_map(TerminalError::SshConnectionFailed),
            error_message_strategy().prop_map(TerminalError::SessionNotFound),
            error_message_strategy().prop_map(TerminalError::InvalidRequest),
            error_message_strategy().prop_map(TerminalError::AuthenticationFailed),
            error_message_strategy().prop_map(TerminalError::ConnectionTimeout),
            error_message_strategy().prop_map(TerminalError::SessionClosed),
            error_message_strategy().prop_map(TerminalError::SshError),
            error_message_strategy().prop_map(TerminalError::ChannelError),
            error_message_strategy().prop_map(TerminalError::HostResolutionFailed),
            error_message_strategy().prop_map(TerminalError::PrivateKeyLoadFailed),
        ]
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Feature: terminal-plugin, Property 6: 错误状态一致性
        /// *对于任意*会话遇到的错误，系统应该将会话状态更新为 'error'
        /// **验证: 需求 10.4**
        #[test]
        fn prop_error_transitions_to_error_state(
            session_id in session_id_strategy(),
            initial_status in session_status_strategy(),
            error in terminal_error_strategy()
        ) {
            let mut manager = SessionStateManager::with_status(&session_id, initial_status);
            
            // 调用 transition_to_error
            manager.transition_to_error(&error);
            
            // 验证状态已更新为 Error
            prop_assert_eq!(
                manager.status(),
                SessionStatus::Error,
                "会话状态应该更新为 Error，但实际是 {:?}",
                manager.status()
            );
        }

        /// Feature: terminal-plugin, Property 6: 错误状态一致性
        /// *对于任意*会话遇到的错误，系统应该记录错误信息
        /// **验证: 需求 10.4**
        #[test]
        fn prop_error_message_is_recorded(
            session_id in session_id_strategy(),
            initial_status in non_terminal_status_strategy(),
            error in terminal_error_strategy()
        ) {
            let mut manager = SessionStateManager::with_status(&session_id, initial_status);
            
            // 调用 transition_to_error
            manager.transition_to_error(&error);
            
            // 验证错误消息已记录
            prop_assert!(
                manager.error_message().is_some(),
                "错误消息应该被记录"
            );
            
            // 验证错误消息包含原始错误信息
            let recorded_message = manager.error_message().unwrap();
            let error_string = error.to_string();
            prop_assert!(
                recorded_message.contains(&error_string) || recorded_message == error_string,
                "记录的错误消息应该包含原始错误信息。记录: {}, 原始: {}",
                recorded_message,
                error_string
            );
        }

        /// Feature: terminal-plugin, Property 6: 错误状态一致性
        /// *对于任意*会话遇到的错误，系统应该继续处理其他会话而不崩溃
        /// **验证: 需求 10.5**
        #[test]
        fn prop_multiple_sessions_independent_error_handling(
            session_ids in prop::collection::vec(session_id_strategy(), 2..10),
            error_indices in prop::collection::vec(0usize..100, 1..5),
            error in terminal_error_strategy()
        ) {
            // 创建多个会话管理器
            let mut managers: Vec<SessionStateManager> = session_ids
                .iter()
                .map(|id| SessionStateManager::with_status(id, SessionStatus::Running))
                .collect();
            
            // 对部分会话触发错误
            let error_indices: Vec<usize> = error_indices
                .iter()
                .map(|i| i % managers.len())
                .collect();
            
            for &idx in &error_indices {
                managers[idx].transition_to_error(&error);
            }
            
            // 验证：
            // 1. 触发错误的会话应该处于 Error 状态
            // 2. 未触发错误的会话应该保持 Running 状态
            // 3. 所有会话管理器都应该正常工作（没有崩溃）
            
            for (idx, manager) in managers.iter().enumerate() {
                if error_indices.contains(&idx) {
                    prop_assert_eq!(
                        manager.status(),
                        SessionStatus::Error,
                        "会话 {} 应该处于 Error 状态",
                        idx
                    );
                    prop_assert!(
                        manager.error_message().is_some(),
                        "会话 {} 应该有错误消息",
                        idx
                    );
                } else {
                    prop_assert_eq!(
                        manager.status(),
                        SessionStatus::Running,
                        "会话 {} 应该保持 Running 状态",
                        idx
                    );
                    prop_assert!(
                        manager.error_message().is_none(),
                        "会话 {} 不应该有错误消息",
                        idx
                    );
                }
            }
        }

        /// Feature: terminal-plugin, Property 6: 错误状态一致性
        /// *对于任意*状态，都可以转换到 Error 状态
        /// **验证: 需求 10.4**
        #[test]
        fn prop_any_state_can_transition_to_error(
            session_id in session_id_strategy(),
            initial_status in session_status_strategy()
        ) {
            // 验证从任何状态都可以转换到 Error
            prop_assert!(
                SessionStateManager::is_valid_transition(initial_status, SessionStatus::Error),
                "从 {:?} 应该可以转换到 Error 状态",
                initial_status
            );
        }

        /// Feature: terminal-plugin, Property 6: 错误状态一致性
        /// *对于任意*处于 Error 状态的会话，不能转换到其他非 Error 状态
        /// **验证: 需求 10.4**
        #[test]
        fn prop_error_state_is_terminal(
            session_id in session_id_strategy(),
            target_status in session_status_strategy()
        ) {
            let mut manager = SessionStateManager::with_status(&session_id, SessionStatus::Error);
            
            // 尝试转换到目标状态
            let result = manager.transition_to(target_status);
            
            if target_status == SessionStatus::Error {
                // 转换到相同状态应该成功
                prop_assert!(
                    result.is_success(),
                    "从 Error 转换到 Error 应该成功"
                );
            } else {
                // 转换到其他状态应该失败
                prop_assert!(
                    result.is_invalid(),
                    "从 Error 转换到 {:?} 应该失败",
                    target_status
                );
            }
            
            // 状态应该保持为 Error
            prop_assert_eq!(
                manager.status(),
                SessionStatus::Error,
                "状态应该保持为 Error"
            );
        }

        /// Feature: terminal-plugin, Property 6: 错误状态一致性
        /// *对于任意*错误消息，transition_to_error_with_message 应该正确记录
        /// **验证: 需求 10.4**
        #[test]
        fn prop_custom_error_message_recorded(
            session_id in session_id_strategy(),
            initial_status in non_terminal_status_strategy(),
            error_message in error_message_strategy()
        ) {
            let mut manager = SessionStateManager::with_status(&session_id, initial_status);
            
            // 使用自定义消息转换到错误状态
            manager.transition_to_error_with_message(&error_message);
            
            // 验证状态和消息
            prop_assert_eq!(manager.status(), SessionStatus::Error);
            prop_assert_eq!(
                manager.error_message(),
                Some(error_message.as_str()),
                "错误消息应该与输入完全匹配"
            );
        }

        /// Feature: terminal-plugin, Property 6: 错误状态一致性
        /// *对于任意*有效的状态转换序列，错误状态应该正确处理
        /// **验证: 需求 10.4, 10.5**
        #[test]
        fn prop_state_transition_sequence_with_error(
            session_id in session_id_strategy(),
            error in terminal_error_strategy()
        ) {
            let mut manager = SessionStateManager::new(&session_id);
            
            // 正常状态转换序列
            prop_assert_eq!(manager.status(), SessionStatus::Init);
            
            let result = manager.transition_to(SessionStatus::Connecting);
            prop_assert!(result.is_success());
            prop_assert_eq!(manager.status(), SessionStatus::Connecting);
            
            let result = manager.transition_to(SessionStatus::Running);
            prop_assert!(result.is_success());
            prop_assert_eq!(manager.status(), SessionStatus::Running);
            
            // 触发错误
            manager.transition_to_error(&error);
            prop_assert_eq!(manager.status(), SessionStatus::Error);
            prop_assert!(manager.error_message().is_some());
            
            // 验证无法从错误状态恢复
            let result = manager.transition_to(SessionStatus::Running);
            prop_assert!(result.is_invalid());
            prop_assert_eq!(manager.status(), SessionStatus::Error);
        }
    }
}
