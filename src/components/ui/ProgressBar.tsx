import React from 'react';

interface ProgressBarProps {
  progress: number; // 0 to 100
  size?: 'sm' | 'md' | 'lg';
  variant?: 'primary' | 'success' | 'warning' | 'danger' | 'gradient';
  showLabel?: boolean;
  animated?: boolean;
  className?: string;
}

export const ProgressBar: React.FC<ProgressBarProps> = ({
  progress,
  size = 'md',
  variant = 'primary',
  showLabel = false,
  animated = true,
  className = '',
}) => {
  const clampedProgress = Math.min(100, Math.max(0, progress));

  const sizeStyles = {
    sm: 'h-1.5',
    md: 'h-2.5',
    lg: 'h-4',
  };

  const variantStyles = {
    primary: 'bg-indigo-500',
    success: 'bg-emerald-500',
    warning: 'bg-amber-500',
    danger: 'bg-rose-500',
    gradient: 'bg-gradient-to-r from-indigo-500 via-purple-500 to-pink-500',
  };

  return (
    <div className={`w-full ${className}`}>
      {showLabel && (
        <div className="flex justify-between items-center mb-1 text-xs text-slate-400">
          <span>Postup</span>
          <span className="font-mono font-medium text-slate-200">{clampedProgress.toFixed(1)}%</span>
        </div>
      )}
      <div className={`w-full bg-slate-800 rounded-full overflow-hidden border border-slate-700/50 ${sizeStyles[size]}`}>
        <div
          className={`${sizeStyles[size]} ${variantStyles[variant]} rounded-full transition-all duration-300 ease-out ${
            animated && clampedProgress > 0 && clampedProgress < 100 ? 'relative overflow-hidden' : ''
          }`}
          style={{ width: `${clampedProgress}%` }}
        >
          {animated && clampedProgress > 0 && clampedProgress < 100 && (
            <div className="absolute inset-0 bg-white/20 animate-[pulse_1.5s_infinite]"></div>
          )}
        </div>
      </div>
    </div>
  );
};
