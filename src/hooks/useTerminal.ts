/**
 * useTerminal Hook
 *
 * 管理终端实例的 React Hook。
 * 集成 xterm.js 包装器和 RPC 通信。
 * 使用 RpcContext 进行通信。
 *
 * @module hooks/useTerminal
 */

import { useEffect, useRef, useCallback, useState } from "react";
import { TermWrap } from "@/lib/termwrap";
import { useRpcContext } from "@/lib/rpc-context";
import type { TerminalConfig } from "@/types/terminal";
import { defaultConfig } from "@/types/terminal";

export interface UseTerminalOptions {
  /** 终端配置 */
  config?: TerminalConfig;
  /** 会话 ID，用于关联 RPC 通信 */
  sessionId?: string;
  /** 数据回调（用户输入） */
  onData?: (data: string) => void;
  /** 大小变化回调 */
  onResize?: (cols: number, rows: number) => void;
  /** 标题变化回调 */
  onTitleChange?: (title: string) => void;
}

export interface UseTerminalResult {
  /** 容器引用 */
  containerRef: React.RefObject<HTMLDivElement>;
  /** 写入数据到终端 */
  write: (data: string) => void;
  /** 聚焦终端 */
  focus: () => void;
  /** 清屏 */
  clear: () => void;
  /** 搜索 */
  search: (query: string, options?: { caseSensitive?: boolean; regex?: boolean }) => boolean;
  /** 搜索下一个 */
  searchNext: () => boolean;
  /** 搜索上一个 */
  searchPrevious: () => boolean;
  /** 清除搜索高亮 */
  clearSearch: () => void;
  /** 获取当前尺寸 */
  getSize: () => { cols: number; rows: number };
  /** 终端是否已就绪 */
  isReady: boolean;
}

/**
 * 终端 Hook
 *
 * 提供终端实例管理，包括：
 * - xterm.js 初始化和生命周期管理
 * - RPC 输出通知监听
 * - 输入/输出处理
 * - 大小调整
 */
export function useTerminal(options: UseTerminalOptions = {}): UseTerminalResult {
  const { config = defaultConfig, sessionId, onData, onResize, onTitleChange } = options;
  const containerRef = useRef<HTMLDivElement>(null);
  const termWrapRef = useRef<TermWrap | null>(null);
  const [isReady, setIsReady] = useState(false);
  const rpc = useRpcContext();

  // 初始化终端
  useEffect(() => {
    if (!containerRef.current) return;

    const termWrap = new TermWrap(config);
    termWrap.onData = onData;
    termWrap.onResize = onResize;
    termWrap.onTitleChange = onTitleChange;
    termWrap.open(containerRef.current);
    termWrapRef.current = termWrap;
    setIsReady(true);

    return () => {
      termWrap.dispose();
      termWrapRef.current = null;
      setIsReady(false);
    };
  }, [config, onData, onResize, onTitleChange]);

  // 监听 RPC 输出通知
  useEffect(() => {
    if (!sessionId) return;

    const unsubscribe = rpc.onOutput((sid, data) => {
      if (sid === sessionId && termWrapRef.current) {
        // 解码 base64 数据
        try {
          const decoded = atob(data);
          termWrapRef.current.write(decoded);
        } catch (e) {
          console.error("Failed to decode terminal output:", e);
        }
      }
    });

    return unsubscribe;
  }, [sessionId, rpc]);

  // 监听标题变化通知
  useEffect(() => {
    if (!sessionId) return;

    const unsubscribe = rpc.onTitle((sid, title) => {
      if (sid === sessionId) {
        onTitleChange?.(title);
      }
    });

    return unsubscribe;
  }, [sessionId, rpc, onTitleChange]);

  const write = useCallback((data: string) => {
    termWrapRef.current?.write(data);
  }, []);

  const focus = useCallback(() => {
    termWrapRef.current?.focus();
  }, []);

  const clear = useCallback(() => {
    termWrapRef.current?.clear();
  }, []);

  const search = useCallback(
    (query: string, options?: { caseSensitive?: boolean; regex?: boolean }) => {
      return termWrapRef.current?.search(query, options) ?? false;
    },
    []
  );

  const searchNext = useCallback(() => {
    return termWrapRef.current?.searchNext() ?? false;
  }, []);

  const searchPrevious = useCallback(() => {
    return termWrapRef.current?.searchPrevious() ?? false;
  }, []);

  const clearSearch = useCallback(() => {
    termWrapRef.current?.clearSearch();
  }, []);

  const getSize = useCallback(() => {
    return termWrapRef.current?.getSize() ?? { cols: 80, rows: 24 };
  }, []);

  return {
    containerRef: containerRef as React.RefObject<HTMLDivElement>,
    write,
    focus,
    clear,
    search,
    searchNext,
    searchPrevious,
    clearSearch,
    getSize,
    isReady,
  };
}
