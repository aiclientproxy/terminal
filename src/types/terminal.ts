/**
 * 终端类型定义
 *
 * 终端组件相关的类型。
 */

import type { ITerminalOptions } from '@xterm/xterm';

/**
 * 终端主题
 */
export interface TerminalTheme {
  background: string;
  foreground: string;
  cursor: string;
  cursorAccent: string;
  selectionBackground: string;
  selectionForeground: string;
  black: string;
  red: string;
  green: string;
  yellow: string;
  blue: string;
  magenta: string;
  cyan: string;
  white: string;
  brightBlack: string;
  brightRed: string;
  brightGreen: string;
  brightYellow: string;
  brightBlue: string;
  brightMagenta: string;
  brightCyan: string;
  brightWhite: string;
}

/**
 * 终端配置
 */
export interface TerminalConfig {
  fontSize: number;
  fontFamily: string;
  lineHeight: number;
  cursorBlink: boolean;
  cursorStyle: 'block' | 'underline' | 'bar';
  scrollback: number;
  theme: TerminalTheme;
}

/**
 * 默认终端主题 (VS Code Dark+)
 */
export const defaultTheme: TerminalTheme = {
  background: '#1e1e1e',
  foreground: '#d4d4d4',
  cursor: '#aeafad',
  cursorAccent: '#000000',
  selectionBackground: '#264f78',
  selectionForeground: '#ffffff',
  black: '#000000',
  red: '#cd3131',
  green: '#0dbc79',
  yellow: '#e5e510',
  blue: '#2472c8',
  magenta: '#bc3fbc',
  cyan: '#11a8cd',
  white: '#e5e5e5',
  brightBlack: '#666666',
  brightRed: '#f14c4c',
  brightGreen: '#23d18b',
  brightYellow: '#f5f543',
  brightBlue: '#3b8eea',
  brightMagenta: '#d670d6',
  brightCyan: '#29b8db',
  brightWhite: '#e5e5e5',
};

/**
 * 默认终端配置
 */
export const defaultConfig: TerminalConfig = {
  fontSize: 14,
  fontFamily: 'Menlo, Monaco, "Courier New", monospace',
  lineHeight: 1.2,
  cursorBlink: true,
  cursorStyle: 'block',
  scrollback: 10000,
  theme: defaultTheme,
};

/**
 * 将配置转换为 xterm.js 选项
 */
export function toXtermOptions(config: TerminalConfig): ITerminalOptions {
  return {
    fontSize: config.fontSize,
    fontFamily: config.fontFamily,
    lineHeight: config.lineHeight,
    cursorBlink: config.cursorBlink,
    cursorStyle: config.cursorStyle,
    scrollback: config.scrollback,
    theme: config.theme,
    allowProposedApi: true,
  };
}
