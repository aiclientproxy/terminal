/**
 * RPC 客户端
 *
 * 与 Rust 后端进行 JSON-RPC 通信。
 * 实现 EventEmitter 模式处理通知。
 */

import type {
  JsonRpcRequest,
  JsonRpcResponse,
  JsonRpcNotification,
  CreateSessionRequest,
  CreateSessionResponse,
  InputRequest,
  ResizeRequest,
  SessionInfo,
  TermSize,
  OutputNotification,
  SessionStatusNotification,
} from '@/types/rpc';

// 通知事件类型
export type NotificationEvents = {
  'terminal.output': OutputNotification;
  'session.status': SessionStatusNotification;
  'session.title': { session_id: string; title: string };
  'session.cwd': { session_id: string; cwd: string };
};

type NotificationMethod = keyof NotificationEvents;
type NotificationHandler<T extends NotificationMethod> = (params: NotificationEvents[T]) => void;

/**
 * 插件宿主接口
 * 由 ProxyCast 主应用提供
 */
interface PluginHost {
  sendMessage(message: string): void;
  onMessage(callback: (message: string) => void): void;
}

/**
 * JSON-RPC 客户端
 * 
 * 实现 JSON-RPC 2.0 协议，通过插件宿主与 Rust 后端通信。
 * 支持请求/响应模式和通知事件。
 */
export class RpcClient {
  private requestId = 0;
  private pendingRequests = new Map<
    number | string,
    { 
      resolve: (value: unknown) => void; 
      reject: (error: Error) => void;
      timeout: ReturnType<typeof setTimeout>;
    }
  >();
  
  // 使用 Map 存储每种通知类型的处理器
  private notificationHandlers = new Map<string, Set<(params: unknown) => void>>();
  
  // 插件宿主引用
  private pluginHost: PluginHost | null = null;
  
  // 请求超时时间（毫秒）
  private readonly REQUEST_TIMEOUT = 30000;

  constructor() {
    // 初始化通知处理器集合
    const methods: NotificationMethod[] = [
      'terminal.output',
      'session.status',
      'session.title',
      'session.cwd',
    ];
    methods.forEach(method => {
      this.notificationHandlers.set(method, new Set());
    });
  }

  /**
   * 初始化与插件宿主的连接
   */
  initialize(pluginHost: PluginHost): void {
    this.pluginHost = pluginHost;
    
    // 监听来自后端的消息
    pluginHost.onMessage((message: string) => {
      this.handleMessage(message);
    });
  }

  /**
   * 处理来自后端的消息
   */
  private handleMessage(message: string): void {
    try {
      const parsed = JSON.parse(message);
      
      // 检查是否是通知（没有 id 字段）
      if (!('id' in parsed) || parsed.id === undefined) {
        this.handleNotification(parsed as JsonRpcNotification);
      } else {
        this.handleResponse(parsed as JsonRpcResponse);
      }
    } catch (error) {
      console.error('Failed to parse RPC message:', error);
    }
  }

  /**
   * 发送请求
   */
  private async request<T>(method: string, params?: unknown): Promise<T> {
    if (!this.pluginHost) {
      throw new Error('RPC client not initialized. Call initialize() first.');
    }

    const id = ++this.requestId;
    const request: JsonRpcRequest = {
      jsonrpc: '2.0',
      method,
      params,
      id,
    };

    return new Promise((resolve, reject) => {
      // 设置超时
      const timeout = setTimeout(() => {
        this.pendingRequests.delete(id);
        reject(new Error(`Request timeout: ${method}`));
      }, this.REQUEST_TIMEOUT);

      this.pendingRequests.set(id, {
        resolve: resolve as (value: unknown) => void,
        reject,
        timeout,
      });

      // 发送请求
      const message = JSON.stringify(request);
      this.pluginHost!.sendMessage(message);
    });
  }

  /**
   * 处理响应
   */
  private handleResponse(response: JsonRpcResponse): void {
    const pending = this.pendingRequests.get(response.id as number | string);
    if (!pending) {
      console.warn('Received response for unknown request:', response.id);
      return;
    }

    // 清除超时
    clearTimeout(pending.timeout);
    this.pendingRequests.delete(response.id as number | string);

    if (response.error) {
      pending.reject(new Error(`RPC Error [${response.error.code}]: ${response.error.message}`));
    } else {
      pending.resolve(response.result);
    }
  }

