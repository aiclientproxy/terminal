/**
 * NewConnectionDialog 组件
 *
 * 新建连接对话框，支持本地终端和 SSH 远程连接。
 * 本地终端支持自定义 shell、工作目录和环境变量。
 * SSH 连接支持密码和私钥认证。
 *
 * @module components/Dialogs/NewConnectionDialog
 */

import React, { useState, useCallback, useRef, useEffect } from 'react';
import type { ConnectionType } from '@/types/rpc';

export interface NewConnectionDialogProps {
  /** 连接回调 */
  onConnect: (connection: ConnectionType) => void;
  /** 关闭回调 */
  onClose: () => void;
  /** 是否显示高级选项（默认折叠） */
  showAdvanced?: boolean;
}

type ConnectionMode = 'local' | 'ssh';
type SshAuthMode = 'password' | 'key';

/**
 * 输入框组件
 */
const FormInput: React.FC<{
  label: string;
  value: string;
  onChange: (value: string) => void;
  placeholder?: string;
  type?: 'text' | 'password' | 'number';
  required?: boolean;
  error?: string;
  autoFocus?: boolean;
}> = ({ label, value, onChange, placeholder, type = 'text', required, error, autoFocus }) => (
  <div>
    <label className="block text-sm text-gray-400 mb-1">
      {label}
      {required && <span className="text-red-400 ml-1">*</span>}
    </label>
    <input
      type={type}
      value={value}
      onChange={(e) => onChange(e.target.value)}
      placeholder={placeholder}
      autoFocus={autoFocus}
      className={`w-full px-3 py-2 bg-gray-700 text-gray-200 rounded border 
        ${error ? 'border-red-500' : 'border-gray-600'} 
        focus:outline-none focus:border-blue-500 transition-colors`}
    />
    {error && <p className="text-xs text-red-400 mt-1">{error}</p>}
  </div>
);

/**
 * 文本区域组件（用于环境变量）
 */
const FormTextarea: React.FC<{
  label: string;
  value: string;
  onChange: (value: string) => void;
  placeholder?: string;
  rows?: number;
  hint?: string;
}> = ({ label, value, onChange, placeholder, rows = 3, hint }) => (
  <div>
    <label className="block text-sm text-gray-400 mb-1">{label}</label>
    <textarea
      value={value}
      onChange={(e) => onChange(e.target.value)}
      placeholder={placeholder}
      rows={rows}
      className="w-full px-3 py-2 bg-gray-700 text-gray-200 rounded border border-gray-600 
        focus:outline-none focus:border-blue-500 resize-none font-mono text-sm"
    />
    {hint && <p className="text-xs text-gray-500 mt-1">{hint}</p>}
  </div>
);

/**
 * 折叠面板组件
 */
const CollapsibleSection: React.FC<{
  title: string;
  isOpen: boolean;
  onToggle: () => void;
  children: React.ReactNode;
}> = ({ title, isOpen, onToggle, children }) => (
  <div className="border border-gray-600 rounded overflow-hidden">
    <button
      type="button"
      className="w-full px-3 py-2 flex items-center justify-between bg-gray-700/50 
        hover:bg-gray-700 text-gray-300 text-sm transition-colors"
      onClick={onToggle}
    >
      <span>{title}</span>
      <svg
        className={`w-4 h-4 transition-transform ${isOpen ? 'rotate-180' : ''}`}
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        strokeWidth="2"
      >
        <polyline points="6 9 12 15 18 9" />
      </svg>
    </button>
    {isOpen && <div className="p-3 space-y-3 bg-gray-800/50">{children}</div>}
  </div>
);

/**
 * 解析环境变量字符串为对象
 * 格式: KEY=VALUE，每行一个
 */
function parseEnvString(envStr: string): Record<string, string> | undefined {
  if (!envStr.trim()) return undefined;

  const env: Record<string, string> = {};
  const lines = envStr.split('\n');

  for (const line of lines) {
    const trimmed = line.trim();
    if (!trimmed || trimmed.startsWith('#')) continue;

    const eqIndex = trimmed.indexOf('=');
    if (eqIndex > 0) {
      const key = trimmed.substring(0, eqIndex).trim();
      const value = trimmed.substring(eqIndex + 1).trim();
      if (key) {
        env[key] = value;
      }
    }
  }

  return Object.keys(env).length > 0 ? env : undefined;
}

/**
 * 新建连接对话框组件
 *
 * 提供本地终端和 SSH 连接两种模式：
 *
 * 本地终端选项：
 * - 自定义 shell 路径（可选，默认使用系统 shell）
 * - 工作目录（可选）
 * - 环境变量（可选）
 *
 * SSH 连接选项：
 * - 主机地址（必填）
 * - 端口（默认 22）
 * - 用户名（可选）
 * - 认证方式：密码或私钥文件
 *
 * @example
 * ```tsx
 * <NewConnectionDialog
 *   onConnect={(conn) => createSession(conn)}
 *   onClose={() => setShowDialog(false)}
 * />
 * ```
 */
