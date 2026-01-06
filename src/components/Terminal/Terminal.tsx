/**
 * Terminal 主组件
 *
 * 终端的主要容器组件，管理终端实例和生命周期。
 * 集成 xterm.js 渲染、RPC 通信和焦点管理。
 *
 * @module components/Terminal/Terminal
 */

import React, { useEffect, useCallback, useImperativeHandle, forwardRef } from 'react';
import { useTerminal } from '@/hooks/useTerminal';
import { useRpc } from '@/hooks/useRpc';
import type { TerminalConfig } from '@/types/terminal';
import { defaultConfig } from '@/types/terminal';

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
  /** 聚焦终端 */
  focus: () => void;
  /** 清屏 */
  clear: () => void;
  /** 写入数据 */
  write: (data: string) => void;
  /** 搜索 */
  search: (query: string, options?: { caseSensitive?: boolean; regex?: boolean }) => boolean;
  /** 搜索下一个 */
  searchNext: () => boolean;
  /** 搜索上一个 */
  searchPrevious: () => boolean;
  /** 清除搜索 */
  clearSearch: () => void;
  /** 获取尺寸 */
  getSize: () => { cols: number; rows: number };
}

/**
 * 终端组件
 *
 * 提供完整的终端功能：
 * - xterm.js 渲染
 * - 用户输入处理
 * - 后端输出显示
 * - 大小自适应
 * - 焦点管理
 *
 * @example
 * ```tsx
 * <Terminal
 *   sessionId="session-123"
 *   autoFocus
 *   onStatusChange={(status) => console.log('Status:', status)}
 * />
 * ```
 */
export const Terminal = forwardRef<TerminalRef, TerminalProps>(
  ({ sessionId, config = defaultConfig, className, autoFocus = false, onStatusChange, onTitleChange }, ref) => {
    const { sendInput, resize } = useRpc(
      undefined,
      // 状态变化回调
      useCallback(
        (sid: string, status: string, exitCode?: number) => {
          if (sid === sessionId) {
            onStatusChange?.(status, exitCode);
          }
        },
        [sessionId, onStatusChange]
      )
    );

    // 处理用户输入
    const handleData = useCallback(
      (data: string) => {
        if (sessionId) {
          sendInput(sessionId, data).catch((err) => {
            console.error('Failed to send input:', err);
          });
        }
      },
      [sessionId, sendInput]
    );

    // 处理大小变化
    const handleResize = useCallback(
      (cols: number, rows: number) => {
        if (sessionId) {
          resize(sessionId, { cols, rows }).catch((err) => {
            console.error('Failed to resize:', err);
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

    // 暴露方法给父组件
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

    // 自动聚焦
    useEffect(() => {
      if (autoFocus && isReady) {
        focus();
      }
    }, [autoFocus, isReady, focus]);

    // 处理点击聚焦
    const handleClick = useCallback(() => {
      focus();
    }, [focus]);

    // 处理键盘事件（确保终端获得焦点时能接收输入）
    const handleKeyDown = useCallback(
      (e: React.KeyboardEvent) => {
        // 阻止某些快捷键的默认行为
        if (e.ctrlKey && e.key === 'c') {
          // Ctrl+C 应该发送到终端
          return;
        }
      },
      []
    );

    return (
      <div
        ref={containerRef}
        className={`terminal-container w-full h-full ${className || ''}`}
        onClick={handleClick}
        onKeyDown={handleKeyDown}
        tabIndex={0}
        role="application"
        aria-label="Terminal"
      />
    );
  }
);

Terminal.displayName = 'Terminal';
