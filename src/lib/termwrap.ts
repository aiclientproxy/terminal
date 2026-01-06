/**
 * TermWrap - xterm.js 包装器
 *
 * 封装 xterm.js 的初始化和交互逻辑。
 * 加载 FitAddon, SearchAddon, WebLinksAddon，可选加载 WebglAddon。
 *
 * @module lib/termwrap
 * @requires @xterm/xterm
 * @requires @xterm/addon-fit
 * @requires @xterm/addon-search
 * @requires @xterm/addon-web-links
 * @requires @xterm/addon-webgl
 */

import { Terminal } from '@xterm/xterm';
import { FitAddon } from '@xterm/addon-fit';
import { SearchAddon } from '@xterm/addon-search';
import { WebLinksAddon } from '@xterm/addon-web-links';
import { WebglAddon } from '@xterm/addon-webgl';
import type { TerminalConfig } from '@/types/terminal';
import { toXtermOptions, defaultConfig } from '@/types/terminal';

/**
 * 搜索选项
 */
export interface SearchOptions {
  caseSensitive?: boolean;
  regex?: boolean;
  wholeWord?: boolean;
}

/**
 * xterm.js 包装器
 *
 * 封装 xterm.js 终端实例，提供：
 * - 终端初始化和配置
 * - 插件管理 (FitAddon, SearchAddon, WebLinksAddon, WebglAddon)
 * - 输入/输出处理
 * - 大小调整
 * - 搜索功能
 *
 * @example
 * ```typescript
 * const termWrap = new TermWrap({ fontSize: 14 });
 * termWrap.onData = (data) => sendToBackend(data);
 * termWrap.onResize = (cols, rows) => resizeBackend(cols, rows);
 * termWrap.open(containerElement);
 * ```
 */
export class TermWrap {
  private terminal: Terminal;
  private fitAddon: FitAddon;
  private searchAddon: SearchAddon;
  private webLinksAddon: WebLinksAddon;
  private webglAddon?: WebglAddon;
  private resizeObserver?: ResizeObserver;
  private container?: HTMLElement;
  private isDisposed = false;
  private currentSearchQuery = '';
  private currentSearchOptions: SearchOptions = {};

  /**
   * 数据回调 - 当用户在终端中输入时触发
   */
  public onData?: (data: string) => void;

  /**
   * 大小变化回调 - 当终端大小改变时触发
   */
  public onResize?: (cols: number, rows: number) => void;

  /**
   * 标题变化回调 - 当终端标题改变时触发（通过 OSC 序列）
   */
  public onTitleChange?: (title: string) => void;

  /**
   * 创建 TermWrap 实例
   *
   * @param config - 终端配置，包含字体、主题等设置
   */
  constructor(config: TerminalConfig = defaultConfig) {
    this.terminal = new Terminal(toXtermOptions(config));
    this.fitAddon = new FitAddon();
    this.searchAddon = new SearchAddon();
    this.webLinksAddon = new WebLinksAddon();

    // 加载核心插件
    this.terminal.loadAddon(this.fitAddon);
    this.terminal.loadAddon(this.searchAddon);
    this.terminal.loadAddon(this.webLinksAddon);

    // 监听数据输入
    this.terminal.onData((data) => {
      if (!this.isDisposed) {
        this.onData?.(data);
      }
    });

    // 监听标题变化
    this.terminal.onTitleChange((title) => {
      if (!this.isDisposed) {
        this.onTitleChange?.(title);
      }
    });
  }

  /**
   * 挂载到 DOM 元素
   *
   * @param container - 要挂载终端的 DOM 元素
   */
  open(container: HTMLElement): void {
    if (this.isDisposed) {
      console.warn('TermWrap: Cannot open disposed terminal');
      return;
    }

    this.container = container;
    this.terminal.open(container);

    // 尝试加载 WebGL 插件以提升渲染性能
    this.tryLoadWebglAddon();

    // 初始适配
    this.fit();

    // 监听容器大小变化
    this.resizeObserver = new ResizeObserver(() => {
      // 使用 requestAnimationFrame 避免频繁调用
      requestAnimationFrame(() => {
        if (!this.isDisposed) {
          this.fit();
        }
      });
    });
    this.resizeObserver.observe(container);
  }

  /**
   * 尝试加载 WebGL 插件
   * 如果 WebGL 不可用，会静默失败并使用 Canvas 渲染
   */
  private tryLoadWebglAddon(): void {
    try {
      this.webglAddon = new WebglAddon();

      // 监听 WebGL 上下文丢失事件
      this.webglAddon.onContextLoss(() => {
        console.warn('TermWrap: WebGL context lost, falling back to canvas renderer');
        this.webglAddon?.dispose();
        this.webglAddon = undefined;
      });

      this.terminal.loadAddon(this.webglAddon);
    } catch (e) {
      console.warn('TermWrap: WebGL addon not available, using canvas renderer:', e);
      this.webglAddon = undefined;
    }
  }

