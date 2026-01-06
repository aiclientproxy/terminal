# Terminal Plugin

ProxyCast 终端插件，提供本地 PTY 终端和 SSH 远程连接功能。

## 功能特性

- 🖥️ 本地 PTY 终端会话
- 🔐 SSH 远程连接（密码/私钥认证）
- 📑 多标签页管理
- 🔍 终端内搜索
- 🎨 多主题支持
- 📋 OSC 序列处理（工作目录、剪贴板）

## 技术栈

### 后端 (Rust)
- portable-pty - PTY 管理
- russh - SSH 客户端
- tokio - 异步运行时
- serde/serde_json - 序列化

### 前端 (React + TypeScript)
- xterm.js - 终端渲染
- React 18 - UI 框架
- Vite - 构建工具
- TailwindCSS - 样式

## 开发

### 前端开发

```bash
# 安装依赖
npm install

# 开发模式
npm run dev

# 构建
npm run build
```

### 后端开发

```bash
cd src-tauri

# 构建
cargo build

# 运行测试
cargo test

# 发布构建
cargo build --release
```

## 项目结构

```
terminal/
├── plugin/                 # 插件配置和输出
│   ├── plugin.json        # 插件元数据
│   ├── config.json        # 运行时配置
│   └── dist/              # 构建输出
├── src/                    # 前端源码
│   ├── components/        # React 组件
│   ├── hooks/             # React Hooks
│   ├── lib/               # 工具库
│   ├── store/             # 状态管理
│   └── types/             # TypeScript 类型
└── src-tauri/             # 后端源码
    └── src/
        ├── rpc/           # JSON-RPC 通信
        ├── pty/           # PTY 管理
        ├── ssh/           # SSH 连接
        ├── shell/         # Shell 集成
        └── utils/         # 工具模块
```

## 许可证

MIT
