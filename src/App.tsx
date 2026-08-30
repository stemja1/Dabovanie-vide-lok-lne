import React, { useState, useEffect } from 'react';
import { Header } from './components/Header';
import { Navigation, NavTab } from './components/Navigation';
import { PipelineStudio } from './components/Pipeline/PipelineStudio';
import { UtteranceTable } from './components/MetadataEditor/UtteranceTable';
import { DubbedVideoPlayer } from './components/VideoPlayer/DubbedVideoPlayer';
import { SetupWizard } from './components/Wizard/SetupWizard';
import { LogViewer } from './components/Logs/LogViewer';
import { SettingsModal } from './components/Settings/SettingsModal';
import { LiveSystemMetrics, PipelineExecutionState } from './types/pipeline';
import { invokeCommand, addTauriListener } from './utils/tauriBridge';

export const App: React.FC = () => {
  const [activeTab, setActiveTab] = useState<NavTab>('pipeline');
  const [metrics, setMetrics] = useState<LiveSystemMetrics | null>(null);
  const [pipelineState, setPipelineState] = useState<PipelineExecutionState | null>(null);

  // Poll live metrics for header
  useEffect(() => {
    const fetchMetrics = async () => {
      try {
        const m = await invokeCommand<LiveSystemMetrics>('get_live_system_metrics');
        setMetrics(m);
      } catch (err) {
        // quiet
      }
    };

    fetchMetrics();
    const interval = setInterval(fetchMetrics, 3000);
    return () => clearInterval(interval);
  }, []);

  // Listen for pipeline state updates
  useEffect(() => {
    const fetchState = async () => {
      try {
        const st = await invokeCommand<PipelineExecutionState>('get_pipeline_state');
        setPipelineState(st);
      } catch (err) {
        // quiet
      }
    };
    fetchState();

    const unsubState = addTauriListener('pipeline_state_updated', (st: PipelineExecutionState) => {
      setPipelineState(st);
      if (st.is_paused_for_review) {
        setActiveTab('metadata');
      }
    });

    return () => unsubState();
  }, []);

  const isPausedForReview = pipelineState?.is_paused_for_review || false;
  const hasOutput = !!pipelineState?.output_video_path_win && pipelineState?.stages.every((s) => s.status === 'completed');

  const handleConfirmAndContinueFromEditor = async () => {
    try {
      await invokeCommand('continue_pipeline_after_review');
      setActiveTab('pipeline');
    } catch (err) {
      console.error(err);
    }
  };

  return (
    <div className="flex flex-col h-screen w-screen bg-slate-950 text-slate-100 font-sans overflow-hidden">
      {/* Header with Live AMD RX 7700 XT VRAM / RAM metrics */}
      <Header
        metrics={metrics}
        onOpenSettings={() => setActiveTab('settings')}
        onOpenWizard={() => setActiveTab('wizard')}
        activeTab={activeTab}
      />

      {/* Main Navigation Bar */}
      <Navigation
        activeTab={activeTab}
        onTabChange={setActiveTab}
        isPausedForReview={isPausedForReview}
        hasOutputVideo={hasOutput}
      />

      {/* Workspace Content View */}
      <main className="flex-1 overflow-y-auto px-6 py-6 custom-scrollbar">
        {activeTab === 'pipeline' && (
          <PipelineStudio
            onNavigateToReview={() => setActiveTab('metadata')}
            onNavigateToPlayer={() => setActiveTab('player')}
          />
        )}

        {activeTab === 'metadata' && (
          <UtteranceTable
            isPausedForReview={isPausedForReview}
            onConfirmAndContinue={handleConfirmAndContinueFromEditor}
          />
        )}

        {activeTab === 'player' && (
          <DubbedVideoPlayer
            inputVideoPath={pipelineState?.input_video_path_win}
            outputVideoPath={pipelineState?.output_video_path_win || undefined}
          />
        )}

        {activeTab === 'wizard' && <SetupWizard />}

        {activeTab === 'logs' && <LogViewer />}

        {activeTab === 'settings' && <SettingsModal />}
      </main>
    </div>
  );
};
export default App;
