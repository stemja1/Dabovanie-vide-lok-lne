import React from 'react';

interface ButtonProps extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: 'primary' | 'secondary' | 'outline' | 'danger' | 'success' | 'ghost';
  size?: 'sm' | 'md' | 'lg';
  isLoading?: boolean;
  leftIcon?: React.ReactNode;
  rightIcon?: React.ReactNode;
}

export const Button: React.FC<ButtonProps> = ({
  children,
  variant = 'primary',
  size = 'md',
  isLoading = false,
  leftIcon,
  rightIcon,
  className = '',
  disabled,
  ...props
}) => {
  const baseStyles = 'inline-flex items-center justify-center font-medium rounded-lg transition-all duration-200 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-offset-slate-900 disabled:opacity-50 disabled:cursor-not-allowed select-none';

  const sizeStyles = {
    sm: 'px-2.5 py-1.5 text-xs gap-1.5',
    md: 'px-4 py-2 text-sm gap-2',
    lg: 'px-5 py-2.5 text-base gap-2.5',
  };

  const variantStyles = {
    primary: 'bg-indigo-600 hover:bg-indigo-500 active:bg-indigo-700 text-white shadow-lg shadow-indigo-600/20 focus:ring-indigo-500 border border-indigo-500/30',
    secondary: 'bg-slate-800 hover:bg-slate-700 active:bg-slate-850 text-slate-200 border border-slate-700 focus:ring-slate-500 shadow-sm',
    outline: 'bg-transparent hover:bg-slate-800/80 active:bg-slate-800 text-slate-300 border border-slate-700 focus:ring-slate-500',
    danger: 'bg-rose-600 hover:bg-rose-500 active:bg-rose-700 text-white shadow-lg shadow-rose-600/20 focus:ring-rose-500 border border-rose-500/30',
    success: 'bg-emerald-600 hover:bg-emerald-500 active:bg-emerald-700 text-white shadow-lg shadow-emerald-600/20 focus:ring-emerald-500 border border-emerald-500/30',
    ghost: 'bg-transparent hover:bg-slate-800 text-slate-400 hover:text-slate-100 focus:ring-slate-500',
  };

  return (
    <button
      className={`${baseStyles} ${sizeStyles[size]} ${variantStyles[variant]} ${className}`}
      disabled={disabled || isLoading}
      {...props}
    >
      {isLoading ? (
        <svg className="animate-spin -ml-1 mr-2 h-4 w-4 text-current" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
          <circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4"></circle>
          <path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
        </svg>
      ) : leftIcon}
      <span>{children}</span>
      {!isLoading && rightIcon}
    </button>
  );
};
