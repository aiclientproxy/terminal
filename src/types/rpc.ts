/**
 * RPC 类型定义
 *
 * 与 Rust 后端的 JSON-RPC 通信类型。
 */

/**
 * 终端尺寸
 */
export interface TermSize {
  rows: number;
  cols: number;
}

/**
 * 本地连接配置
 */
export interface LocalConnection {
  type: 'local';
  shell_path?: string;
  cwd?: string;
  env?: Record<string, string>;
}

/**
 * SSH 连接配置
 */
export interface SshConnection {
  type: 'ssh';
  host: string;
  port?: number;
  user?: string;
  identity_file?: string;
  password?: string;
}

/**
 * 连接类型
 */
export type ConnectionType = LocalConnection | SshConnection;

/**
 * 会话状态
 */
export type SessionStatus = 'init' | 'connecting' | 'running' | 'done' | 'error';

/**
 * 会话信息
 */
export interface SessionInfo {
  id: string;
  connection_type: ConnectionType;
  status: SessionStatus;
  title?: string;
  cwd?: string;
  exit_code?: number;
  created_at: number;
}

/**
 * 创建会话请求
 */
export interface CreateSessionRequest {
  connection: ConnectionType;
  term_size: TermSize;
}

/**
 * 创建会话响应
 */
export interface CreateSessionResponse {
  session_id: string;
}

/**
 * 输入请求
 */
export interface InputRequest {
  session_id: string;
  data: string; // base64 encoded
}

/**
 * 调整大小请求
 */
export interface ResizeRequest {
  session_id: string;
  term_size: TermSize;
}

/**
 * 输出通知
 */
export interface OutputNotification {
  session_id: string;
  data: string; // base64 encoded
}

/**
 * 状态通知
 */
export interface SessionStatusNotification {
  session_id: string;
  status: SessionStatus;
  exit_code?: number;
}

/**
 * JSON-RPC 请求
 */
export interface JsonRpcRequest {
  jsonrpc: '2.0';
  method: string;
  params?: unknown;
  id: string | number;
}

/**
 * JSON-RPC 响应
 */
export interface JsonRpcResponse<T = unknown> {
  jsonrpc: '2.0';
  result?: T;
  error?: JsonRpcError;
  id: string | number | null;
}

/**
 * JSON-RPC 错误
 */
export interface JsonRpcError {
  code: number;
  message: string;
  data?: unknown;
}

/**
 * JSON-RPC 通知
 */
export interface JsonRpcNotification<T = unknown> {
  jsonrpc: '2.0';
  method: string;
  params?: T;
}
