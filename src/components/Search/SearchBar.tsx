/**
 * SearchBar 组件
 *
 * 终端搜索栏组件，提供搜索输入框、上一个/下一个导航按钮和搜索选项。
 * 支持区分大小写和正则表达式搜索。
 *
 * @module components/Search/SearchBar
 * @requires react
 */

import React, { useState, useRef, useEffect, useCallback } from 'react';

/**
 * 搜索选项
 */
export interface SearchOptions {
  /** 是否区分大小写 */
  caseSensitive: boolean;
  /** 是否使用正则表达式 */
  regex: boolean;
}

/**
 * 搜索结果信息
 */
export interface SearchResult {
  /** 当前匹配索引（从 1 开始） */
  currentIndex: number;
  /** 总匹配数 */
  totalMatches: number;
}

/**
 * SearchBar 组件属性
 */
export interface SearchBarProps {
  /** 搜索回调 - 当搜索查询或选项变化时触发 */
  onSearch: (query: string, options: SearchOptions) => void;
  /** 下一个匹配项回调 */
  onNext: () => void;
  /** 上一个匹配项回调 */
  onPrevious: () => void;
  /** 关闭搜索栏回调 */
  onClose: () => void;
  /** 搜索结果信息（可选，用于显示匹配数） */
  searchResult?: SearchResult;
  /** 初始搜索查询（可选） */
  initialQuery?: string;
  /** 自定义类名 */
  className?: string;
}

/**
 * 搜索图标组件
 */
const SearchIcon: React.FC<{ className?: string }> = ({ className }) => (
  <svg
    className={className || 'w-4 h-4'}
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    strokeWidth="2"
    strokeLinecap="round"
    strokeLinejoin="round"
  >
    <circle cx="11" cy="11" r="8" />
    <line x1="21" y1="21" x2="16.65" y2="16.65" />
  </svg>
);

/**
 * 上箭头图标组件
 */
const ChevronUpIcon: React.FC<{ className?: string }> = ({ className }) => (
  <svg
    className={className || 'w-4 h-4'}
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    strokeWidth="2"
    strokeLinecap="round"
    strokeLinejoin="round"
  >
    <polyline points="18 15 12 9 6 15" />
  </svg>
);

/**
 * 下箭头图标组件
 */
const ChevronDownIcon: React.FC<{ className?: string }> = ({ className }) => (
  <svg
    className={className || 'w-4 h-4'}
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    strokeWidth="2"
    strokeLinecap="round"
    strokeLinejoin="round"
  >
    <polyline points="6 9 12 15 18 9" />
  </svg>
);

/**
 * 关闭图标组件
 */
const CloseIcon: React.FC<{ className?: string }> = ({ className }) => (
  <svg
    className={className || 'w-4 h-4'}
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    strokeWidth="2"
    strokeLinecap="round"
    strokeLinejoin="round"
  >
    <line x1="18" y1="6" x2="6" y2="18" />
    <line x1="6" y1="6" x2="18" y2="18" />
  </svg>
);

/**
 * 搜索栏组件
 *
 * 提供终端搜索功能：
 * - 搜索输入框（支持实时搜索）
 * - 上一个/下一个导航按钮
 * - 区分大小写选项
 * - 正则表达式选项
 * - 显示匹配数量
 *
 * 键盘快捷键：
 * - Enter: 下一个匹配项
 * - Shift+Enter: 上一个匹配项
 * - Escape: 关闭搜索栏
 *
 * @example
 * ```tsx
 * <SearchBar
 *   onSearch={(query, options) => terminal.search(query, options)}
 *   onNext={() => terminal.searchNext()}
 *   onPrevious={() => terminal.searchPrevious()}
 *   onClose={() => setShowSearch(false)}
 *   searchResult={{ currentIndex: 1, totalMatches: 5 }}
 * />
 * ```
 */
