# Components

Terminal Plugin 的 React 组件集合。

## 文件索引

| 目录 | 描述 |
|------|------|
| `Dialogs/` | 对话框组件（新建连接等） |
| `Search/` | 搜索功能组件 |
| `Terminal/` | 终端核心组件（终端视图、标签页等） |

## 组件架构

```
components/
├── Dialogs/
│   ├── NewConnectionDialog.tsx  # 新建连接对话框
│   └── index.ts
├── Search/
│   ├── SearchBar.tsx            # 搜索栏组件
│   └── index.ts
└── Terminal/
    ├── Terminal.tsx             # 终端核心组件
    ├── TerminalTabs.tsx         # 标签页管理
    ├── TerminalView.tsx         # 终端视图（含搜索集成）
    └── index.tsx
```

## 使用说明

所有组件都通过各自目录的 `index.ts` 导出，可以直接从目录导入：

```tsx
import { NewConnectionDialog } from '@/components/Dialogs';
import { SearchBar } from '@/components/Search';
import { Terminal, TerminalTabs, TerminalView } from '@/components/Terminal';
```
