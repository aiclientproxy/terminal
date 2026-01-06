/**
 * useRpc Hook
 *
 * RPC 通信的 React Hook。
 */

import { useEffect, useCallback } from 'react';
import { rpcClient } from '@/lib/rpc-client';
import type { ConnectionType, TermSize, SessionInfo } from '@/types/rpc';

export interface UseRpcResult {
  createSession: (connection: ConnectionType, termSize: TermSize) => Promise<string>;
  sendInput: (sessionId: string, data: string) => Promise<void>;
  resize: (sessionId: string, termSize: TermSize) => Promise<void>;
  closeSession: (sessionId: string) => Promise<void>;
  listSessions: () => Promise<SessionInfo[]>;
  getSession: (sessionId: string) => Promise<SessionInfo>;
}

/**
 * RPC Hook
 */
export function useRpc(
  onOutput?: (sessionId: string, data: string) => void,
  onStatusChange?: (sessionId: string, status: string, exitCode?: number) => void
): UseRpcResult {
  useEffect(() => {
    const unsubscribe = rpcClient.onNotification((method, params) => {
      const p = params as Record<string, unknown>;

      switch (method) {
        case 'terminal.output':
          onOutput?.(p.session_id as string, p.data as string);
          break;
        case 'session.status':
          onStatusChange?.(
            p.session_id as string,
            p.status as string,
            p.exit_code as number | undefined
          );
          break;
      }
    });

    return unsubscribe;
  }, [onOutput, onStatusChange]);

  const createSession = useCallback(
    async (connection: ConnectionType, termSize: TermSize) => {
      const response = await rpcClient.createSession({ connection, term_size: termSize });
      return response.session_id;
    },
    []
  );

  const sendInput = useCallback(async (sessionId: string, data: string) => {
    // rpcClient.sendInput 内部会进行 base64 编码
    await rpcClient.sendInput(sessionId, data);
  }, []);

  const resize = useCallback(async (sessionId: string, termSize: TermSize) => {
    await rpcClient.resizeSession(sessionId, termSize);
  }, []);

  const closeSession = useCallback(async (sessionId: string) => {
    await rpcClient.closeSession(sessionId);
  }, []);

  const listSessions = useCallback(async () => {
    return rpcClient.listSessions();
  }, []);

  const getSession = useCallback(async (sessionId: string) => {
    return rpcClient.getSession(sessionId);
  }, []);

  return {
    createSession,
    sendInput,
    resize,
    closeSession,
    listSessions,
    getSession,
  };
}
