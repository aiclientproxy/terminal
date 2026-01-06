# 工具库

终端插件的核心工具库模块。

## 文件索引

| 文件 | 描述 |
|------|------|
| `rpc-client.ts` | JSON-RPC 客户端，与 Rust 后端通信 |
| `termwrap.ts` | xterm.js 包装器，封装终端渲染和交互 |
| `theme.ts` | 终端主题配置 |

## 模块说明

### rpc-client.ts

JSON-RPC 2.0 客户端实现，通过插件宿主与 Rust 后端通信。

主要功能：
- 请求/响应模式
- 通知事件订阅 (EventEmitter 模式)
- 30 秒请求超时
- 自动 base64 编码输入数据

### termwrap.ts

xterm.js 终端包装器，封装终端初始化和交互逻辑。

### theme.ts

终端主题配置，定义颜色方案和字体设置。