  /**
   * 处理通知
   */
  private handleNotification(notification: JsonRpcNotification): void {
    const handlers = this.notificationHandlers.get(notification.method);
    if (!handlers || handlers.size === 0) {
      return;
    }

    handlers.forEach(handler => {
      try {
        handler(notification.params);
      } catch (error) {
        console.error(`Error in notification handler for ${notification.method}:`, error);
      }
    });
  }

  /**
   * 订阅通知事件
   */
  on<T extends NotificationMethod>(
    method: T,
    handler: NotificationHandler<T>
  ): () => void {
    const handlers = this.notificationHandlers.get(method);
    if (handlers) {
      handlers.add(handler as (params: unknown) => void);
    }

    // 返回取消订阅函数
    return () => {
      handlers?.delete(handler as (params: unknown) => void);
    };
  }

  /**
   * 取消订阅通知事件
   */
  off<T extends NotificationMethod>(
    method: T,
    handler: NotificationHandler<T>
  ): void {
    const handlers = this.notificationHandlers.get(method);
    if (handlers) {
      handlers.delete(handler as (params: unknown) => void);
    }
  }

  /**
   * 订阅所有通知（通用处理器）
   * @deprecated 使用 on() 方法订阅特定通知类型
   */
  onNotification(
    handler: (method: string, params: unknown) => void
  ): () => void {
    // 为所有通知类型添加处理器
    const wrappedHandlers = new Map<string, (params: unknown) => void>();

    this.notificationHandlers.forEach((_, method) => {
      const wrappedHandler = (params: unknown) => handler(method, params);
      wrappedHandlers.set(method, wrappedHandler);
      this.notificationHandlers.get(method)?.add(wrappedHandler);
    });

    // 返回取消订阅函数
    return () => {
      wrappedHandlers.forEach((wrappedHandler, method) => {
        this.notificationHandlers.get(method)?.delete(wrappedHandler);
      });
    };
  }

  // ============ RPC 方法 ============

  /**
   * 创建新会话
   */
  async createSession(params: CreateSessionRequest): Promise<CreateSessionResponse> {
    return this.request<CreateSessionResponse>('session.create', params);
  }

  /**
   * 发送输入数据到会话
   */
  async sendInput(sessionId: string, data: string): Promise<void> {
    const params: InputRequest = {
      session_id: sessionId,
      data: btoa(data), // base64 编码
    };
    await this.request<null>('session.input', params);
  }

  /**
   * 发送原始输入数据（已 base64 编码）
   */
  async sendRawInput(sessionId: string, base64Data: string): Promise<void> {
    const params: InputRequest = {
      session_id: sessionId,
      data: base64Data,
    };
    await this.request<null>('session.input', params);
  }

  /**
   * 调整会话终端大小
   */
  async resizeSession(sessionId: string, termSize: TermSize): Promise<void> {
    const params: ResizeRequest = {
      session_id: sessionId,
      term_size: termSize,
    };
    await this.request<null>('session.resize', params);
  }

  /**
   * 关闭会话
   */
  async closeSession(sessionId: string): Promise<void> {
    await this.request<null>('session.close', { session_id: sessionId });
  }

  /**
   * 列出所有会话
   */
  async listSessions(): Promise<SessionInfo[]> {
    return this.request<SessionInfo[]>('session.list');
  }

  /**
   * 获取会话信息
   */
  async getSession(sessionId: string): Promise<SessionInfo> {
    return this.request<SessionInfo>('session.get', { session_id: sessionId });
  }

  /**
   * 销毁客户端，清理资源
   */
  destroy(): void {
    // 清除所有待处理请求
    this.pendingRequests.forEach(({ reject, timeout }) => {
      clearTimeout(timeout);
      reject(new Error('RPC client destroyed'));
    });
    this.pendingRequests.clear();

    // 清除所有通知处理器
    this.notificationHandlers.forEach(handlers => handlers.clear());

    this.pluginHost = null;
  }
}

// 导出单例实例
export const rpcClient = new RpcClient();
