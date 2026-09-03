import React, { useState, useEffect } from 'react';
import {
  Wrench,
  CheckCircle2,
  XCircle,
  Play,
  Square,
  RefreshCw,
  Layers,
  DownloadCloud,
  HelpCircle,
  ShieldAlert,
  Copy,
  Check,
  RotateCcw,
} from 'lucide-react';
import { SystemDiagnosticsReport, ModelManifestItem } from '../../types/wizard';
import { ProcessLogLine } from '../../types/pipeline';
import { invokeCommand, addTauriListener } from '../../utils/tauriBridge';
import { Button } from '../ui/Button';
import { Badge } from '../ui/Badge';
import { Card } from '../ui/Card';
import { ProgressBar } from '../ui/ProgressBar';
import { DiagnosticsReport } from './DiagnosticsReport';
import { CheckpointDownloader } from './CheckpointDownloader';

interface SetupWizardProps {
  onClose?: () => void;
}

export const SetupWizard: React.FC<SetupWizardProps> = ({ onClose }) => {
  const [activeTab, setActiveTab] = useState<'diagnostics' | 'wizard_run' | 'models' | 'guide'>('diagnostics');
  const [report, setReport] = useState<SystemDiagnosticsReport | null>(null);
  const [models, setModels] = useState<ModelManifestItem[]>([]);
  const [isLoadingReport, setIsLoadingReport] = useState<boolean>(false);

  // Automated installer run state
  const [isRunningAll, setIsRunningAll] = useState<boolean>(false);
  const [currentStepIndex, setCurrentStepIndex] = useState<number>(0);
  const [failedStepIndex, setFailedStepIndex] = useState<number | null>(null);
  const [installerLogs, setInstallerLogs] = useState<ProcessLogLine[]>([]);
  const [downloadingModelId, setDownloadingModelId] = useState<string | null>(null);
  const [copiedCmd, setCopiedCmd] = useState<string | null>(null);

  const wizardSteps = [
    { id: 'wsl_install', title: '1. Inštalácia WSL2 & Ubuntu-24.04', desc: 'Overenie subsystému a inicializácia distribúcie' },
    { id: 'system_packages', title: '2. Systémové balíky Ubuntu', desc: 'ffmpeg, git, python3-venv, build-essential (ako root)' },
    { id: 'python_rocm', title: '3. Python Venv & PyTorch ROCm 6.2/6.4', desc: 'Virtuálne prostredie s AMD ROCm GPU podporou' },
    { id: 'lipsync_repos', title: '4. AI Repozitáre (LatentSync 1.5 & MuseTalk)', desc: 'Klonovanie a inštalácia závislostí modelov' },
    { id: 'model_whisper-large-v3-sk', title: '5. Whisper Large-v3 SK Model', desc: 'Slovenský ASR checkpoint' },
    { id: 'model_nllb-200-distilled-600m', title: '6. NLLB-200 Prekladač', desc: 'SK -> ZH neurónový preklad' },
    { id: 'model_piper-zh-huayan', title: '7. Piper TTS Čínsky Hlas', desc: 'Komerčne bezpečný syntetizátor' },
    { id: 'model_latentsync-1-5', title: '8. LatentSync 1.5 Model Váhy', desc: 'Lip-sync UNet checkpoint pre 12 GB VRAM' },
    { id: 'model_musetalk-weights', title: '9. MuseTalk Záložný Model', desc: 'Odľahčený OOM fallback model' },
  ];

  const fetchDiagnostics = async () => {
    setIsLoadingReport(true);
    try {
      const rep = await invokeCommand<SystemDiagnosticsReport>('run_system_diagnostics');
      setReport(rep);
      const mList = await invokeCommand<ModelManifestItem[]>('get_models_manifest');
      setModels(mList);
    } catch (err) {
      console.error('Failed to run diagnostics', err);
    } finally {
      setIsLoadingReport(false);
    }
  };

  useEffect(() => {
    fetchDiagnostics();
  }, []);

  // Listen for logs from backend
  useEffect(() => {
    const unsubscribe = addTauriListener('wizard_log_event', (payload: ProcessLogLine) => {
      setInstallerLogs((prev) => [...prev.slice(-400), payload]);
    });
    return () => unsubscribe();
  }, []);

  const handleRunAllSteps = async (startFromIndex: number = 0) => {
    setIsRunningAll(true);
    setFailedStepIndex(null);
    setActiveTab('wizard_run');
    if (startFromIndex === 0) {
      setInstallerLogs([]);
    }

    for (let i = startFromIndex; i < wizardSteps.length; i++) {
      setCurrentStepIndex(i);
      const step = wizardSteps[i];
      try {
        setInstallerLogs((prev) => [
          ...prev,
          {
            stream: 'system',
            message: `>>> [${new Date().toLocaleTimeString()}] Spúšťam krok ${i + 1}/${wizardSteps.length}: ${step.title}...`,
            timestamp_ms: Date.now(),
            is_progress: false,
            progress_percent: null,
            step_tag: step.id,
          },
        ]);

        const res = await invokeCommand<boolean>('run_wizard_step', { step_id: step.id });
        if (!res) {
          throw new Error(`Krok '${step.title}' vrátil neúspešný stav.`);
        }

        setInstallerLogs((prev) => [
          ...prev,
          {
            stream: 'system',
            message: `✓ Krok ${i + 1}/${wizardSteps.length}: ${step.title} bol ÚSPEŠNE DOKONČENÝ.`,
            timestamp_ms: Date.now(),
            is_progress: false,
            progress_percent: 100,
            step_tag: step.id,
          },
        ]);
      } catch (err: any) {
        console.error(`Step ${step.id} failed`, err);
        setFailedStepIndex(i);
        setInstallerLogs((prev) => [
          ...prev,
          {
            stream: 'stderr',
            message: `❌ CHYBA v kroku "${step.title}": ${err?.message || err}`,
            timestamp_ms: Date.now(),
            is_progress: false,
            progress_percent: null,
            step_tag: step.id,
          },
        ]);
        break;
      }
    }

    setIsRunningAll(false);
    await fetchDiagnostics();
  };

  const handleFixSingleItem = async (itemId: string) => {
    setActiveTab('wizard_run');
    setInstallerLogs([]);
    setIsRunningAll(true);
    setFailedStepIndex(null);

    try {
      const stepId = itemId.startsWith('model_')
        ? itemId
        : itemId.includes('wsl') || itemId.includes('distro')
        ? 'wsl_install'
        : itemId.includes('pkg')
        ? 'system_packages'
        : itemId.includes('python') || itemId.includes('torch')
        ? 'python_rocm'
        : itemId.includes('latentsync') || itemId.includes('musetalk')
        ? 'lipsync_repos'
        : itemId;

      await invokeCommand('run_wizard_step', { step_id: stepId });
    } catch (err) {
      console.error(err);
    } finally {
      setIsRunningAll(false);
      await fetchDiagnostics();
    }
  };

  const handleDownloadModel = async (modelId: string) => {
    setDownloadingModelId(modelId);
    try {
      await invokeCommand('run_wizard_step', { step_id: `model_${modelId}` });
      await fetchDiagnostics();
    } catch (err) {
      console.error(err);
    } finally {
      setDownloadingModelId(null);
    }
  };

  const handleCancel = async () => {
    await invokeCommand('cancel_wizard_install');
    setIsRunningAll(false);
  };

  const copyToClipboard = (text: string) => {
    navigator.clipboard.writeText(text);
    setCopiedCmd(text);
    setTimeout(() => setCopiedCmd(null), 2000);
  };

  const installedModelSet = new Set(
    report?.items.filter((i) => i.is_installed).map((i) => i.id) || []
  );

  return (
    <div className="space-y-6 max-w-5xl mx-auto pb-12">
      {/* Header bar */}
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4 pb-4 border-b border-slate-800">
        <div>
          <div className="flex items-center gap-2">
            <Wrench className="w-5 h-5 text-indigo-400" />
            <h2 className="text-xl font-bold text-slate-100">Sprievodca Inštaláciou & Diagnostika</h2>
            <Badge variant="primary">Idempotentný Setup</Badge>
          </div>
          <p className="text-xs text-slate-400 mt-1">
            Automatická konfigurácia WSL2, ROCm knižníc, AI modelov a overenie hardvéru.
          </p>
        </div>

        <div className="flex items-center gap-2">
          {isRunningAll ? (
            <Button
              variant="danger"
              size="sm"
              leftIcon={<Square className="w-3.5 h-3.5" />}
              onClick={handleCancel}
            >
              Zrušiť inštaláciu
            </Button>
          ) : (
            <Button
              variant="primary"
              size="sm"
              leftIcon={<Play className="w-3.5 h-3.5" />}
              onClick={() => handleRunAllSteps(0)}
            >
              Automaticky nainštalovať všetko
            </Button>
          )}
        </div>
      </div>

      {/* Tabs */}
      <div className="flex items-center gap-2 border-b border-slate-800 pb-2">
        <button
          onClick={() => setActiveTab('diagnostics')}
          className={`flex items-center gap-2 px-3 py-1.5 rounded-lg text-xs font-medium transition-all ${
            activeTab === 'diagnostics'
              ? 'bg-indigo-600 text-white'
              : 'text-slate-400 hover:text-slate-200 bg-slate-900/60'
          }`}
        >
          <Layers className="w-3.5 h-3.5" />
          Prehľad & Diagnostika
        </button>

        <button
          onClick={() => setActiveTab('wizard_run')}
          className={`flex items-center gap-2 px-3 py-1.5 rounded-lg text-xs font-medium transition-all ${
            activeTab === 'wizard_run'
              ? 'bg-indigo-600 text-white'
              : 'text-slate-400 hover:text-slate-200 bg-slate-900/60'
          }`}
        >
          <RefreshCw className={`w-3.5 h-3.5 ${isRunningAll ? 'animate-spin' : ''}`} />
          Priebeh Inštalácie
        </button>

        <button
          onClick={() => setActiveTab('models')}
          className={`flex items-center gap-2 px-3 py-1.5 rounded-lg text-xs font-medium transition-all ${
            activeTab === 'models'
              ? 'bg-indigo-600 text-white'
              : 'text-slate-400 hover:text-slate-200 bg-slate-900/60'
          }`}
        >
          <DownloadCloud className="w-3.5 h-3.5" />
          Sťahovanie Modelov
        </button>

        <button
          onClick={() => setActiveTab('guide')}
          className={`flex items-center gap-2 px-3 py-1.5 rounded-lg text-xs font-medium transition-all ${
            activeTab === 'guide'
              ? 'bg-indigo-600 text-white'
              : 'text-slate-400 hover:text-slate-200 bg-slate-900/60'
          }`}
        >
          <HelpCircle className="w-3.5 h-3.5" />
          Manuálny Návod
        </button>
      </div>

      {/* Tab 1: Diagnostics */}
      {activeTab === 'diagnostics' && (
        <DiagnosticsReport
          report={report}
          isLoading={isLoadingReport}
          onRefresh={fetchDiagnostics}
          onFixItem={handleFixSingleItem}
        />
      )}

      {/* Tab 2: Wizard Execution & Live Terminal */}
      {activeTab === 'wizard_run' && (
        <div className="space-y-5">
          {/* Progress Overview */}
          <Card className="space-y-4">
            <div className="flex items-center justify-between">
              <div>
                <h4 className="font-semibold text-sm text-slate-200">
                  {isRunningAll
                    ? `Vykonávam krok ${currentStepIndex + 1}/${wizardSteps.length}: ${wizardSteps[currentStepIndex]?.title}`
                    : failedStepIndex !== null
                    ? `Inštalácia sa zastavila na kroku ${failedStepIndex + 1}: ${wizardSteps[failedStepIndex]?.title}`
                    : 'Inštalácia pripravená'}
                </h4>
                <p className="text-xs text-slate-400 mt-0.5">
                  {wizardSteps[currentStepIndex]?.desc || 'Vyberte akciu alebo spustite celú inštaláciu.'}
                </p>
              </div>
              <div className="flex items-center gap-2">
                {failedStepIndex !== null && !isRunningAll && (
                  <Button
                    variant="secondary"
                    size="sm"
                    leftIcon={<RotateCcw className="w-3.5 h-3.5 text-amber-400" />}
                    onClick={() => handleRunAllSteps(failedStepIndex)}
                  >
                    Opakovať od kroku {failedStepIndex + 1}
                  </Button>
                )}
                <Badge variant={isRunningAll ? 'primary' : failedStepIndex !== null ? 'danger' : 'secondary'}>
                  {isRunningAll ? 'Inštaluje sa...' : failedStepIndex !== null ? 'Zlyhané' : 'Neaktívne'}
                </Badge>
              </div>
            </div>

            <ProgressBar
              progress={((currentStepIndex + (isRunningAll ? 0.5 : 0)) / wizardSteps.length) * 100}
              variant="gradient"
              showLabel
            />
          </Card>

          {/* Terminal Logs */}
          <Card className="p-0 overflow-hidden bg-slate-950 border-slate-800">
            <div className="px-4 py-2 bg-slate-900 border-b border-slate-800 flex items-center justify-between">
              <span className="text-xs font-mono text-slate-400">Live Inštalačný Log</span>
              <Button
                variant="ghost"
                size="sm"
                onClick={() => setInstallerLogs([])}
                className="text-[11px] h-6 px-2"
              >
                Vymazať
              </Button>
            </div>

            <div className="p-4 h-80 overflow-y-auto font-mono text-xs text-slate-300 space-y-1 custom-scrollbar">
              {installerLogs.length === 0 ? (
                <div className="text-slate-600 text-center py-16">
                  Žiadne logy. Kliknite na "Automaticky nainštalovať všetko" alebo vyberte komponent na inštaláciu.
                </div>
              ) : (
                installerLogs.map((log, idx) => (
                  <div
                    key={idx}
                    className={`leading-relaxed ${
                      log.stream === 'stderr'
                        ? 'text-rose-400 font-semibold'
                        : log.stream === 'system'
                        ? 'text-indigo-400 font-semibold'
                        : 'text-slate-300'
                    }`}
                  >
                    <span className="text-slate-600 select-none mr-2">
                      [{new Date(log.timestamp_ms).toLocaleTimeString()}]
                    </span>
                    {log.message}
                  </div>
                ))
              )}
            </div>
          </Card>
        </div>
      )}

      {/* Tab 3: Models */}
      {activeTab === 'models' && (
        <CheckpointDownloader
          models={models}
          installedModelIds={installedModelSet}
          downloadingModelId={downloadingModelId}
          downloadProgress={75}
          onDownloadModel={handleDownloadModel}
        />
      )}

      {/* Tab 4: Manual Instructions & Troubleshooting */}
      {activeTab === 'guide' && (
        <div className="space-y-4">
          <Card className="space-y-4">
            <div className="flex items-center gap-2 text-indigo-400">
              <HelpCircle className="w-5 h-5" />
              <h3 className="font-semibold text-base text-slate-100">
                Manuálna inštalácia WSL2 a ROCm (v prípade potreby)
              </h3>
            </div>
            <p className="text-xs text-slate-400 leading-relaxed">
              Ak inštalácia WSL2 vyžaduje zvýšené administrátorské práva na vašom Windows PC, môžete
              otvoriť <strong>PowerShell ako Administrátor</strong> a spustiť nasledovné príkazy manuálne:
            </p>

            {/* Step 1 Command */}
            <div className="space-y-1">
              <label className="text-xs font-semibold text-slate-300">1. Inštalácia WSL2 s Ubuntu 24.04:</label>
              <div className="bg-slate-950 p-3 rounded-lg border border-slate-800 flex items-center justify-between font-mono text-xs text-slate-200">
                <code>wsl --install -d Ubuntu-24.04</code>
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={() => copyToClipboard('wsl --install -d Ubuntu-24.04')}
                >
                  {copiedCmd === 'wsl --install -d Ubuntu-24.04' ? (
                    <Check className="w-3.5 h-3.5 text-emerald-400" />
                  ) : (
                    <Copy className="w-3.5 h-3.5" />
                  )}
                </Button>
              </div>
            </div>

            {/* Step 2 Command */}
            <div className="space-y-1">
              <label className="text-xs font-semibold text-slate-300">
                2. Inštalácia PyTorch ROCm vo vnútri Ubuntu 24.04:
              </label>
              <div className="bg-slate-950 p-3 rounded-lg border border-slate-800 flex items-center justify-between font-mono text-xs text-slate-200">
                <code>pip install --pre torch torchvision torchaudio --index-url https://download.pytorch.org/whl/rocm6.2</code>
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={() =>
                    copyToClipboard(
                      'pip install --pre torch torchvision torchaudio --index-url https://download.pytorch.org/whl/rocm6.2'
                    )
                  }
                >
                  {copiedCmd?.includes('rocm6.2') ? (
                    <Check className="w-3.5 h-3.5 text-emerald-400" />
                  ) : (
                    <Copy className="w-3.5 h-3.5" />
                  )}
                </Button>
              </div>
            </div>

            {/* License Note Alert */}
            <div className="p-4 rounded-xl bg-amber-500/10 border border-amber-500/30 text-amber-300 text-xs flex items-start gap-3">
              <ShieldAlert className="w-5 h-5 text-amber-400 flex-shrink-0 mt-0.5" />
              <div>
                <strong className="font-semibold block text-amber-200">Licenčné odporúčanie pre TTS:</strong>
                Pre komerčné nasadenie zvoľte v nastaveniach <strong>Piper TTS (MIT)</strong> alebo <strong>Kokoro TTS (Apache 2.0)</strong>. Model Coqui XTTS-v2 je viazaný nekomerčnou CPML licenciou.
              </div>
            </div>

            {/* Windows SmartScreen & Defender Guide */}
            <div className="p-4 rounded-xl bg-indigo-500/10 border border-indigo-500/30 text-slate-200 text-xs space-y-3">
              <div className="flex items-center gap-2 text-indigo-400 font-semibold text-sm">
                <ShieldAlert className="w-4 h-4" />
                <span>Čo robiť, ak Windows SmartScreen zobrazí modré varovanie?</span>
              </div>
              <p className="text-slate-300 text-xs leading-relaxed">
                Pretože ide o open-source nástroj zostavený pre vaše PC, Windows SmartScreen môže pri prvom spustení zobraziť: <em>"Systém Windows ochránil váš počítač"</em> (neznámy vydavateľ).
              </p>
              <div className="bg-slate-950 p-3 rounded-lg border border-slate-800 space-y-1.5 text-[11px]">
                <p><strong>1. Krok:</strong> V modrom okne kliknite na text <strong>"Ďalšie informácie" (More info)</strong>.</p>
                <p><strong>2. Krok:</strong> V pravom dolnom rohu kliknite na tlačidlo <strong>"Spustiť aj tak" (Run anyway)</strong>.</p>
                <p><strong>3. Krok (Alternatíva cez Vlastnosti):</strong> Kliknite pravým tlačidlom na stiahnutý súbor &rarr; <em>Vlastnosti (Properties)</em> &rarr; v záložke Všeobecné zaškrtnite <strong>Odblokovať (Unblock)</strong> &rarr; Použiť.</p>
              </div>
            </div>
          </Card>
        </div>
      )}
    </div>
  );
};
