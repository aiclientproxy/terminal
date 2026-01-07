/**
 * Terminal 主组件
 *
 * 终端的主要容器组件，管理终端实例和生命周期。
 * 集成 xterm.js 渲染、RPC 通信和焦点管理。
 * 使用 RpcContext 进行通信。
 *
 * @module components/Terminal/Terminal
 */

import React, { useEffect, useCallback, useImperativeHandle, forwardRef } from "react";
import { useTerminal } from "@/hooks/useTerminal";
import { useRpc } from "@/lib/rpc-context";
import type { TerminalConfig } from "@/types/terminal";
import { defaultConfig } from "@/types/terminal";

export interface TerminalProps {
  /** 会话 ID */
  sessionId?: string;
  /** 终端配置 */
  config?: TerminalConfig;
  /** 自定义类名 */
  className?: string;
  /** 是否自动聚焦 */
  autoFocus?: boolean;
  /** 会话状态变化回调 */
  onStatusChange?: (status: string, exitCode?: number) => void;
  /** 标题变化回调 */
  onTitleChange?: (title: string) => void;
}

export interface TerminalRef {
  focus: () => void;
  clear: () => void;
  write: (data: string) => void;
  search: (query: string, options?: { caseSensitive?: boolean; regex?: boolean }) => boolean;
  searchNext: () => boolean;
  searchPrevious: () => boolean;
  clearSearch: () => void;
  getSize: () => { cols: number; rows: number };
}

/**
 * 终端组件
 */
export const Terminal = forwardRef<TerminalRef, TerminalProps>(
  ({ sessionId, config = defaultConfig, className, autoFocus = false, onStatusChange, onTitleChange }, ref) => {
    const { sendInput, resize } = useRpc(
      undefined,
      useCallback(
        (sid: string, status: string, exitCode?: number) => {
          if (sid === sessionId) {
            onStatusChange?.(status, exitCode);
          }
        },
        [sessionId, onStatusChange]
      )
    );

    const handleData = useCallback(
      (data: string) => {
        if (sessionId) {
          sendInput(sessionId, data).catch((err) => {
            console.error("Failed to send input:", err);
          });
        }
      },
      [sessionId, sendInput]
    );

    const handleResize = useCallback(
      (cols: number, rows: number) => {
        if (sessionId) {
          resize(sessionId, { cols, rows }).catch((err) => {
            console.error("Failed to resize:", err);
          });
        }
      },
      [sessionId, resize]
    );

    const {
      containerRef,
      write,
      focus,
      clear,
      search,
      searchNext,
      searchPrevious,
      clearSearch,
      getSize,
      isReady,
    } = useTerminal({
      config,
      sessionId,
      onData: handleData,
      onResize: handleResize,
      onTitleChange,
    });

    useImperativeHandle(
      ref,
      () => ({
        focus,
        clear,
        write,
        search,
        searchNext,
        searchPrevious,
        clearSearch,
        getSize,
      }),
      [focus, clear, write, search, searchNext, searchPrevious, clearSearch, getSize]
    );

    useEffect(() => {
      if (autoFocus && isReady) {
        focus();
      }
    }, [autoFocus, isReady, focus]);

    const handleClick = useCallback(() => {
      focus();
    }, [focus]);

    const handleKeyDown = useCallback((_e: React.KeyboardEvent) => {
      // 阻止某些快捷键的默认行为
    }, []);

    return (
      <div
        ref={containerRef}
        className={`terminal-container w-full h-full ${className || ""}`}
        onClick={handleClick}
        onKeyDown={handleKeyDown}
        tabIndex={0}
        role="application"
        aria-label="Terminal"
      />
    );
  }
);

Terminal.displayName = "Terminal";
