# 需求文档

## 简介

ProxyCast Terminal Plugin 是一个功能完整的终端插件，将 Wave Terminal 的终端功能移植到 ProxyCast 应用中。该插件采用与 kiro-provider 相同的插件架构，提供本地 PTY 终端和 SSH 远程连接功能。

## 术语表

- **Terminal_Plugin**: 终端插件主系统，负责协调前后端通信和会话管理
- **PTY_Manager**: 本地伪终端管理器，负责创建和管理本地 shell 会话
- **SSH_Manager**: SSH 连接管理器，负责建立和管理远程 SSH 会话
- **RPC_Server**: JSON-RPC 服务器，处理前端请求并发送通知
- **RPC_Client**: JSON-RPC 客户端，前端用于与后端通信
- **TermWrap**: xterm.js 包装器，封装终端渲染和交互逻辑
- **Session**: 终端会话，可以是本地 PTY 或 SSH 连接
- **OSC_Handler**: 操作系统命令序列处理器，处理 OSC 7/52 等特殊序列

## 需求

### 需求 1: 本地 PTY 终端会话

**用户故事:** 作为开发者，我希望能够创建本地终端会话，以便在本地机器上执行 shell 命令。

#### 验收标准

1. WHEN 用户请求创建本地会话时，THE PTY_Manager SHALL 使用检测到的默认 shell 创建新的 PTY 进程
2. WHEN 本地会话创建成功时，THE PTY_Manager SHALL 返回唯一的会话 ID
3. WHEN 收到用户输入时，THE PTY_Manager SHALL 将数据写入 PTY 的标准输入
4. WHEN PTY 产生输出时，THE Terminal_Plugin SHALL 通过 JSON-RPC 通知将输出数据发送到前端
5. WHEN 会话大小调整时，THE PTY_Manager SHALL 更新 PTY 窗口大小
6. WHEN 会话关闭时，THE PTY_Manager SHALL 终止 PTY 进程并清理资源
7. WHEN shell 进程退出时，THE Terminal_Plugin SHALL 通知前端退出状态

### 需求 2: SSH 远程连接

**用户故事:** 作为开发者，我希望能够通过 SSH 连接到远程服务器，以便从终端管理远程系统。

#### 验收标准

1. WHEN 用户请求 SSH 连接时，THE SSH_Manager SHALL 建立到指定主机的连接
2. WHEN 需要 SSH 认证时，THE SSH_Manager SHALL 支持密码和私钥认证方式
3. WHEN SSH 连接建立后，THE SSH_Manager SHALL 创建用于终端交互的 PTY 通道
4. WHEN 收到 SSH 输入时，THE SSH_Manager SHALL 将数据写入 SSH 通道
5. WHEN SSH 通道产生输出时，THE Terminal_Plugin SHALL 将输出数据发送到前端
6. WHEN SSH 连接失败时，THE Terminal_Plugin SHALL 返回描述性错误消息
7. WHEN SSH 会话关闭时，THE SSH_Manager SHALL 关闭连接并清理资源

### 需求 3: JSON-RPC 通信协议

**用户故事:** 作为插件开发者，我希望有可靠的前后端通信协议，以便构建响应式的终端交互。

#### 验收标准

1. THE RPC_Server SHALL 通过 stdin 接收 JSON-RPC 2.0 请求
2. THE RPC_Server SHALL 通过 stdout 发送 JSON-RPC 2.0 响应
3. THE RPC_Server SHALL 为异步事件发送 JSON-RPC 2.0 通知
4. WHEN 收到请求时，THE RPC_Server SHALL 在 30 秒内处理并返回响应
5. WHEN 收到无效请求时，THE RPC_Server SHALL 返回 JSON-RPC 错误响应
6. FOR ALL 有效的 RPC 请求，序列化后再反序列化 SHALL 产生等价的对象（往返属性）

### 需求 4: 终端渲染和交互

**用户故事:** 作为用户，我希望有响应式的终端界面，以便高效地与 shell 交互。

#### 验收标准

1. WHEN 终端组件挂载时，THE TermWrap SHALL 使用配置的主题和字体设置初始化 xterm.js
2. WHEN 用户在终端中输入时，THE TermWrap SHALL 通过 RPC 将输入数据发送到后端
3. WHEN 收到终端输出时，THE TermWrap SHALL 将数据写入 xterm.js 进行渲染
4. WHEN 终端容器大小改变时，THE TermWrap SHALL 重新计算尺寸并通知后端
5. WHEN 支持 WebGL 时，THE TermWrap SHALL 使用 WebGL 插件提升渲染性能
6. WHEN 终端获得焦点时，THE TermWrap SHALL 捕获键盘输入

