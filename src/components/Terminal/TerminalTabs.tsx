/**
 * TerminalTabs 组件
 *
 * 多标签页管理组件，支持标签页切换、新建、关闭和状态指示。
 *
 * @module components/Terminal/TerminalTabs
 */

import React, { useCallback } from 'react';
import type { SessionStatus } from '@/types/rpc';

/**
 * 标签页数据
 */
export interface Tab {
  /** 标签页 ID（对应会话 ID） */
  id: string;
  /** 标签页标题 */
  title: string;
  /** 会话状态 */
  status: SessionStatus;
  /** 是否为 SSH 连接 */
  isSSH?: boolean;
}

/**
 * TerminalTabs 组件属性
 */
export interface TerminalTabsProps {
  /** 标签页列表 */
  tabs: Tab[];
  /** 当前活动标签页 ID */
  activeTabId?: string | null;
  /** 标签页选择回调 */
  onTabSelect: (tabId: string) => void;
  /** 标签页关闭回调 */
  onTabClose: (tabId: string) => void;
  /** 新建标签页回调 */
  onNewTab: () => void;
  /** 自定义类名 */
  className?: string;
}

/**
 * 状态指示器颜色映射
 */
const statusColors: Record<SessionStatus, string> = {
  init: 'bg-gray-500',
  connecting: 'bg-yellow-500 animate-pulse',
  running: 'bg-green-500',
  done: 'bg-gray-400',
  error: 'bg-red-500',
};

/**
 * 状态提示文本映射
 */
const statusTitles: Record<SessionStatus, string> = {
  init: '初始化中',
  connecting: '连接中',
  running: '运行中',
  done: '已结束',
  error: '出错',
};

/**
 * 状态指示器组件
 */
const StatusIndicator: React.FC<{ status: SessionStatus }> = ({ status }) => (
  <span
    className={`inline-block w-2 h-2 rounded-full mr-2 ${statusColors[status]}`}
    title={statusTitles[status]}
  />
);

/**
 * 终端图标组件
 */
const TerminalIcon: React.FC<{ isSSH?: boolean; className?: string }> = ({ isSSH, className }) => (
  <svg
    className={className || 'w-4 h-4 mr-1.5'}
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    strokeWidth="2"
    strokeLinecap="round"
    strokeLinejoin="round"
  >
    {isSSH ? (
      // SSH 图标 - 带锁的终端
      <>
        <rect x="3" y="11" width="18" height="11" rx="2" ry="2" />
        <path d="M7 11V7a5 5 0 0 1 10 0v4" />
      </>
    ) : (
      // 本地终端图标
      <>
        <polyline points="4 17 10 11 4 5" />
        <line x1="12" y1="19" x2="20" y2="19" />
      </>
    )}
  </svg>
);

/**
 * 关闭按钮组件
 */
const CloseButton: React.FC<{ onClick: (e: React.MouseEvent) => void }> = ({ onClick }) => (
  <button
    className="ml-2 p-0.5 rounded hover:bg-gray-600 text-gray-500 hover:text-gray-300 transition-colors"
    onClick={onClick}
    title="关闭标签页"
    aria-label="关闭标签页"
  >
    <svg className="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
      <line x1="18" y1="6" x2="6" y2="18" />
      <line x1="6" y1="6" x2="18" y2="18" />
    </svg>
  </button>
);

/**
 * 新建标签页按钮组件
 */
const NewTabButton: React.FC<{ onClick: () => void }> = ({ onClick }) => (
  <button
    className="flex items-center justify-center px-3 py-2 text-gray-400 hover:text-gray-200 hover:bg-gray-700/50 transition-colors"
    onClick={onClick}
    title="新建终端"
    aria-label="新建终端"
  >
    <svg className="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
      <line x1="12" y1="5" x2="12" y2="19" />
      <line x1="5" y1="12" x2="19" y2="12" />
    </svg>
  </button>
);

/**
 * 单个标签页组件
 */
