/**
 * ProxyCast 插件组件类型声明
 *
 * 这些组件由 ProxyCast 主应用提供，插件通过全局变量访问。
 */

declare module '@proxycast/plugin-components' {
  import { FC, ReactNode } from 'react';

  export interface ButtonProps {
    children: ReactNode;
    onClick?: () => void;
    variant?: 'primary' | 'secondary' | 'ghost' | 'danger';
    size?: 'sm' | 'md' | 'lg';
    disabled?: boolean;
    className?: string;
  }

  export interface InputProps {
    value: string;
    onChange: (value: string) => void;
    placeholder?: string;
    type?: 'text' | 'password';
    disabled?: boolean;
    className?: string;
  }

  export interface CardProps {
    children: ReactNode;
    className?: string;
  }

  export const Button: FC<ButtonProps>;
  export const Input: FC<InputProps>;
  export const Card: FC<CardProps>;
}
