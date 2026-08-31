import React from 'react';
import {
  CheckCircle2,
  Clock,
  AlertTriangle,
  Play,
  RotateCcw,
  Sparkles,
  Cpu,
  Layers,
  Edit,
} from 'lucide-react';
import { PipelineStageInfo, StageStatus } from '../../types/pipeline';
import { formatDurationMs } from '../../utils/formatters';
import { Button } from '../ui/Button';
import { Badge } from '../ui/Badge';
import { ProgressBar } from '../ui/ProgressBar';

interface StepProgressCardProps {
  stage: PipelineStageInfo;
  index: number;
  isActive: boolean;
  onRunSingle: () => void;
  onOpenReview?: () => void;
  disabled?: boolean;
}

export const StepProgressCard: React.FC<StepProgressCardProps> = ({
  stage,
  index,
  isActive,
  onRunSingle,
  onOpenReview,
  disabled = false,
}) => {
  const getStatusIcon = (status: StageStatus) => {
    switch (status) {
      case 'completed':
        return <CheckCircle2 className="w-5 h-5 text-emerald-400" />;
      case 'running':
        return (
          <div className="w-5 h-5 rounded-full border-2 border-indigo-500 border-t-transparent animate-spin" />
        );
      case 'review_paused':
        return <Clock className="w-5 h-5 text-amber-400 animate-pulse" />;
      case 'failed':
        return <AlertTriangle className="w-5 h-5 text-rose-400" />;
      case 'skipped':
        return <span className="w-2 h-2 rounded-full bg-slate-600" />;
      default:
        return (
          <div className="w-5 h-5 rounded-full bg-slate-800 border border-slate-700 flex items-center justify-center text-[10px] text-slate-400 font-mono">
            {index + 1}
          </div>
        );
    }
  };

  const isReviewStep = stage.id === 'review';

  return (
    <div
      className={`p-4 rounded-xl border transition-all duration-200 ${
        isActive
          ? 'bg-slate-900/90 border-indigo-500/50 shadow-lg shadow-indigo-600/10'
          : stage.status === 'completed'
          ? 'bg-slate-900/40 border-slate-800/80'
          : stage.status === 'review_paused'
          ? 'bg-amber-950/20 border-amber-500/40'
          : stage.status === 'failed'
          ? 'bg-rose-950/20 border-rose-500/40'
          : 'bg-slate-900/20 border-slate-800/60'
      }`}
    >
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3">
        {/* Step Title & Icon */}
        <div className="flex items-start gap-3">
          <div className="mt-0.5">{getStatusIcon(stage.status)}</div>
          <div>
            <div className="flex flex-wrap items-center gap-2">
              <h4 className="font-semibold text-sm text-slate-100">{stage.name}</h4>
              <Badge variant="secondary" size="sm">
                {stage.engine_badge}
              </Badge>
              {stage.is_gpu_accelerated && (
                <Badge variant="primary" size="sm" icon={<Cpu className="w-3 h-3 text-indigo-400" />}>
                  ROCm GPU
                </Badge>
              )}
            </div>
            <p className="text-xs text-slate-400 mt-0.5">{stage.description}</p>
          </div>
        </div>

        {/* Status / VRAM Estimate & Actions */}
        <div className="flex items-center gap-3 self-end sm:self-center">
          <div className="text-right hidden md:block">
            <div className="text-[11px] text-slate-400">
              VRAM: <span className="font-mono text-slate-200 font-medium">~{stage.estimated_vram_gb} GB</span> | RAM: <span className="font-mono text-slate-200 font-medium">~{stage.estimated_ram_gb} GB</span>
            </div>
            <div className="text-[10px] text-slate-500 font-mono mt-0.5">
              Trvanie: {formatDurationMs(stage.started_at_ms, stage.completed_at_ms)}
            </div>
          </div>

          {isReviewStep && stage.status === 'review_paused' && onOpenReview && (
            <Button
              variant="primary"
              size="sm"
              leftIcon={<Edit className="w-3.5 h-3.5" />}
              onClick={onOpenReview}
            >
              Otvoriť Editor Metadát
            </Button>
          )}

          {stage.status === 'failed' && (
            <Button
              variant="danger"
              size="sm"
              leftIcon={<RotateCcw className="w-3.5 h-3.5" />}
              onClick={onRunSingle}
              disabled={disabled}
            >
              Opakovať krok
            </Button>
          )}

          {stage.status === 'idle' && !isReviewStep && (
            <Button
              variant="ghost"
              size="sm"
              onClick={onRunSingle}
              disabled={disabled}
              className="text-xs text-slate-400 hover:text-slate-100"
            >
              Spustiť samostatne
            </Button>
          )}
        </div>
      </div>

      {/* Progress Bar for active step */}
      {stage.status === 'running' && (
        <div className="mt-3 pt-2 border-t border-slate-800">
          <ProgressBar
            progress={stage.progress_percent}
            size="sm"
            variant="gradient"
            showLabel
          />
        </div>
      )}

      {/* Error / Remedy Message */}
      {stage.error_message && (
        <div className="mt-3 p-3 rounded-lg bg-rose-500/10 border border-rose-500/30 text-rose-300 text-xs">
          <p className="font-mono font-semibold">Chyba pri vykonávaní fázy:</p>
          <p className="font-mono text-[11px] text-rose-400/90 mt-1">{stage.error_message}</p>
          {stage.user_suggestion && (
            <p className="mt-2 text-slate-200 bg-rose-950/40 p-2 rounded border border-rose-500/20">
              💡 <strong>Riešenie:</strong> {stage.user_suggestion}
            </p>
          )}
        </div>
      )}
    </div>
  );
};
