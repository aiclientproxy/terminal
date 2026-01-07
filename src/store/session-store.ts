/**
 * 会话状态管理
 *
 * 管理多个终端会话的状态，提供 React Context 集成。
 * 支持会话的创建、切换、关闭和状态更新。
 * 使用 RpcContext 进行 RPC 通信。
 *
 * @module store/session-store
 */

import React, { createContext, useContext, useReducer, useCallback, useMemo } from "react";
import type { SessionInfo, SessionStatus, ConnectionType, TermSize } from "@/types/rpc";
import { useRpcContext } from "@/lib/rpc-context";

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
  | { type: "ADD_SESSION"; payload: SessionInfo }
  | { type: "REMOVE_SESSION"; payload: string }
  | { type: "UPDATE_SESSION_STATUS"; payload: { sessionId: string; status: SessionStatus; exitCode?: number } }
  | { type: "SET_ACTIVE_SESSION"; payload: string }
  | { type: "UPDATE_SESSION_TITLE"; payload: { sessionId: string; title: string } }
  | { type: "UPDATE_SESSION_CWD"; payload: { sessionId: string; cwd: string } }
  | { type: "SET_LOADING"; payload: boolean }
  | { type: "SET_ERROR"; payload: string | null }
  | { type: "CLEAR_ALL_SESSIONS" };

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
    case "ADD_SESSION": {
      const sessions = new Map(state.sessions);
      sessions.set(action.payload.id, action.payload);
      return {
        ...state,
        sessions,
        activeSessionId: state.activeSessionId ?? action.payload.id,
        error: null,
      };
    }

    case "REMOVE_SESSION": {
      const sessions = new Map(state.sessions);
      sessions.delete(action.payload);

      let activeSessionId = state.activeSessionId;
      if (activeSessionId === action.payload) {
        const ids = Array.from(sessions.keys());
        activeSessionId = ids.length > 0 ? ids[0] : null;
      }

      return {
        ...state,
        sessions,
        activeSessionId,
      };
    }

    case "UPDATE_SESSION_STATUS": {
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

    case "SET_ACTIVE_SESSION": {
      if (!state.sessions.has(action.payload)) return state;
      return {
        ...state,
        activeSessionId: action.payload,
      };
    }

    case "UPDATE_SESSION_TITLE": {
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

    case "UPDATE_SESSION_CWD": {
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

    case "SET_LOADING": {
      return {
        ...state,
        isLoading: action.payload,
      };
    }

    case "SET_ERROR": {
      return {
        ...state,
        error: action.payload,
        isLoading: false,
      };
    }

    case "CLEAR_ALL_SESSIONS": {
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
  state: SessionState;
  getSessions: () => SessionInfo[];
  getActiveSession: () => SessionInfo | null;
  createSession: (connection: ConnectionType, termSize: TermSize) => Promise<string>;
  closeSession: (sessionId: string) => Promise<void>;
  setActiveSession: (sessionId: string) => void;
  updateSessionStatus: (sessionId: string, status: SessionStatus, exitCode?: number) => void;
  updateSessionTitle: (sessionId: string, title: string) => void;
  updateSessionCwd: (sessionId: string, cwd: string) => void;
}

const SessionContext = createContext<SessionContextValue | null>(null);

export interface SessionProviderProps {
  children: React.ReactNode;
}

/**
 * 会话 Provider 组件
 */
export function SessionProvider({ children }: SessionProviderProps): React.ReactElement {
  const [state, dispatch] = useReducer(sessionReducer, createInitialState());
  const rpc = useRpcContext();

  const getSessions = useCallback((): SessionInfo[] => {
    return Array.from(state.sessions.values());
  }, [state.sessions]);

  const getActiveSession = useCallback((): SessionInfo | null => {
    if (!state.activeSessionId) return null;
    return state.sessions.get(state.activeSessionId) ?? null;
  }, [state.activeSessionId, state.sessions]);

  const createSession = useCallback(
    async (connection: ConnectionType, termSize: TermSize): Promise<string> => {
      dispatch({ type: "SET_LOADING", payload: true });
      dispatch({ type: "SET_ERROR", payload: null });

      try {
        const sessionId = await rpc.createSession(connection, termSize);

        const sessionInfo: SessionInfo = {
          id: sessionId,
          connection_type: connection,
          status: "connecting",
          title: connection.type === "local" ? "Terminal" : `SSH: ${connection.host}`,
          created_at: Date.now(),
        };

        dispatch({ type: "ADD_SESSION", payload: sessionInfo });
        dispatch({ type: "SET_LOADING", payload: false });

        return sessionId;
      } catch (error) {
        const message = error instanceof Error ? error.message : "创建会话失败";
        dispatch({ type: "SET_ERROR", payload: message });
        throw error;
      }
    },
    [rpc]
  );

  const closeSession = useCallback(
    async (sessionId: string): Promise<void> => {
      try {
        await rpc.closeSession(sessionId);
        dispatch({ type: "REMOVE_SESSION", payload: sessionId });
      } catch (error) {
        console.error("Failed to close session:", error);
        dispatch({ type: "REMOVE_SESSION", payload: sessionId });
      }
    },
    [rpc]
  );

  const setActiveSession = useCallback((sessionId: string): void => {
    dispatch({ type: "SET_ACTIVE_SESSION", payload: sessionId });
  }, []);

  const updateSessionStatus = useCallback(
    (sessionId: string, status: SessionStatus, exitCode?: number): void => {
      dispatch({
        type: "UPDATE_SESSION_STATUS",
        payload: { sessionId, status, exitCode },
      });
    },
    []
  );

  const updateSessionTitle = useCallback((sessionId: string, title: string): void => {
    dispatch({
      type: "UPDATE_SESSION_TITLE",
      payload: { sessionId, title },
    });
  }, []);

  const updateSessionCwd = useCallback((sessionId: string, cwd: string): void => {
    dispatch({
      type: "UPDATE_SESSION_CWD",
      payload: { sessionId, cwd },
    });
  }, []);

  // 监听 RPC 通知
  React.useEffect(() => {
    const unsubscribeStatus = rpc.onStatus((sessionId, status, exitCode) => {
      updateSessionStatus(sessionId, status as SessionStatus, exitCode);
    });

    const unsubscribeTitle = rpc.onTitle((sessionId, title) => {
      updateSessionTitle(sessionId, title);
    });

    return () => {
      unsubscribeStatus();
      unsubscribeTitle();
    };
  }, [rpc, updateSessionStatus, updateSessionTitle]);

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
 */
export function useSessionStore(): SessionContextValue {
  const context = useContext(SessionContext);
  if (!context) {
    throw new Error("useSessionStore must be used within a SessionProvider");
  }
  return context;
}

export function useSessionCount(): number {
  const { state } = useSessionStore();
  return state.sessions.size;
}

export function useActiveSessionId(): string | null {
  const { state } = useSessionStore();
  return state.activeSessionId;
}

export function useSession(sessionId: string): SessionInfo | undefined {
  const { state } = useSessionStore();
  return state.sessions.get(sessionId);
}
