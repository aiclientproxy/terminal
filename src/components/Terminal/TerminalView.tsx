/**
 * TerminalView 组件
 *
 * 终端视图组件，包含终端、搜索栏和状态栏。
 * 提供完整的终端交互界面，集成搜索功能。
 *
 * @module components/Terminal/TerminalView
 */

import React, { useRef, useCallback, useState, useEffect } from 'react';
import { Terminal, TerminalRef } from './Terminal';
import { SearchBar, SearchResult } from '@/components/Search';
import type { TerminalConfig } from '@/types/terminal';
import type { SessionStatus } from '@/types/rpc';

export interface TerminalViewProps {
  /** 会话 ID */
  sessionId?: string;
  /** 终端配置 */
  config?: TerminalConfig;
  /** 是否自动聚焦 */
  autoFocus?: boolean;
  /** 会话状态变化回调 */
  onStatusChange?: (status: SessionStatus, exitCode?: number) => void;
  /** 标题变化回调 */
  onTitleChange?: (title: string) => void;
  /** 自定义类名 */
  className?: string;
}

/**
 * 终端视图组件
 *
 * 包装 Terminal 组件，提供：
 * - 终端渲染
 * - 搜索功能（Ctrl/Cmd+F 打开）
 * - 状态显示
 * - 工具栏（可扩展）
 *
 * @example
 * ```tsx
 * <TerminalView
 *   sessionId="session-123"
 *   autoFocus
 *   onStatusChange={(status) => updateSessionStatus(status)}
 * />
 * ```
 */
export const TerminalView: React.FC<TerminalViewProps> = ({
  sessionId,
  config,
  autoFocus = true,
  onStatusChange,
  onTitleChange,
  className,
}) => {
  const terminalRef = useRef<TerminalRef>(null);
  const [status, setStatus] = useState<SessionStatus>('init');
  const [showSearch, setShowSearch] = useState(false);
  const [searchResult, setSearchResult] = useState<SearchResult | undefined>(undefined);

  // 处理状态变化
  const handleStatusChange = useCallback(
    (newStatus: string, exitCode?: number) => {
      setStatus(newStatus as SessionStatus);
      onStatusChange?.(newStatus as SessionStatus, exitCode);
    },
    [onStatusChange]
  );

  // 聚焦终端
  const focusTerminal = useCallback(() => {
    terminalRef.current?.focus();
  }, []);

  // 打开搜索栏
  const openSearch = useCallback(() => {
    setShowSearch(true);
  }, []);

  // 关闭搜索栏
  const closeSearch = useCallback(() => {
    setShowSearch(false);
    setSearchResult(undefined);
    terminalRef.current?.clearSearch();
    // 关闭搜索后聚焦终端
    setTimeout(() => {
      terminalRef.current?.focus();
    }, 0);
  }, []);

  // 执行搜索
  const handleSearch = useCallback((query: string, options: { caseSensitive: boolean; regex: boolean }) => {
    if (!query) {
      setSearchResult(undefined);
      terminalRef.current?.clearSearch();
      return;
    }

    const found = terminalRef.current?.search(query, options);
    // 由于 xterm.js SearchAddon 不提供匹配计数，我们只能显示是否找到
    // 如果需要精确计数，需要自行实现搜索逻辑
    setSearchResult({
      currentIndex: found ? 1 : 0,
      totalMatches: found ? 1 : 0, // 简化显示，实际上 SearchAddon 不提供总数
    });
  }, []);

  // 搜索下一个
  const handleSearchNext = useCallback(() => {
    const found = terminalRef.current?.searchNext();
    if (searchResult && found) {
      // 更新当前索引（简化实现）
      setSearchResult((prev) => prev ? { ...prev, currentIndex: prev.currentIndex } : undefined);
    }
  }, [searchResult]);

  // 搜索上一个
  const handleSearchPrevious = useCallback(() => {
    const found = terminalRef.current?.searchPrevious();
    if (searchResult && found) {
      // 更新当前索引（简化实现）
      setSearchResult((prev) => prev ? { ...prev, currentIndex: prev.currentIndex } : undefined);
    }
  }, [searchResult]);

  // 监听键盘快捷键
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      // Ctrl/Cmd + F 打开搜索
      if ((e.ctrlKey || e.metaKey) && e.key === 'f') {
        e.preventDefault();
        openSearch();
      }
      // Escape 关闭搜索（如果搜索栏打开）
      if (e.key === 'Escape' && showSearch) {
        e.preventDefault();
        closeSearch();
      }
    };

    document.addEventListener('keydown', handleKeyDown);
    return () => {
      document.removeEventListener('keydown', handleKeyDown);
    };
  }, [openSearch, closeSearch, showSearch]);

  return (
    <div className={`flex flex-col h-full bg-[#1e1e1e] ${className || ''}`}>
      {/* 搜索栏 */}
      {showSearch && (
        <SearchBar
          onSearch={handleSearch}
          onNext={handleSearchNext}
          onPrevious={handleSearchPrevious}
          onClose={closeSearch}
          searchResult={searchResult}
        />
      )}

      {/* 终端主体 */}
      <div className="flex-1 min-h-0" onClick={focusTerminal}>
        <Terminal
          ref={terminalRef}
          sessionId={sessionId}
          config={config}
          autoFocus={autoFocus && !showSearch}
          onStatusChange={handleStatusChange}
          onTitleChange={onTitleChange}
        />
      </div>

      {/* 状态栏（可选） */}
      {status === 'error' && (
        <div className="px-2 py-1 text-xs text-red-400 bg-red-900/20 border-t border-red-800">
          会话出错
        </div>
      )}
      {status === 'done' && (
        <div className="px-2 py-1 text-xs text-gray-400 bg-gray-800/50 border-t border-gray-700">
          会话已结束
        </div>
      )}
    </div>
  );
};
