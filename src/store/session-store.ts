/**
 * 会话状态管理
 *
 * 管理多个终端会话的状态，提供 React Context 集成。
 * 支持会话的创建、切换、关闭和状态更新。
 *
 * @module store/session-store
 */

import React, { createContext, useContext, useReducer, useCallback, useMemo } from 'react';
import type { SessionInfo, SessionStatus, ConnectionType, TermSize } from '@/types/rpc';
import { rpcClient } from '@/lib/rpc-client';

/**
 * 会话状态
 */
export interface SessionState {
  /** 所有会话的映射 */
  sessions: Map<string, SessionInfo>;
  /** 当前活动会话 ID */
  activeSessionId: string | null;
  /** 是否正在加载 */
  isLoading: boolean;
  /** 错误信息 */
  error: string | null;
}

/**
 * 会话操作类型
 */
type SessionAction =
  | { type: 'ADD_SESSION'; payload: SessionInfo }
  | { type: 'REMOVE_SESSION'; payload: string }
  | { type: 'UPDATE_SESSION_STATUS'; payload: { sessionId: string; status: SessionStatus; exitCode?: number } }
  | { type: 'SET_ACTIVE_SESSION'; payload: string }
  | { type: 'UPDATE_SESSION_TITLE'; payload: { sessionId: string; title: string } }
  | { type: 'UPDATE_SESSION_CWD'; payload: { sessionId: string; cwd: string } }
  | { type: 'SET_LOADING'; payload: boolean }
  | { type: 'SET_ERROR'; payload: string | null }
  | { type: 'CLEAR_ALL_SESSIONS' };

/**
 * 创建初始状态
 */
export function createInitialState(): SessionState {
  return {
    sessions: new Map(),
    activeSessionId: null,
    isLoading: false,
    error: null,
  };
}

/**
 * 会话 Reducer
 */
function sessionReducer(state: SessionState, action: SessionAction): SessionState {
  switch (action.type) {
    case 'ADD_SESSION': {
      const sessions = new Map(state.sessions);
      sessions.set(action.payload.id, action.payload);
      return {
        ...state,
        sessions,
        activeSessionId: state.activeSessionId ?? action.payload.id,
        error: null,
      };
    }

    case 'REMOVE_SESSION': {
      const sessions = new Map(state.sessions);
      sessions.delete(action.payload);

      let activeSessionId = state.activeSessionId;
      if (activeSessionId === action.payload) {
        // 选择下一个会话
        const ids = Array.from(sessions.keys());
        activeSessionId = ids.length > 0 ? ids[0] : null;
      }

      return {
        ...state,
        sessions,
        activeSessionId,
      };
    }

    case 'UPDATE_SESSION_STATUS': {
      const session = state.sessions.get(action.payload.sessionId);
      if (!session) return state;

      const sessions = new Map(state.sessions);
      sessions.set(action.payload.sessionId, {
        ...session,
        status: action.payload.status,
        exit_code: action.payload.exitCode,
      });

      return {
        ...state,
        sessions,
      };
    }

    case 'SET_ACTIVE_SESSION': {
      if (!state.sessions.has(action.payload)) return state;
      return {
        ...state,
        activeSessionId: action.payload,
      };
    }

    case 'UPDATE_SESSION_TITLE': {
      const session = state.sessions.get(action.payload.sessionId);
      if (!session) return state;

      const sessions = new Map(state.sessions);
      sessions.set(action.payload.sessionId, {
        ...session,
        title: action.payload.title,
      });

      return {
        ...state,
        sessions,
      };
    }

    case 'UPDATE_SESSION_CWD': {
      const session = state.sessions.get(action.payload.sessionId);
      if (!session) return state;

      const sessions = new Map(state.sessions);
      sessions.set(action.payload.sessionId, {
        ...session,
        cwd: action.payload.cwd,
      });

      return {
        ...state,
        sessions,
      };
    }

    case 'SET_LOADING': {
      return {
        ...state,
        isLoading: action.payload,
      };
    }

    case 'SET_ERROR': {
      return {
        ...state,
        error: action.payload,
        isLoading: false,
      };
    }

    case 'CLEAR_ALL_SESSIONS': {
      return createInitialState();
    }

    default:
      return state;
  }
}

/**
 * 会话上下文值类型
 */
export interface SessionContextValue {
  /** 当前状态 */
  state: SessionState;
  /** 获取所有会话列表 */
  getSessions: () => SessionInfo[];
  /** 获取活动会话 */
  getActiveSession: () => SessionInfo | null;
  /** 创建新会话 */
  createSession: (connection: ConnectionType, termSize: TermSize) => Promise<string>;
  /** 关闭会话 */
  closeSession: (sessionId: string) => Promise<void>;
  /** 切换活动会话 */
  setActiveSession: (sessionId: string) => void;
  /** 更新会话状态 */
  updateSessionStatus: (sessionId: string, status: SessionStatus, exitCode?: number) => void;
  /** 更新会话标题 */
  updateSessionTitle: (sessionId: string, title: string) => void;
  /** 更新会话工作目录 */
  updateSessionCwd: (sessionId: string, cwd: string) => void;
}

/**
 * 会话上下文
 */
const SessionContext = createContext<SessionContextValue | null>(null);

/**
 * 会话 Provider Props
 */
export interface SessionProviderProps {
  children: React.ReactNode;
}