  /**
   * 写入数据到终端
   *
   * @param data - 要写入的数据（可以包含 ANSI 转义序列）
   */
  write(data: string): void {
    if (!this.isDisposed) {
      this.terminal.write(data);
    }
  }

  /**
   * 写入数据并换行
   *
   * @param data - 要写入的数据
   */
  writeln(data: string): void {
    if (!this.isDisposed) {
      this.terminal.writeln(data);
    }
  }

  /**
   * 适配容器大小
   * 重新计算终端尺寸以适应容器
   */
  fit(): void {
    if (this.isDisposed || !this.container) return;

    try {
      this.fitAddon.fit();
      this.onResize?.(this.terminal.cols, this.terminal.rows);
    } catch (e) {
      console.warn('TermWrap: Failed to fit terminal:', e);
    }
  }

  /**
   * 获取当前尺寸
   *
   * @returns 终端的列数和行数
   */
  getSize(): { cols: number; rows: number } {
    return {
      cols: this.terminal.cols,
      rows: this.terminal.rows,
    };
  }

  /**
   * 搜索文本
   *
   * @param query - 搜索查询字符串
   * @param options - 搜索选项
   * @returns 是否找到匹配项
   */
  search(query: string, options?: SearchOptions): boolean {
    if (this.isDisposed) return false;

    this.currentSearchQuery = query;
    this.currentSearchOptions = options || {};
    return this.searchAddon.findNext(query, {
      caseSensitive: options?.caseSensitive,
      regex: options?.regex,
      wholeWord: options?.wholeWord,
    });
  }

  /**
   * 搜索下一个匹配项
   *
   * @returns 是否找到下一个匹配项
   */
  searchNext(): boolean {
    if (this.isDisposed || !this.currentSearchQuery) return false;
    return this.searchAddon.findNext(this.currentSearchQuery, {
      caseSensitive: this.currentSearchOptions.caseSensitive,
      regex: this.currentSearchOptions.regex,
      wholeWord: this.currentSearchOptions.wholeWord,
    });
  }

  /**
   * 搜索上一个匹配项
   *
   * @returns 是否找到上一个匹配项
   */
  searchPrevious(): boolean {
    if (this.isDisposed || !this.currentSearchQuery) return false;
    return this.searchAddon.findPrevious(this.currentSearchQuery, {
      caseSensitive: this.currentSearchOptions.caseSensitive,
      regex: this.currentSearchOptions.regex,
      wholeWord: this.currentSearchOptions.wholeWord,
    });
  }

  /**
   * 清除搜索高亮
   */
  clearSearch(): void {
    if (!this.isDisposed) {
      this.currentSearchQuery = '';
      this.searchAddon.clearDecorations();
    }
  }

  /**
   * 聚焦终端
   */
  focus(): void {
    if (!this.isDisposed) {
      this.terminal.focus();
    }
  }

  /**
   * 取消聚焦
   */
  blur(): void {
    if (!this.isDisposed) {
      this.terminal.blur();
    }
  }

  /**
   * 检查终端是否聚焦
   */
  isFocused(): boolean {
    return !this.isDisposed && document.activeElement === this.terminal.textarea;
  }

  /**
   * 清屏
   */
  clear(): void {
    if (!this.isDisposed) {
      this.terminal.clear();
    }
  }

  /**
   * 重置终端
   */
  reset(): void {
    if (!this.isDisposed) {
      this.terminal.reset();
    }
  }

  /**
   * 滚动到底部
   */
  scrollToBottom(): void {
    if (!this.isDisposed) {
      this.terminal.scrollToBottom();
    }
  }

  /**
   * 滚动到顶部
   */
  scrollToTop(): void {
    if (!this.isDisposed) {
      this.terminal.scrollToTop();
    }
  }

  /**
   * 选择所有文本
   */
  selectAll(): void {
    if (!this.isDisposed) {
      this.terminal.selectAll();
    }
  }

  /**
   * 获取选中的文本
   */
  getSelection(): string {
    if (this.isDisposed) return '';
    return this.terminal.getSelection();
  }

  /**
   * 清除选择
   */
  clearSelection(): void {
    if (!this.isDisposed) {
      this.terminal.clearSelection();
    }
  }

  /**
   * 检查是否已销毁
   */
  get disposed(): boolean {
    return this.isDisposed;
  }

  /**
   * 获取底层 xterm.js 实例（用于高级操作）
   */
  get xterm(): Terminal {
    return this.terminal;
  }

  /**
   * 销毁终端实例，释放所有资源
   */
  dispose(): void {
    if (this.isDisposed) return;

    this.isDisposed = true;
    this.resizeObserver?.disconnect();
    this.webglAddon?.dispose();
    this.searchAddon.dispose();
    this.fitAddon.dispose();
    this.webLinksAddon.dispose();
    this.terminal.dispose();

    this.container = undefined;
    this.onData = undefined;
    this.onResize = undefined;
    this.onTitleChange = undefined;
  }
}
