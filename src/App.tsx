/**
 * Terminal Plugin 主应用组件
 *
 * 提供终端界面，支持本地 PTY 和 SSH 连接。
 * 集成多标签页管理和会话状态。
 * 使用 ProxyCast Plugin SDK 进行 RPC 通信。
 * 参考 waveterm 设计风格。
 *
 * @module App
 */

import React, { useCallback, useMemo, useState, useEffect } from "react";
import { SessionProvider, useSessionStore } from "@/store/session-store";
import { TerminalTabs, EmptyTabsPlaceholder, Tab } from "@/components/Terminal";
import { TerminalView } from "@/components/Terminal/TerminalView";
import { NewConnectionDialog } from "@/components/Dialogs";
import { RpcProvider, useRpcContext } from "@/lib/rpc-context";
import type { SessionStatus, ConnectionType } from "@/types/rpc";
import type { PluginSDK } from "@proxycast/plugin-components";

/**
 * 插件组件 Props
 */
interface TerminalPluginProps {
  sdk: PluginSDK;
  pluginId: string;
}

/**
 * 终端主界面组件
 */
const TerminalMain: React.FC = () => {
  const {
    state,
    getSessions,
    getActiveSession,
    createSession,
    closeSession,
    setActiveSession,
    updateSessionStatus,
    updateSessionTitle,
  } = useSessionStore();

  const { isConnected, connect } = useRpcContext();

  // 新建连接对话框状态
  const [showNewConnectionDialog, setShowNewConnectionDialog] = useState(false);

  // 初始化 RPC 连接
  useEffect(() => {
    if (!isConnected) {
      connect().catch((err: Error) => {
        console.error("Failed to connect RPC:", err);
      });
    }
  }, [isConnected, connect]);

  // 将会话转换为标签页数据
  const tabs = useMemo<Tab[]>(() => {
    return getSessions().map((session) => ({
      id: session.id,
      title:
        session.title ||
        (session.connection_type.type === "local"
          ? "Terminal"
          : `SSH: ${session.connection_type.host}`),
      status: session.status,
      isSSH: session.connection_type.type === "ssh",
    }));
  }, [getSessions]);

  // 获取活动会话
  const activeSession = getActiveSession();

  // 打开新建连接对话框
  const handleNewTab = useCallback(() => {
    setShowNewConnectionDialog(true);
  }, []);

  // 处理连接
  const handleConnect = useCallback(
    async (connection: ConnectionType) => {
      setShowNewConnectionDialog(false);
      try {
        await createSession(connection, { rows: 24, cols: 80 });
      } catch (error) {
        console.error("Failed to create session:", error);
      }
    },
    [createSession]
  );

  // 关闭新建连接对话框
  const handleCloseDialog = useCallback(() => {
    setShowNewConnectionDialog(false);
  }, []);

  // 关闭标签页
  const handleTabClose = useCallback(
    async (tabId: string) => {
      await closeSession(tabId);
    },
    [closeSession]
  );

  // 切换标签页
  const handleTabSelect = useCallback(
    (tabId: string) => {
      setActiveSession(tabId);
    },
    [setActiveSession]
  );

  // 处理会话状态变化
  const handleStatusChange = useCallback(
    (status: SessionStatus, exitCode?: number) => {
      if (activeSession) {
        updateSessionStatus(activeSession.id, status, exitCode);
      }
    },
    [activeSession, updateSessionStatus]
  );

  // 处理标题变化
  const handleTitleChange = useCallback(
    (title: string) => {
      if (activeSession) {
        updateSessionTitle(activeSession.id, title);
      }
    },
    [activeSession, updateSessionTitle]
  );

  // 没有会话时显示空状态
  if (tabs.length === 0) {
    return (
      <div className="flex flex-col h-full w-full" style={{ backgroundColor: 'var(--terminal-bg, #0d1117)' }}>
        <TerminalTabs
          tabs={[]}
          activeTabId={null}
          onTabSelect={handleTabSelect}
          onTabClose={handleTabClose}
          onNewTab={handleNewTab}
        />
        <div className="flex-1">
          <EmptyTabsPlaceholder onNewTab={handleNewTab} />
        </div>

        {showNewConnectionDialog && (
          <NewConnectionDialog
            onConnect={handleConnect}
            onClose={handleCloseDialog}
          />
        )}
      </div>
    );
  }

  return (
    <div className="relative flex flex-col h-full w-full" style={{ backgroundColor: 'var(--terminal-bg, #0d1117)' }}>
      {/* 标签页栏 */}
      <TerminalTabs
        tabs={tabs}
        activeTabId={state.activeSessionId}
        onTabSelect={handleTabSelect}
        onTabClose={handleTabClose}
        onNewTab={handleNewTab}
      />

      {/* 终端视图 */}
      <div className="flex-1 min-h-0 overflow-hidden">
        {activeSession && (
          <TerminalView
            key={activeSession.id}
            sessionId={activeSession.id}
            autoFocus
            onStatusChange={handleStatusChange}
            onTitleChange={handleTitleChange}
          />
        )}
      </div>

      {/* 新建连接对话框 */}
      {showNewConnectionDialog && (
        <NewConnectionDialog
          onConnect={handleConnect}
          onClose={handleCloseDialog}
        />
      )}

      {/* 加载状态 */}
      {state.isLoading && (
        <div className="absolute inset-0 bg-black/60 flex items-center justify-center z-50">
          <div className="text-gray-300 text-sm">创建会话中...</div>
        </div>
      )}

      {/* 错误提示 */}
      {state.error && (
        <div className="absolute bottom-4 right-4 bg-red-600/90 text-white px-4 py-2 rounded-lg shadow-lg text-sm z-50">
          {state.error}
        </div>
      )}
    </div>
  );
};

/**
 * 终端应用组件
 *
 * 包装 RpcProvider 和 SessionProvider 提供状态管理。
 */
export const TerminalApp: React.FC<TerminalPluginProps> = ({ sdk, pluginId: _pluginId }) => {
  return (
    <RpcProvider sdk={sdk}>
      <SessionProvider>
        <TerminalMain />
      </SessionProvider>
    </RpcProvider>
  );
};

export default TerminalApp;