/**
 * 会话 Provider 组件
 *
 * 提供会话状态管理和 RPC 通信集成。
 *
 * @example
 * ```tsx
 * <SessionProvider>
 *   <TerminalApp />
 * </SessionProvider>
 * ```
 */
export function SessionProvider({ children }: SessionProviderProps): React.ReactElement {
  const [state, dispatch] = useReducer(sessionReducer, createInitialState());

  // 获取所有会话列表
  const getSessions = useCallback((): SessionInfo[] => {
    return Array.from(state.sessions.values());
  }, [state.sessions]);

  // 获取活动会话
  const getActiveSession = useCallback((): SessionInfo | null => {
    if (!state.activeSessionId) return null;
    return state.sessions.get(state.activeSessionId) ?? null;
  }, [state.activeSessionId, state.sessions]);

  // 创建新会话
  const createSession = useCallback(
    async (connection: ConnectionType, termSize: TermSize): Promise<string> => {
      dispatch({ type: 'SET_LOADING', payload: true });
      dispatch({ type: 'SET_ERROR', payload: null });

      try {
        const response = await rpcClient.createSession({ connection, term_size: termSize });
        const sessionId = response.session_id;

        // 创建会话信息
        const sessionInfo: SessionInfo = {
          id: sessionId,
          connection_type: connection,
          status: 'connecting',
          title: connection.type === 'local' ? 'Terminal' : `SSH: ${connection.host}`,
          created_at: Date.now(),
        };

        dispatch({ type: 'ADD_SESSION', payload: sessionInfo });
        dispatch({ type: 'SET_LOADING', payload: false });

        return sessionId;
      } catch (error) {
        const message = error instanceof Error ? error.message : '创建会话失败';
        dispatch({ type: 'SET_ERROR', payload: message });
        throw error;
      }
    },
    []
  );

  // 关闭会话
  const closeSession = useCallback(async (sessionId: string): Promise<void> => {
    try {
      await rpcClient.closeSession(sessionId);
      dispatch({ type: 'REMOVE_SESSION', payload: sessionId });
    } catch (error) {
      console.error('Failed to close session:', error);
      // 即使 RPC 失败，也从本地状态移除
      dispatch({ type: 'REMOVE_SESSION', payload: sessionId });
    }
  }, []);

  // 切换活动会话
  const setActiveSession = useCallback((sessionId: string): void => {
    dispatch({ type: 'SET_ACTIVE_SESSION', payload: sessionId });
  }, []);

  // 更新会话状态
  const updateSessionStatus = useCallback(
    (sessionId: string, status: SessionStatus, exitCode?: number): void => {
      dispatch({
        type: 'UPDATE_SESSION_STATUS',
        payload: { sessionId, status, exitCode },
      });
    },
    []
  );

  // 更新会话标题
  const updateSessionTitle = useCallback((sessionId: string, title: string): void => {
    dispatch({
      type: 'UPDATE_SESSION_TITLE',
      payload: { sessionId, title },
    });
  }, []);

  // 更新会话工作目录
  const updateSessionCwd = useCallback((sessionId: string, cwd: string): void => {
    dispatch({
      type: 'UPDATE_SESSION_CWD',
      payload: { sessionId, cwd },
    });
  }, []);

  // 监听 RPC 通知
  React.useEffect(() => {
    // 监听会话状态变化
    const unsubscribeStatus = rpcClient.on('session.status', (notification) => {
      updateSessionStatus(
        notification.session_id,
        notification.status,
        notification.exit_code
      );
    });

    // 监听标题变化
    const unsubscribeTitle = rpcClient.on('session.title', (notification) => {
      updateSessionTitle(notification.session_id, notification.title);
    });

    // 监听工作目录变化
    const unsubscribeCwd = rpcClient.on('session.cwd', (notification) => {
      updateSessionCwd(notification.session_id, notification.cwd);
    });

    return () => {
      unsubscribeStatus();
      unsubscribeTitle();
      unsubscribeCwd();
    };
  }, [updateSessionStatus, updateSessionTitle, updateSessionCwd]);

  const value = useMemo<SessionContextValue>(
    () => ({
      state,
      getSessions,
      getActiveSession,
      createSession,
      closeSession,
      setActiveSession,
      updateSessionStatus,
      updateSessionTitle,
      updateSessionCwd,
    }),
    [
      state,
      getSessions,
      getActiveSession,
      createSession,
      closeSession,
      setActiveSession,
      updateSessionStatus,
      updateSessionTitle,
      updateSessionCwd,
    ]
  );

  return React.createElement(SessionContext.Provider, { value }, children);
}

/**
 * 使用会话上下文的 Hook
 *
 * @throws 如果在 SessionProvider 外部使用会抛出错误
 *
 * @example
 * ```tsx
 * const { state, createSession, closeSession } = useSessionStore();
 * ```
 */
export function useSessionStore(): SessionContextValue {
  const context = useContext(SessionContext);
  if (!context) {
    throw new Error('useSessionStore must be used within a SessionProvider');
  }
  return context;
}

/**
 * 获取会话数量的 Hook
 */
export function useSessionCount(): number {
  const { state } = useSessionStore();
  return state.sessions.size;
}

/**
 * 获取活动会话 ID 的 Hook
 */
export function useActiveSessionId(): string | null {
  const { state } = useSessionStore();
  return state.activeSessionId;
}

/**
 * 获取特定会话的 Hook
 */
export function useSession(sessionId: string): SessionInfo | undefined {
  const { state } = useSessionStore();
  return state.sessions.get(sessionId);
}
