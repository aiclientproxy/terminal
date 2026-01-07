/**
 * RPC Context
 *
 * 提供基于 Plugin SDK 的 RPC 通信上下文。
 * 替代原有的 rpcClient 单例模式。
 *
 * @module lib/rpc-context
 */

import React, { createContext, useContext, useCallback, useEffect, useState, useMemo } from "react";
import type { PluginSDK, Unsubscribe } from "@proxycast/plugin-components";
import type {
  ConnectionType,
  TermSize,
  SessionInfo,
  CreateSessionResponse,
  OutputNotification,
  SessionStatusNotification,
} from "@/types/rpc";

/**
 * RPC Context 值类型
 */
export interface RpcContextValue {
  /** 是否已连接 */
  isConnected: boolean;
  /** 连接 RPC */
  connect: () => Promise<void>;
  /** 断开 RPC */
  disconnect: () => Promise<void>;
  /** 创建会话 */
  createSession: (connection: ConnectionType, termSize: TermSize) => Promise<string>;
  /** 发送输入 */
  sendInput: (sessionId: string, data: string) => Promise<void>;
  /** 调整大小 */
  resize: (sessionId: string, termSize: TermSize) => Promise<void>;
  /** 关闭会话 */
  closeSession: (sessionId: string) => Promise<void>;
  /** 列出会话 */
  listSessions: () => Promise<SessionInfo[]>;
  /** 获取会话 */
  getSession: (sessionId: string) => Promise<SessionInfo>;
  /** 订阅输出通知 */
  onOutput: (callback: (sessionId: string, data: string) => void) => Unsubscribe;
  /** 订阅状态通知 */
  onStatus: (callback: (sessionId: string, status: string, exitCode?: number) => void) => Unsubscribe;
  /** 订阅标题通知 */
  onTitle: (callback: (sessionId: string, title: string) => void) => Unsubscribe;
}

const RpcContext = createContext<RpcContextValue | null>(null);

/**
 * RPC Provider Props
 */
interface RpcProviderProps {
  sdk: PluginSDK;
  children: React.ReactNode;
}

/**
 * RPC Provider
 *
 * 提供 RPC 通信功能给子组件。
 */
export function RpcProvider({ sdk, children }: RpcProviderProps): React.ReactElement {
  const [isConnected, setIsConnected] = useState(false);

  // 连接 RPC
  const connect = useCallback(async () => {
    try {
      await sdk.rpc.connect();
      setIsConnected(true);
    } catch (error) {
      console.error("RPC connect failed:", error);
      throw error;
    }
  }, [sdk]);

  // 断开 RPC
  const disconnect = useCallback(async () => {
    try {
      await sdk.rpc.disconnect();
      setIsConnected(false);
    } catch (error) {
      console.error("RPC disconnect failed:", error);
      throw error;
    }
  }, [sdk]);

  // 创建会话
  const createSession = useCallback(
    async (connection: ConnectionType, termSize: TermSize): Promise<string> => {
      const response = await sdk.rpc.call<CreateSessionResponse>("session.create", {
        connection,
        term_size: termSize,
      });
      return response.session_id;
    },
    [sdk]
  );

  // 发送输入
  const sendInput = useCallback(
    async (sessionId: string, data: string): Promise<void> => {
      // Base64 编码
      const base64Data = btoa(data);
      await sdk.rpc.call("session.input", {
        session_id: sessionId,
        data: base64Data,
      });
    },
    [sdk]
  );

  // 调整大小
  const resize = useCallback(
    async (sessionId: string, termSize: TermSize): Promise<void> => {
      await sdk.rpc.call("session.resize", {
        session_id: sessionId,
        term_size: termSize,
      });
    },
    [sdk]
  );

  // 关闭会话
  const closeSession = useCallback(
    async (sessionId: string): Promise<void> => {
      await sdk.rpc.call("session.close", { session_id: sessionId });
    },
    [sdk]
  );

  // 列出会话
  const listSessions = useCallback(async (): Promise<SessionInfo[]> => {
    return sdk.rpc.call<SessionInfo[]>("session.list");
  }, [sdk]);

  // 获取会话
  const getSession = useCallback(
    async (sessionId: string): Promise<SessionInfo> => {
      return sdk.rpc.call<SessionInfo>("session.get", { session_id: sessionId });
    },
    [sdk]
  );

  // 订阅输出通知
  const onOutput = useCallback(
    (callback: (sessionId: string, data: string) => void): Unsubscribe => {
      return sdk.rpc.on<OutputNotification>("terminal.output", (params) => {
        callback(params.session_id, params.data);
      });
    },
    [sdk]
  );

  // 订阅状态通知
  const onStatus = useCallback(
    (callback: (sessionId: string, status: string, exitCode?: number) => void): Unsubscribe => {
      return sdk.rpc.on<SessionStatusNotification>("session.status", (params) => {
        callback(params.session_id, params.status, params.exit_code);
      });
    },
    [sdk]
  );

  // 订阅标题通知
  const onTitle = useCallback(
    (callback: (sessionId: string, title: string) => void): Unsubscribe => {
      return sdk.rpc.on<{ session_id: string; title: string }>("session.title", (params) => {
        callback(params.session_id, params.title);
      });
    },
    [sdk]
  );

  // 组件卸载时断开连接
  useEffect(() => {
    return () => {
      if (isConnected) {
        disconnect().catch(console.error);
      }
    };
  }, [isConnected, disconnect]);

  const value = useMemo<RpcContextValue>(
    () => ({
      isConnected,
      connect,
      disconnect,
      createSession,
      sendInput,
      resize,
      closeSession,
      listSessions,
      getSession,
      onOutput,
      onStatus,
      onTitle,
    }),
    [
      isConnected,
      connect,
      disconnect,
      createSession,
      sendInput,
      resize,
      closeSession,
      listSessions,
      getSession,
      onOutput,
      onStatus,
      onTitle,
    ]
  );

  return React.createElement(RpcContext.Provider, { value }, children);
}

/**
 * 使用 RPC Context 的 Hook
 */
export function useRpcContext(): RpcContextValue {
  const context = useContext(RpcContext);
  if (!context) {
    throw new Error("useRpcContext must be used within a RpcProvider");
  }
  return context;
}

/**
 * 使用 RPC 通信的 Hook（兼容旧 API）
 */
export function useRpc(
  onOutput?: (sessionId: string, data: string) => void,
  onStatusChange?: (sessionId: string, status: string, exitCode?: number) => void
) {
  const rpc = useRpcContext();

  // 订阅通知
  useEffect(() => {
    const unsubscribes: Unsubscribe[] = [];

    if (onOutput) {
      unsubscribes.push(rpc.onOutput(onOutput));
    }

    if (onStatusChange) {
      unsubscribes.push(rpc.onStatus(onStatusChange));
    }

    return () => {
      unsubscribes.forEach((unsub) => unsub());
    };
  }, [rpc, onOutput, onStatusChange]);

  return {
    createSession: rpc.createSession,
    sendInput: rpc.sendInput,
    resize: rpc.resize,
    closeSession: rpc.closeSession,
    listSessions: rpc.listSessions,
    getSession: rpc.getSession,
  };
}