### 需求 5: 多标签页管理

**用户故事:** 作为用户，我希望能够在标签页中管理多个终端会话，以便同时使用多个 shell。

#### 验收标准

1. WHEN 用户创建新标签页时，THE Terminal_Plugin SHALL 创建新会话并在新标签页中显示
2. WHEN 用户切换标签页时，THE Terminal_Plugin SHALL 显示对应的终端会话
3. WHEN 用户关闭标签页时，THE Terminal_Plugin SHALL 关闭关联的会话并释放资源
4. WHEN 会话状态改变时，THE Terminal_Plugin SHALL 相应更新标签页指示器
5. THE Terminal_Plugin SHALL 在切换标签页时保持会话状态

### 需求 6: 终端搜索功能

**用户故事:** 作为用户，我希望能够在终端输出中搜索，以便在滚动缓冲区中找到特定文本。

#### 验收标准

1. WHEN 用户打开搜索栏时，THE TermWrap SHALL 激活搜索插件
2. WHEN 用户输入搜索查询时，THE TermWrap SHALL 在终端中高亮匹配的文本
3. WHEN 用户导航搜索结果时，THE TermWrap SHALL 滚动到并高亮下一个/上一个匹配项
4. WHEN 用户关闭搜索栏时，THE TermWrap SHALL 清除搜索高亮
5. THE TermWrap SHALL 支持区分大小写和正则表达式搜索选项

### 需求 7: OSC 序列处理

**用户故事:** 作为开发者，我希望有 shell 集成功能，以便获得增强的终端功能。

#### 验收标准

1. WHEN 收到 OSC 7（工作目录）时，THE OSC_Handler SHALL 解析目录路径并更新会话元数据
2. WHEN 收到 OSC 52（剪贴板）时，THE OSC_Handler SHALL 将解码后的内容复制到系统剪贴板
3. WHEN 收到无效的 OSC 序列时，THE OSC_Handler SHALL 忽略它而不崩溃
4. WHEN OSC 52 数据超过大小限制时，THE OSC_Handler SHALL 拒绝该操作

### 需求 8: Shell 检测和配置

**用户故事:** 作为用户，我希望终端使用我偏好的 shell，以便在熟悉的环境中工作。

#### 验收标准

1. WHEN 未指定 shell 时，THE PTY_Manager SHALL 检测并使用系统默认 shell
2. WHEN 提供自定义 shell 路径时，THE PTY_Manager SHALL 使用指定的 shell
3. WHEN 指定工作目录时，THE PTY_Manager SHALL 在该目录中启动 shell
4. WHEN 提供环境变量时，THE PTY_Manager SHALL 将它们传递给 shell 进程
5. THE PTY_Manager SHALL 设置 TERM=xterm-256color 以实现正确的终端模拟

### 需求 9: 插件配置和元数据

**用户故事:** 作为 ProxyCast 用户，我希望终端插件能够无缝集成，以便像使用其他插件一样使用它。

#### 验收标准

1. THE Terminal_Plugin SHALL 提供包含必需元数据的有效 plugin.json
2. THE Terminal_Plugin SHALL 为 macOS、Linux 和 Windows 提供平台特定的二进制文件
3. THE Terminal_Plugin SHALL 提供用于运行时配置的 config.json
4. WHEN 插件加载时，THE Terminal_Plugin SHALL 在工具面板中注册其 UI 界面
5. THE Terminal_Plugin SHALL 支持 plugin.json 中指定的最低 ProxyCast 版本

### 需求 10: 错误处理和恢复

**用户故事:** 作为用户，我希望有优雅的错误处理，以便理解并从问题中恢复。

#### 验收标准

1. WHEN PTY 创建失败时，THE PTY_Manager SHALL 返回描述性错误消息
2. WHEN SSH 连接失败时，THE SSH_Manager SHALL 返回连接错误详情
3. WHEN 后端进程崩溃时，THE Terminal_Plugin SHALL 在 UI 中显示错误状态
4. WHEN 会话遇到错误时，THE Terminal_Plugin SHALL 将会话状态更新为 'error'
5. IF 发生意外错误，THEN THE Terminal_Plugin SHALL 记录错误并继续运行
