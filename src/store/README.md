# 状态管理模块

本目录包含终端插件的状态管理逻辑。

## 文件索引

| 文件 | 描述 |
|------|------|
| `session-store.ts` | 会话状态管理，提供 React Context 集成 |
| `index.ts` | 模块导出 |

## 会话状态管理

### 使用方式

```tsx
import { SessionProvider, useSessionStore } from '@/store';

// 在应用根组件包装 Provider
function App() {
  return (
    <SessionProvider>
      <TerminalMain />
    </SessionProvider>
  );
}

// 在子组件中使用
function TerminalMain() {
  const {
    state,
    createSession,
    closeSession,
    setActiveSession,
  } = useSessionStore();

  // 创建新会话
  const handleNewSession = async () => {
    const sessionId = await createSession(
      { type: 'local' },
      { rows: 24, cols: 80 }
    );
  };

  return (
    // ...
  );
}
```

### 状态结构

```typescript
interface SessionState {
  sessions: Map<string, SessionInfo>;  // 所有会话
  activeSessionId: string | null;       // 当前活动会话
  isLoading: boolean;                   // 加载状态
  error: string | null;                 // 错误信息
}
```

### 可用 Hooks

- `useSessionStore()` - 获取完整的会话上下文
- `useSessionCount()` - 获取会话数量
- `useActiveSessionId()` - 获取活动会话 ID
- `useSession(sessionId)` - 获取特定会话信息

### RPC 通知集成

SessionProvider 自动监听以下 RPC 通知并更新状态：

- `session.status` - 会话状态变化
- `session.title` - 会话标题变化
- `session.cwd` - 工作目录变化
