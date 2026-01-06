/**
 * Vite 构建配置
 * 
 * 配置前端构建为 IIFE 格式，用于 ProxyCast 插件系统
 * - 输出格式: IIFE (立即执行函数表达式)
 * - CSS 提取: 单独的 styles.css 文件
 * - 外部依赖: React, ReactDOM 由主应用提供
 */

import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import path from 'path';
import fs from 'fs';

// 检查是否在 CI 环境或主应用组件库是否存在
const isCI = process.env.CI === 'true' || process.env.GITHUB_ACTIONS === 'true';
const proxycastComponentsPath = path.resolve(__dirname, '../proxycast/src/lib/plugin-components');
const hasLocalComponents = fs.existsSync(proxycastComponentsPath);

export default defineConfig({
  plugins: [react()],
  
  define: {
    // 定义 process.env.NODE_ENV，避免运行时报错
    'process.env.NODE_ENV': JSON.stringify('production'),
  },
  
  resolve: {
    alias: {
      '@': path.resolve(__dirname, './src'),
      // 仅在本地开发且主应用存在时使用 alias
      // CI 环境使用类型声明文件，运行时从全局变量获取
      ...(hasLocalComponents && !isCI ? {
        '@proxycast/plugin-components': proxycastComponentsPath,
      } : {}),
    },
  },
  
  build: {
    // 输出目录
    outDir: 'plugin/dist',
    
    // 清空输出目录
    emptyOutDir: true,
    
    // 库模式配置
    lib: {
      entry: path.resolve(__dirname, 'src/index.tsx'),
      name: 'TerminalPluginUI',
      formats: ['iife'],
      fileName: () => 'index.js',
    },
    
    rollupOptions: {
      // 这些依赖由主应用提供，不打包进插件
      external: [
        'react',
        'react-dom',
        '@proxycast/plugin-components',
      ],
      
      output: {
        // 从全局变量获取依赖
        globals: {
          'react': 'React',
          'react-dom': 'ReactDOM',
          '@proxycast/plugin-components': 'ProxyCastPluginComponents',
        },
        
        // IIFE 格式需要导出到全局变量
        name: 'TerminalPlugin',
        
        // 确保默认导出可用
        exports: 'named',
        
        // CSS 提取到单独文件
        assetFileNames: (assetInfo) => {
          if (assetInfo.name && assetInfo.name.endsWith('.css')) {
            return 'styles.css';
          }
          return assetInfo.name || 'assets/[name]-[hash][extname]';
        },
      },
    },
    
    // 不分割 CSS，合并到单个文件
    cssCodeSplit: false,
    
    // 生成 sourcemap 用于调试
    sourcemap: true,
    
    // 压缩配置
    minify: 'esbuild',
    
    // 目标浏览器
    target: 'es2020',
    
    // 报告压缩后的大小
    reportCompressedSize: true,
    
    // chunk 大小警告阈值 (KB)
    chunkSizeWarningLimit: 500,
  },
  
  // CSS 配置
  css: {
    // 开启 CSS modules
    modules: {
      localsConvention: 'camelCase',
    },
    
    // PostCSS 配置
    postcss: './postcss.config.js',
    
    // 开发时注入样式
    devSourcemap: true,
  },
  
  // 开发服务器配置
  server: {
    port: 3001,
    strictPort: true,
    host: true,
  },
  
  // 预览服务器配置
  preview: {
    port: 3001,
  },
});
