/**
 * @proxycast/plugin-components 类型声明
 *
 * 在 CI 环境或独立构建时使用
 */

declare module "@proxycast/plugin-components" {
  import type React from "react";

  // ============================================================================
  // Plugin SDK 类型
  // ============================================================================

  export type PluginId = string;
  export type Unsubscribe = () => void;
  export type RpcNotificationCallback<T = unknown> = (params: T) => void;

  export interface RpcApi {
    call<T = unknown>(method: string, params?: unknown): Promise<T>;
    on<T = unknown>(event: string, callback: RpcNotificationCallback<T>): Unsubscribe;
    off<T = unknown>(event: string, callback: RpcNotificationCallback<T>): void;
    isConnected(): boolean;
    connect(): Promise<void>;
    disconnect(): Promise<void>;
  }

  export interface NotificationApi {
    success(message: string): void;
    error(message: string): void;
    info(message: string): void;
    warning(message: string): void;
  }

  export interface EventsApi {
    emit(event: string, data?: unknown): void;
    on<T = unknown>(event: string, callback: (data: T) => void): Unsubscribe;
    once<T = unknown>(event: string, callback: (data: T) => void): void;
  }

  export interface StorageApi {
    get(key: string): Promise<string | null>;
    set(key: string, value: string): Promise<void>;
    delete(key: string): Promise<void>;
    keys(): Promise<string[]>;
  }

  export interface PluginSDK {
    readonly pluginId: PluginId;
    readonly rpc: RpcApi;
    readonly notification: NotificationApi;
    readonly events: EventsApi;
    readonly storage: StorageApi;
    // 其他 API 省略，terminal 插件主要使用 rpc
  }

  // ============================================================================
  // UI 组件
  // ============================================================================

  export const Button: React.FC<{
    variant?: "default" | "outline" | "ghost" | "destructive";
    size?: "default" | "sm" | "lg" | "icon";
    disabled?: boolean;
    onClick?: () => void;
    className?: string;
    children?: React.ReactNode;
  }>;

  export const Card: React.FC<{
    className?: string;
    children?: React.ReactNode;
  }>;

  export const CardHeader: React.FC<{
    className?: string;
    children?: React.ReactNode;
  }>;

  export const CardTitle: React.FC<{
    className?: string;
    children?: React.ReactNode;
  }>;

  export const CardDescription: React.FC<{
    className?: string;
    children?: React.ReactNode;
  }>;

  export const CardContent: React.FC<{
    className?: string;
    children?: React.ReactNode;
  }>;

  export const Badge: React.FC<{
    variant?: "default" | "secondary" | "outline" | "destructive";
    className?: string;
    children?: React.ReactNode;
  }>;

  export const Input: React.FC<{
    type?: string;
    value?: string;
    onChange?: (e: React.ChangeEvent<HTMLInputElement>) => void;
    placeholder?: string;
    className?: string;
    disabled?: boolean;
  }>;

  export const Modal: React.FC<{
    isOpen: boolean;
    onClose: () => void;
    maxWidth?: string;
    children?: React.ReactNode;
  }>;

  // ============================================================================
  // 工具函数
  // ============================================================================

  export function cn(...classes: (string | undefined | null | false)[]): string;

  export const toast: {
    success(message: string): void;
    error(message: string): void;
    info(message: string): void;
    warning(message: string): void;
  };

  // ============================================================================
  // 图标
  // ============================================================================

  export const Loader2: React.FC<{ className?: string }>;
  export const AlertCircle: React.FC<{ className?: string }>;
  export const Plus: React.FC<{ className?: string }>;
  export const X: React.FC<{ className?: string }>;
  export const Terminal: React.FC<{ className?: string }>;
  export const RefreshCw: React.FC<{ className?: string }>;
  export const Settings: React.FC<{ className?: string }>;
  export const Search: React.FC<{ className?: string }>;
}