export const SearchBar: React.FC<SearchBarProps> = ({
  onSearch,
  onNext,
  onPrevious,
  onClose,
  searchResult,
  initialQuery = '',
  className,
}) => {
  const [query, setQuery] = useState(initialQuery);
  const [caseSensitive, setCaseSensitive] = useState(false);
  const [regex, setRegex] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);

  // 自动聚焦输入框
  useEffect(() => {
    inputRef.current?.focus();
    inputRef.current?.select();
  }, []);

  // 处理搜索查询变化
  const handleQueryChange = useCallback(
    (newQuery: string) => {
      setQuery(newQuery);
      onSearch(newQuery, { caseSensitive, regex });
    },
    [caseSensitive, regex, onSearch]
  );

  // 处理区分大小写切换
  const handleCaseSensitiveToggle = useCallback(() => {
    const newValue = !caseSensitive;
    setCaseSensitive(newValue);
    onSearch(query, { caseSensitive: newValue, regex });
  }, [caseSensitive, query, regex, onSearch]);

  // 处理正则表达式切换
  const handleRegexToggle = useCallback(() => {
    const newValue = !regex;
    setRegex(newValue);
    onSearch(query, { caseSensitive, regex: newValue });
  }, [caseSensitive, query, regex, onSearch]);

  // 处理键盘事件
  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (e.key === 'Enter') {
        e.preventDefault();
        if (e.shiftKey) {
          onPrevious();
        } else {
          onNext();
        }
      } else if (e.key === 'Escape') {
        e.preventDefault();
        onClose();
      }
    },
    [onNext, onPrevious, onClose]
  );

  // 格式化搜索结果显示
  const resultText = searchResult
    ? searchResult.totalMatches > 0
      ? `${searchResult.currentIndex}/${searchResult.totalMatches}`
      : '无结果'
    : '';

  return (
    <div
      className={`flex items-center gap-2 px-3 py-2 bg-gray-800 border-b border-gray-700 ${className || ''}`}
      role="search"
      aria-label="终端搜索"
    >
      {/* 搜索图标 */}
      <SearchIcon className="w-4 h-4 text-gray-500 flex-shrink-0" />

      {/* 搜索输入框 */}
      <input
        ref={inputRef}
        type="text"
        value={query}
        onChange={(e) => handleQueryChange(e.target.value)}
        onKeyDown={handleKeyDown}
        placeholder="搜索..."
        className="flex-1 min-w-0 px-2 py-1 text-sm bg-gray-700 text-gray-200 rounded border border-gray-600 focus:outline-none focus:border-blue-500 placeholder-gray-500"
        aria-label="搜索查询"
      />

      {/* 搜索结果计数 */}
      {resultText && (
        <span
          className={`text-xs flex-shrink-0 ${
            searchResult && searchResult.totalMatches > 0 ? 'text-gray-400' : 'text-yellow-500'
          }`}
          aria-live="polite"
        >
          {resultText}
        </span>
      )}

      {/* 分隔线 */}
      <div className="w-px h-4 bg-gray-600 flex-shrink-0" />

      {/* 区分大小写按钮 */}
      <button
        type="button"
        className={`px-2 py-1 text-xs font-medium rounded transition-colors flex-shrink-0 ${
          caseSensitive
            ? 'bg-blue-600 text-white hover:bg-blue-700'
            : 'bg-gray-700 text-gray-400 hover:bg-gray-600 hover:text-gray-300'
        }`}
        onClick={handleCaseSensitiveToggle}
        title="区分大小写 (Alt+C)"
        aria-pressed={caseSensitive}
        aria-label="区分大小写"
      >
        Aa
      </button>

      {/* 正则表达式按钮 */}
      <button
        type="button"
        className={`px-2 py-1 text-xs font-medium rounded transition-colors flex-shrink-0 ${
          regex
            ? 'bg-blue-600 text-white hover:bg-blue-700'
            : 'bg-gray-700 text-gray-400 hover:bg-gray-600 hover:text-gray-300'
        }`}
        onClick={handleRegexToggle}
        title="正则表达式 (Alt+R)"
        aria-pressed={regex}
        aria-label="正则表达式"
      >
        .*
      </button>

      {/* 分隔线 */}
      <div className="w-px h-4 bg-gray-600 flex-shrink-0" />

      {/* 上一个按钮 */}
      <button
        type="button"
        className="p-1 text-gray-400 hover:text-gray-200 hover:bg-gray-700 rounded transition-colors flex-shrink-0 disabled:opacity-50 disabled:cursor-not-allowed"
        onClick={onPrevious}
        disabled={!query || (searchResult && searchResult.totalMatches === 0)}
        title="上一个 (Shift+Enter)"
        aria-label="上一个匹配项"
      >
        <ChevronUpIcon className="w-4 h-4" />
      </button>

      {/* 下一个按钮 */}
      <button
        type="button"
        className="p-1 text-gray-400 hover:text-gray-200 hover:bg-gray-700 rounded transition-colors flex-shrink-0 disabled:opacity-50 disabled:cursor-not-allowed"
        onClick={onNext}
        disabled={!query || (searchResult && searchResult.totalMatches === 0)}
        title="下一个 (Enter)"
        aria-label="下一个匹配项"
      >
        <ChevronDownIcon className="w-4 h-4" />
      </button>

      {/* 关闭按钮 */}
      <button
        type="button"
        className="p-1 text-gray-400 hover:text-gray-200 hover:bg-gray-700 rounded transition-colors flex-shrink-0"
        onClick={onClose}
        title="关闭 (Esc)"
        aria-label="关闭搜索"
      >
        <CloseIcon className="w-4 h-4" />
      </button>
    </div>
  );
};
