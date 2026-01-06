# Search 组件

终端搜索相关组件。

## 文件索引

| 文件 | 描述 |
|------|------|
| `SearchBar.tsx` | 搜索栏组件，提供搜索输入、导航和选项 |
| `index.ts` | 模块导出 |

## 组件

### SearchBar

终端搜索栏组件，提供：
- 搜索输入框（支持实时搜索）
- 上一个/下一个导航按钮
- 区分大小写选项
- 正则表达式选项
- 显示匹配数量

#### 使用示例

```tsx
import { SearchBar } from '@/components/Search';

<SearchBar
  onSearch={(query, options) => terminal.search(query, options)}
  onNext={() => terminal.searchNext()}
  onPrevious={() => terminal.searchPrevious()}
  onClose={() => setShowSearch(false)}
  searchResult={{ currentIndex: 1, totalMatches: 5 }}
/>
```

#### 键盘快捷键

- `Enter`: 下一个匹配项
- `Shift+Enter`: 上一个匹配项
- `Escape`: 关闭搜索栏
