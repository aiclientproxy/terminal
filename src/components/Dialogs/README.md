# Dialogs 组件

对话框组件集合，提供各种模态对话框。

## 文件索引

| 文件 | 描述 |
|------|------|
| `index.ts` | 组件导出入口 |
| `NewConnectionDialog.tsx` | 新建连接对话框 |

## NewConnectionDialog

新建连接对话框组件，支持本地终端和 SSH 远程连接两种模式。

### 功能特性

**本地终端选项：**
- 自定义 shell 路径（可选，默认使用系统 shell）
- 工作目录设置（可选）
- 环境变量配置（可选，KEY=VALUE 格式）

**SSH 连接选项：**
- 主机地址（必填）
- 端口号（默认 22）
- 用户名（可选）
- 认证方式：密码或私钥文件

### 使用示例

```tsx
import { NewConnectionDialog } from '@/components/Dialogs';

<NewConnectionDialog
  onConnect={(connection) => {
    // connection 类型为 ConnectionType
    // { type: 'local', shell_path?, cwd?, env? }
    // 或 { type: 'ssh', host, port?, user?, password?, identity_file? }
    createSession(connection);
  }}
  onClose={() => setShowDialog(false)}
/>
```

### Props

| 属性 | 类型 | 必填 | 描述 |
|------|------|------|------|
| `onConnect` | `(connection: ConnectionType) => void` | 是 | 连接回调 |
| `onClose` | `() => void` | 是 | 关闭回调 |
| `showAdvanced` | `boolean` | 否 | 是否默认展开高级选项 |

### 键盘快捷键

- `Enter` - 确认连接
- `Escape` - 关闭对话框