const TabItem: React.FC<{
  tab: Tab;
  isActive: boolean;
  onSelect: () => void;
  onClose: () => void;
}> = ({ tab, isActive, onSelect, onClose }) => {
  const handleClose = useCallback(
    (e: React.MouseEvent) => {
      e.stopPropagation();
      onClose();
    },
    [onClose]
  );

  return (
    <div
      className={`
        group flex items-center px-3 py-2 cursor-pointer border-r border-gray-700
        transition-colors select-none min-w-0 max-w-[200px]
        ${isActive 
          ? 'bg-gray-700 text-gray-100' 
          : 'text-gray-400 hover:bg-gray-700/50 hover:text-gray-200'
        }
      `}
      onClick={onSelect}
      role="tab"
      aria-selected={isActive}
      tabIndex={isActive ? 0 : -1}
    >
      <StatusIndicator status={tab.status} />
      <TerminalIcon isSSH={tab.isSSH} className="w-3.5 h-3.5 mr-1.5 flex-shrink-0" />
      <span className="text-sm truncate flex-1">{tab.title}</span>
      <CloseButton onClick={handleClose} />
    </div>
  );
};

/**
 * 终端标签页组件
 *
 * 提供多标签页管理功能：
 * - 标签页切换
 * - 新建/关闭标签页
 * - 状态指示器（运行中、已结束、出错）
 * - SSH/本地终端图标区分
 *
 * @example
 * ```tsx
 * <TerminalTabs
 *   tabs={[
 *     { id: '1', title: 'Terminal', status: 'running' },
 *     { id: '2', title: 'SSH: server', status: 'connecting', isSSH: true },
 *   ]}
 *   activeTabId="1"
 *   onTabSelect={(id) => setActiveTab(id)}
 *   onTabClose={(id) => closeTab(id)}
 *   onNewTab={() => createNewTab()}
 * />
 * ```
 */
export const TerminalTabs: React.FC<TerminalTabsProps> = ({
  tabs,
  activeTabId,
  onTabSelect,
  onTabClose,
  onNewTab,
  className,
}) => {
  // 处理键盘导航
  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (tabs.length === 0) return;

      const currentIndex = tabs.findIndex((t) => t.id === activeTabId);
      let newIndex = currentIndex;

      switch (e.key) {
        case 'ArrowLeft':
          newIndex = currentIndex > 0 ? currentIndex - 1 : tabs.length - 1;
          break;
        case 'ArrowRight':
          newIndex = currentIndex < tabs.length - 1 ? currentIndex + 1 : 0;
          break;
        case 'Home':
          newIndex = 0;
          break;
        case 'End':
          newIndex = tabs.length - 1;
          break;
        default:
          return;
      }

      e.preventDefault();
      onTabSelect(tabs[newIndex].id);
    },
    [tabs, activeTabId, onTabSelect]
  );

  return (
    <div
      className={`flex items-center bg-gray-800 border-b border-gray-700 ${className || ''}`}
      role="tablist"
      aria-label="终端标签页"
      onKeyDown={handleKeyDown}
    >
      {/* 标签页列表 */}
      <div className="flex items-center flex-1 overflow-x-auto scrollbar-thin scrollbar-thumb-gray-600">
        {tabs.map((tab) => (
          <TabItem
            key={tab.id}
            tab={tab}
            isActive={tab.id === activeTabId}
            onSelect={() => onTabSelect(tab.id)}
            onClose={() => onTabClose(tab.id)}
          />
        ))}
      </div>

      {/* 新建标签页按钮 */}
      <NewTabButton onClick={onNewTab} />
    </div>
  );
};

/**
 * 空状态组件 - 当没有标签页时显示
 */
export const EmptyTabsPlaceholder: React.FC<{ onNewTab: () => void }> = ({ onNewTab }) => (
  <div className="flex flex-col items-center justify-center h-full bg-gray-900 text-gray-400">
    <TerminalIcon className="w-16 h-16 mb-4 opacity-50" />
    <p className="text-lg mb-4">没有打开的终端</p>
    <button
      className="px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white rounded transition-colors"
      onClick={onNewTab}
    >
      新建终端
    </button>
  </div>
);
