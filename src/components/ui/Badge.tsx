import React from 'react';

interface BadgeProps {
  children: React.ReactNode;
  variant?: 'primary' | 'secondary' | 'success' | 'warning' | 'danger' | 'purple' | 'info';
  size?: 'sm' | 'md';
  className?: string;
  icon?: React.ReactNode;
}

export const Badge: React.FC<BadgeProps> = ({
  children,
  variant = 'primary',
  size = 'md',
  className = '',
  icon,
}) => {
  const sizeStyles = {
    sm: 'px-2 py-0.5 text-xs',
    md: 'px-2.5 py-1 text-xs',
  };

  const variantStyles = {
    primary: 'bg-indigo-500/10 text-indigo-400 border border-indigo-500/30',
    secondary: 'bg-slate-800 text-slate-300 border border-slate-700',
    success: 'bg-emerald-500/10 text-emerald-400 border border-emerald-500/30',
    warning: 'bg-amber-500/10 text-amber-400 border border-amber-500/30',
    danger: 'bg-rose-500/10 text-rose-400 border border-rose-500/30',
    purple: 'bg-fuchsia-500/10 text-fuchsia-400 border border-fuchsia-500/30',
    info: 'bg-sky-500/10 text-sky-400 border border-sky-500/30',
  };

  return (
    <span className={`inline-flex items-center gap-1.5 font-medium rounded-full ${sizeStyles[size]} ${variantStyles[variant]} ${className}`}>
      {icon}
      <span>{children}</span>
    </span>
  );
};
