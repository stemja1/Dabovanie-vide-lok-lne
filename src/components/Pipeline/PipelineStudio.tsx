import React, { useState, useEffect } from 'react';
import {
  Play,
  Square,
  RotateCcw,
  Sparkles,
  Layers,
  ArrowRight,
  Clock,
  ShieldCheck,
  AlertCircle,
  FileCheck,
} from 'lucide-react';
import { PipelineExecutionState, FullPipelineResourceBudget } from '../../types/pipeline';
import { AppConfig } from '../../types/config';
import { invokeCommand, addTauriListener } from '../../utils/tauriBridge';
import { Button } from '../ui/Button';
import { Badge } from '../ui/Badge';
import { VideoDropzone } from './VideoDropzone';
import { StepProgressCard } from './StepProgressCard';
import { VramMeter } from './VramMeter';

interface PipelineStudioProps {
  onNavigateToReview: () => void;
  onNavigateToPlayer: () => void;
}

export const PipelineStudio: React.FC<PipelineStudioProps> = ({
  onNavigateToReview,
  onNavigateToPlayer,
}) => {
  const [pipelineState, setPipelineState] = useState<PipelineExecutionState | null>(null);
  const [budget, setBudget] = useState<FullPipelineResourceBudget | null>(null);
  const [config, setConfig] = useState<AppConfig | null>(null);
  const [isLoading, setIsLoading] = useState<boolean>(false);
  // `start_pipeline_execution` returns Ok(()) immediately and the pre-flight
  // failures (no video, no WSL, script sync failure) are only reported through
  // the backend's state, so the user needs somewhere to actually see them.
  const [errorMessage, setErrorMessage] = useState<string | null>(null);

  const fetchData = async () => {
    try {
      const [st, bud, cfg] = await Promise.all([
        invokeCommand<PipelineExecutionState>('get_pipeline_state'),
        invokeCommand<FullPipelineResourceBudget>('get_resource_budget'),
        invokeCommand<AppConfig>('get_config'),
      ]);
      setPipelineState(st);
      setBudget(bud);
      setConfig(cfg);
    } catch (err) {
      console.error('Failed to load pipeline state', err);
    }
  };

  useEffect(() => {
    fetchData();

    const unsubState = addTauriListener('pipeline_state_updated', (st: PipelineExecutionState) => {
      setPipelineState(st);
    });

    // `pipeline_state_updated` is emitted by the backend, but the backend has no
    // push while a stage runs in older builds and the initial event can be missed
    // entirely (the app was showing mock data when `listen()` was denied). Poll
    // while a run is in flight so progress, the Stop button and the disabled
    // controls are driven by real state instead of never updating.
    const poll = window.setInterval(async () => {
      try {
        const st = await invokeCommand<PipelineExecutionState>('get_pipeline_state');
        setPipelineState((prev) => (prev?.is_running || st?.is_running ? st : prev ?? st));
      } catch {
        // Backend not reachable - keep whatever we already have.
      }
    }, 1000);

    return () => {
      unsubState();
      window.clearInterval(poll);
    };
  }, []);

  const handleVideoSelected = async (videoPath: string) => {
    try {
      const updated = await invokeCommand<PipelineExecutionState>('set_pipeline_video', {
        video_path: videoPath,
      });
      setPipelineState(updated);
    } catch (err) {
      console.error('Failed to set video', err);
    }
  };

  const handleStartPipeline = async () => {
    setIsLoading(true);
    try {
      await invokeCommand('start_pipeline_execution');
      await fetchData();
    } catch (err) {
      console.error('Failed to start pipeline', err);
      setErrorMessage(err instanceof Error ? err.message : String(err));
    } finally {
      setIsLoading(false);
    }
  };

  const handleContinueAfterReview = async () => {
    setIsLoading(true);
    try {
      await invokeCommand('continue_pipeline_after_review');
      await fetchData();
    } catch (err) {
      console.error('Failed to resume pipeline', err);
    } finally {
      setIsLoading(false);
    }
  };

  const handleRunSingleStage = async (stageIndex: number) => {
    try {
      await invokeCommand('run_single_stage', { stage_index: stageIndex });
      await fetchData();
    } catch (err) {
      console.error(`Failed to run single stage ${stageIndex}`, err);
    }
  };

  const handleCancelPipeline = async () => {
    try {
      await invokeCommand('cancel_pipeline_execution');
      await fetchData();
    } catch (err) {
      console.error('Failed to cancel pipeline', err);
    }
  };

  const isRunning = pipelineState?.is_running || false;
  const isPaused = pipelineState?.is_paused_for_review || false;
  const stages = pipelineState?.stages || [];
  const hasInput = !!pipelineState?.input_video_path_win;
  const isCompleted = stages.length > 0 && stages.every((s) => s.status === 'completed');
  // Show whatever the backend put into `error_summary`; it used to be written but
  // never rendered anywhere, so failures appeared to do nothing at all.
  const backendError = pipelineState?.error_summary ?? null;
  const visibleError = errorMessage ?? backendError;

  return (
    <div className="space-y-6 max-w-5xl mx-auto pb-12">
      {visibleError && (
        <div
          role="alert"
          className="flex items-start gap-2.5 rounded-lg border border-rose-800/60 bg-rose-950/40 px-4 py-3 text-sm text-rose-200"
        >
          <AlertCircle className="w-4 h-4 mt-0.5 shrink-0" />
          <div className="flex-1">
            <p className="font-medium">Pipeline sa nepodarilo spustiť</p>
            <p className="mt-0.5 break-words text-rose-300/90">{visibleError}</p>
          </div>
          <button
            type="button"
            onClick={() => setErrorMessage(null)}
            className="text-rose-400 hover:text-rose-200 text-xs"
          >
            Zavrieť
          </button>
        </div>
      )}

      {/* Top Controls & Status Header */}
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4 pb-4 border-b border-slate-800">
        <div>
          <div className="flex items-center gap-2">
            <h2 className="text-xl font-bold text-slate-100">AI Dabingový Pipeline</h2>
            <Badge variant="primary">Slovenčina → Čínština</Badge>
            {isPaused && <Badge variant="warning">Čaká na kontrolu metadát</Badge>}
            {isCompleted && <Badge variant="success">Dabing dokončený</Badge>}
          </div>
          <p className="text-xs text-slate-400 mt-1">
            Sekvenčný orchestrátor s integrovaným ASR, MT prekladom, TTS a LatentSync 1.5 lip-syncom.
          </p>
        </div>

        {/* Action Controls */}
        <div className="flex items-center gap-2.5">
          {isRunning ? (
            <Button
              variant="danger"
              size="sm"
              leftIcon={<Square className="w-3.5 h-3.5" />}
              onClick={handleCancelPipeline}
            >
              Zastaviť proces
            </Button>
          ) : isPaused ? (
            <div className="flex items-center gap-2">
              <Button
                variant="secondary"
                size="sm"
                onClick={onNavigateToReview}
              >
                Upraviť metadáta
              </Button>
              <Button
                variant="primary"
                size="sm"
                leftIcon={<Play className="w-3.5 h-3.5" />}
                onClick={handleContinueAfterReview}
              >
                Potvrdiť a pokračovať v TTS
              </Button>
            </div>
          ) : isCompleted ? (
            <Button
              variant="success"
              size="sm"
              rightIcon={<ArrowRight className="w-3.5 h-3.5" />}
              onClick={onNavigateToPlayer}
            >
              Pozrieť výsledné video
            </Button>
          ) : (
            <Button
              variant="primary"
              size="sm"
              leftIcon={<Play className="w-3.5 h-3.5" />}
              onClick={handleStartPipeline}
              disabled={!hasInput || isLoading}
            >
              Spustiť AI Dabing
            </Button>
          )}
        </div>
      </div>

      {/* Video Selection Dropzone */}
      <VideoDropzone
        currentVideoPath={pipelineState?.input_video_path_win || ''}
        onVideoSelected={handleVideoSelected}
        disabled={isRunning}
      />

      {/* Review Paused Notice Banner */}
      {isPaused && (
        <div className="p-4 rounded-xl bg-amber-500/10 border border-amber-500/30 text-amber-300 flex flex-col sm:flex-row items-start sm:items-center justify-between gap-3 animate-fadeIn">
          <div className="flex items-center gap-3">
            <Clock className="w-5 h-5 text-amber-400 flex-shrink-0" />
            <div>
              <strong className="font-semibold block text-amber-200">
                Kontrolný medzikrok: Preklad a časovanie sú pripravené
              </strong>
              <span className="text-xs text-amber-300/80">
                Pred spustením syntézy reči si môžete skontrolovať a upraviť čínske preklady priamo v editore.
              </span>
            </div>
          </div>
          <Button
            variant="primary"
            size="sm"
            onClick={onNavigateToReview}
            rightIcon={<ArrowRight className="w-3.5 h-3.5" />}
          >
            Otvoriť Editor
          </Button>
        </div>
      )}

      {/* VRAM & Memory Safety Meter */}
      <VramMeter budget={budget} />

      {/* Sequential Pipeline Stages List */}
      <div className="space-y-3">
        <div className="flex items-center justify-between px-1">
          <h3 className="text-xs font-semibold text-slate-400 uppercase tracking-wider flex items-center gap-2">
            <Layers className="w-3.5 h-3.5" />
            Sekvenčné Fázy Spracovania (Jeden model načítaný naraz)
          </h3>
          <span className="text-xs text-slate-500">
            Aktívny lip-sync: <strong>{pipelineState?.active_lipsync_engine || 'LatentSync 1.5'}</strong>
          </span>
        </div>

        <div className="space-y-2.5">
          {stages.map((stage, idx) => (
            <StepProgressCard
              key={stage.id}
              stage={stage}
              index={idx}
              isActive={pipelineState?.current_stage_index === idx && isRunning}
              onRunSingle={() => handleRunSingleStage(idx)}
              onOpenReview={onNavigateToReview}
              disabled={isRunning}
            />
          ))}
        </div>
      </div>
    </div>
  );
};