export const NewConnectionDialog: React.FC<NewConnectionDialogProps> = ({
  onConnect,
  onClose,
  showAdvanced: initialShowAdvanced = false,
}) => {
  // 连接模式
  const [mode, setMode] = useState<ConnectionMode>('local');

  // 本地终端选项
  const [localShellPath, setLocalShellPath] = useState('');
  const [localCwd, setLocalCwd] = useState('');
  const [localEnv, setLocalEnv] = useState('');
  const [showLocalAdvanced, setShowLocalAdvanced] = useState(initialShowAdvanced);

  // SSH 连接选项
  const [sshHost, setSshHost] = useState('');
  const [sshPort, setSshPort] = useState('22');
  const [sshUser, setSshUser] = useState('');
  const [sshAuthMode, setSshAuthMode] = useState<SshAuthMode>('password');
  const [sshPassword, setSshPassword] = useState('');
  const [sshIdentityFile, setSshIdentityFile] = useState('');

  // 验证错误
  const [errors, setErrors] = useState<Record<string, string>>({});

  // 对话框引用（用于点击外部关闭）
  const dialogRef = useRef<HTMLDivElement>(null);

  // 验证表单
  const validate = useCallback((): boolean => {
    const newErrors: Record<string, string> = {};

    if (mode === 'ssh') {
      if (!sshHost.trim()) {
        newErrors.sshHost = '请输入主机地址';
      }
      const port = parseInt(sshPort, 10);
      if (isNaN(port) || port < 1 || port > 65535) {
        newErrors.sshPort = '端口号必须在 1-65535 之间';
      }
      if (sshAuthMode === 'key' && !sshIdentityFile.trim()) {
        newErrors.sshIdentityFile = '请输入私钥文件路径';
      }
    }

    setErrors(newErrors);
    return Object.keys(newErrors).length === 0;
  }, [mode, sshHost, sshPort, sshAuthMode, sshIdentityFile]);

  // 处理连接
  const handleConnect = useCallback(() => {
    if (!validate()) return;

    if (mode === 'local') {
      const connection: ConnectionType = {
        type: 'local',
        shell_path: localShellPath.trim() || undefined,
        cwd: localCwd.trim() || undefined,
        env: parseEnvString(localEnv),
      };
      onConnect(connection);
    } else {
      const connection: ConnectionType = {
        type: 'ssh',
        host: sshHost.trim(),
        port: parseInt(sshPort, 10) || 22,
        user: sshUser.trim() || undefined,
        password: sshAuthMode === 'password' && sshPassword ? sshPassword : undefined,
        identity_file: sshAuthMode === 'key' && sshIdentityFile.trim() ? sshIdentityFile.trim() : undefined,
      };
      onConnect(connection);
    }
  }, [
    mode,
    validate,
    localShellPath,
    localCwd,
    localEnv,
    sshHost,
    sshPort,
    sshUser,
    sshAuthMode,
    sshPassword,
    sshIdentityFile,
    onConnect,
  ]);

  // 处理键盘事件
  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (e.key === 'Escape') {
        onClose();
      } else if (e.key === 'Enter' && !e.shiftKey) {
        // 在文本区域中按 Enter 不触发提交
        if ((e.target as HTMLElement).tagName !== 'TEXTAREA') {
          e.preventDefault();
          handleConnect();
        }
      }
    },
    [onClose, handleConnect]
  );

  // 点击外部关闭
  useEffect(() => {
    const handleClickOutside = (e: MouseEvent) => {
      if (dialogRef.current && !dialogRef.current.contains(e.target as Node)) {
        onClose();
      }
    };

    document.addEventListener('mousedown', handleClickOutside);
    return () => {
      document.removeEventListener('mousedown', handleClickOutside);
    };
  }, [onClose]);

  return (
    <div
      className="fixed inset-0 bg-black/50 flex items-center justify-center z-50"
      onKeyDown={handleKeyDown}
      role="dialog"
      aria-modal="true"
      aria-labelledby="dialog-title"
    >
      <div
        ref={dialogRef}
        className="bg-gray-800 rounded-lg p-6 w-[420px] max-w-[90vw] max-h-[90vh] overflow-y-auto shadow-xl"
      >
        <h2 id="dialog-title" className="text-lg font-semibold text-gray-200 mb-4">
          新建连接
        </h2>

        {/* 连接模式切换 */}
        <div className="flex gap-2 mb-4">
          <button
            type="button"
            className={`flex-1 py-2 rounded font-medium transition-colors ${
              mode === 'local'
                ? 'bg-blue-600 text-white'
                : 'bg-gray-700 text-gray-400 hover:bg-gray-600 hover:text-gray-300'
            }`}
            onClick={() => setMode('local')}
          >
            <span className="flex items-center justify-center gap-2">
              <svg className="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                <polyline points="4 17 10 11 4 5" />
                <line x1="12" y1="19" x2="20" y2="19" />
              </svg>
              本地终端
            </span>
          </button>
          <button
            type="button"
            className={`flex-1 py-2 rounded font-medium transition-colors ${
              mode === 'ssh'
                ? 'bg-blue-600 text-white'
                : 'bg-gray-700 text-gray-400 hover:bg-gray-600 hover:text-gray-300'
            }`}
            onClick={() => setMode('ssh')}
          >
            <span className="flex items-center justify-center gap-2">
              <svg className="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                <rect x="3" y="11" width="18" height="11" rx="2" ry="2" />
                <path d="M7 11V7a5 5 0 0 1 10 0v4" />
              </svg>
              SSH 连接
            </span>
          </button>
        </div>

        {/* 本地终端选项 */}
        {mode === 'local' && (
          <div className="space-y-4">
            <p className="text-sm text-gray-400">
              使用系统默认 shell 创建本地终端会话。
            </p>

            <CollapsibleSection
              title="高级选项"
              isOpen={showLocalAdvanced}
              onToggle={() => setShowLocalAdvanced(!showLocalAdvanced)}
            >
              <FormInput
                label="Shell 路径"
                value={localShellPath}
                onChange={setLocalShellPath}
                placeholder="/bin/zsh 或 /bin/bash"
              />
              <FormInput
                label="工作目录"
                value={localCwd}
                onChange={setLocalCwd}
                placeholder="~/ 或 /path/to/directory"
              />
              <FormTextarea
                label="环境变量"
                value={localEnv}
                onChange={setLocalEnv}
                placeholder="KEY=VALUE&#10;ANOTHER_KEY=value"
                hint="每行一个，格式: KEY=VALUE"
              />
            </CollapsibleSection>
          </div>
        )}

        {/* SSH 连接选项 */}
        {mode === 'ssh' && (
          <div className="space-y-3">
            <FormInput
              label="主机"
              value={sshHost}
              onChange={setSshHost}
              placeholder="example.com 或 192.168.1.1"
              required
              error={errors.sshHost}
              autoFocus
            />
            <div className="grid grid-cols-2 gap-3">
              <FormInput
                label="端口"
                value={sshPort}
                onChange={setSshPort}
                placeholder="22"
                type="number"
                error={errors.sshPort}
              />
              <FormInput
                label="用户名"
                value={sshUser}
                onChange={setSshUser}
                placeholder="root"
              />
            </div>

            {/* 认证方式 */}
            <div>
              <label className="block text-sm text-gray-400 mb-2">认证方式</label>
              <div className="flex gap-2">
                <button
                  type="button"
                  className={`flex-1 py-1.5 text-sm rounded transition-colors ${
                    sshAuthMode === 'password'
                      ? 'bg-gray-600 text-gray-200'
                      : 'bg-gray-700 text-gray-400 hover:bg-gray-600'
                  }`}
                  onClick={() => setSshAuthMode('password')}
                >
                  密码
                </button>
                <button
                  type="button"
                  className={`flex-1 py-1.5 text-sm rounded transition-colors ${
                    sshAuthMode === 'key'
                      ? 'bg-gray-600 text-gray-200'
                      : 'bg-gray-700 text-gray-400 hover:bg-gray-600'
                  }`}
                  onClick={() => setSshAuthMode('key')}
                >
                  私钥文件
                </button>
              </div>
            </div>

            {sshAuthMode === 'password' && (
              <FormInput
                label="密码"
                value={sshPassword}
                onChange={setSshPassword}
                type="password"
                placeholder="输入密码（可选）"
              />
            )}

            {sshAuthMode === 'key' && (
              <FormInput
                label="私钥文件路径"
                value={sshIdentityFile}
                onChange={setSshIdentityFile}
                placeholder="~/.ssh/id_rsa"
                required
                error={errors.sshIdentityFile}
              />
            )}
          </div>
        )}

        {/* 操作按钮 */}
        <div className="flex justify-end gap-2 mt-6">
          <button
            type="button"
            className="px-4 py-2 text-gray-400 hover:text-gray-200 transition-colors"
            onClick={onClose}
          >
            取消
          </button>
          <button
            type="button"
            className="px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700 
              transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
            onClick={handleConnect}
          >
            连接
          </button>
        </div>
      </div>
    </div>
  );
};
