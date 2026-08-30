import React, { useState, useEffect } from 'react';
import {
  Settings,
  Save,
  RotateCcw,
  ShieldCheck,
  AlertTriangle,
  FileCode,
  Sliders,
  Check,
  Cpu,
} from 'lucide-react';
import { AppConfig, TtsEngine, LipsyncEngine, AsrEngine, AsrDevice } from '../../types/config';
import { invokeCommand } from '../../utils/tauriBridge';
import { Button } from '../ui/Button';
import { Badge } from '../ui/Badge';
import { Card } from '../ui/Card';

interface SettingsModalProps {
  onSaved?: () => void;
}

export const SettingsModal: React.FC<SettingsModalProps> = ({ onSaved }) => {
  const [config, setConfig] = useState<AppConfig | null>(null);
  const [activeTab, setActiveTab] = useState<'ui' | 'toml'>('ui');
  const [tomlText, setTomlText] = useState<string>('');
  const [isSaving, setIsSaving] = useState<boolean>(false);
  const [savedSuccess, setSavedSuccess] = useState<boolean>(false);

  const loadConfig = async () => {
    try {
      const cfg = await invokeCommand<AppConfig>('get_config');
      setConfig(cfg);
      const toml = await invokeCommand<string>('export_config_toml');
      setTomlText(toml);
    } catch (err) {
      console.error('Failed to load config', err);
    }
  };

  useEffect(() => {
    loadConfig();
  }, []);

  const handleSave = async () => {
    if (!config) return;
    setIsSaving(true);
    try {
      if (activeTab === 'toml') {
        const parsed = await invokeCommand<AppConfig>('import_config_toml', { toml_str: tomlText });
        setConfig(parsed);
      } else {
        await invokeCommand('save_config', { new_config: config });
        const toml = await invokeCommand<string>('export_config_toml');
        setTomlText(toml);
      }
      setSavedSuccess(true);
      setTimeout(() => setSavedSuccess(false), 2500);
      if (onSaved) onSaved();
    } catch (err) {
      console.error('Failed to save config', err);
    } finally {
      setIsSaving(false);
    }
  };

  const handleReset = async () => {
    try {
      const def = await invokeCommand<AppConfig>('reset_config_to_default');
      setConfig(def);
      const toml = await invokeCommand<string>('export_config_toml');
      setTomlText(toml);
    } catch (err) {
      console.error('Failed to reset config', err);
    }
  };

  if (!config) {
    return <div className="p-8 text-center text-slate-400">Načítavam konfiguráciu...</div>;
  }

  return (
    <div className="space-y-6 max-w-4xl mx-auto pb-12">
      {/* Header */}
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4 pb-4 border-b border-slate-800">
        <div>
          <div className="flex items-center gap-2">
            <Sliders className="w-5 h-5 text-indigo-400" />
            <h2 className="text-xl font-bold text-slate-100">Konfigurácia Aplikácie</h2>
            <Badge variant="secondary">config.toml</Badge>
          </div>
          <p className="text-xs text-slate-400 mt-1">
            Nastavenia WSL2, modelov, ROCm akcelerácie a komerčných licencií.
          </p>
        </div>

        <div className="flex items-center gap-2">
          <Button variant="outline" size="sm" leftIcon={<RotateCcw className="w-3.5 h-3.5" />} onClick={handleReset}>
            Obnoviť predvolené
          </Button>

          <Button
            variant="primary"
            size="sm"
            leftIcon={savedSuccess ? <Check className="w-3.5 h-3.5 text-emerald-400" /> : <Save className="w-3.5 h-3.5" />}
            onClick={handleSave}
            isLoading={isSaving}
          >
            {savedSuccess ? 'Uložené!' : 'Uložiť nastavenia'}
          </Button>
        </div>
      </div>

      {/* Tabs */}
      <div className="flex items-center gap-2 border-b border-slate-800 pb-2">
        <button
          onClick={() => setActiveTab('ui')}
          className={`flex items-center gap-2 px-3 py-1.5 rounded-lg text-xs font-medium transition-all ${
            activeTab === 'ui' ? 'bg-indigo-600 text-white' : 'text-slate-400 hover:text-slate-200 bg-slate-900/60'
          }`}
        >
          <Sliders className="w-3.5 h-3.5" />
          Vizuálny konfigurátor
        </button>

        <button
          onClick={() => setActiveTab('toml')}
          className={`flex items-center gap-2 px-3 py-1.5 rounded-lg text-xs font-medium transition-all ${
            activeTab === 'toml' ? 'bg-indigo-600 text-white' : 'text-slate-400 hover:text-slate-200 bg-slate-900/60'
          }`}
        >
          <FileCode className="w-3.5 h-3.5" />
          Raw TOML Editor
        </button>
      </div>

      {activeTab === 'ui' ? (
        <div className="space-y-6">
          {/* Section 1: TTS Engine & License Compliance */}
          <Card className="space-y-4 border-indigo-500/30">
            <div className="flex items-center justify-between">
              <div>
                <h3 className="text-sm font-semibold text-slate-100">Syntéza Reči (TTS Engine)</h3>
                <p className="text-xs text-slate-400 mt-0.5">Výber modelu na syntézu čínskeho hlasu.</p>
              </div>
              <Badge variant="primary">Komerčná licencia</Badge>
            </div>

            <div className="grid grid-cols-1 md:grid-cols-3 gap-3">
              {/* Option 1: Piper */}
              <div
                onClick={() => setConfig({ ...config, tts_engine: 'piper', tts_voice: 'zh_CN-huayan-medium' })}
                className={`p-4 rounded-xl border cursor-pointer transition-all ${
                  config.tts_engine === 'piper'
                    ? 'border-indigo-500 bg-indigo-950/20 shadow-md shadow-indigo-600/10'
                    : 'border-slate-800 bg-slate-950/40 hover:border-slate-700'
                }`}
              >
                <div className="flex justify-between items-start">
                  <h4 className="font-semibold text-xs text-slate-200">Piper TTS</h4>
                  <Badge variant="success" size="sm" icon={<ShieldCheck className="w-3 h-3" />}>MIT</Badge>
                </div>
                <p className="text-[11px] text-slate-400 mt-1">Ultra-rýchly, ľahký na CPU/GPU. Plne bezpečný pre komerčné použitie.</p>
                <span className="text-[10px] text-indigo-400 font-mono mt-2 block">Hlas: zh_CN-huayan-medium</span>
              </div>

              {/* Option 2: Kokoro */}
              <div
                onClick={() => setConfig({ ...config, tts_engine: 'kokoro', tts_voice: 'kokoro_zh_multilingual' })}
                className={`p-4 rounded-xl border cursor-pointer transition-all ${
                  config.tts_engine === 'kokoro'
                    ? 'border-indigo-500 bg-indigo-950/20 shadow-md shadow-indigo-600/10'
                    : 'border-slate-800 bg-slate-950/40 hover:border-slate-700'
                }`}
              >
                <div className="flex justify-between items-start">
                  <h4 className="font-semibold text-xs text-slate-200">Kokoro TTS</h4>
                  <Badge variant="success" size="sm" icon={<ShieldCheck className="w-3 h-3" />}>Apache 2.0</Badge>
                </div>
                <p className="text-[11px] text-slate-400 mt-1">Moderný 82M neurónový model s vysokou kvalitou intonácie.</p>
                <span className="text-[10px] text-indigo-400 font-mono mt-2 block">Hlas: kokoro_zh</span>
              </div>

              {/* Option 3: Coqui XTTS */}
              <div
                onClick={() => setConfig({ ...config, tts_engine: 'coqui_xtts', tts_voice: 'xtts_voice_clone' })}
                className={`p-4 rounded-xl border cursor-pointer transition-all ${
                  config.tts_engine === 'coqui_xtts'
                    ? 'border-indigo-500 bg-indigo-950/20 shadow-md shadow-indigo-600/10'
                    : 'border-slate-800 bg-slate-950/40 hover:border-slate-700'
                }`}
              >
                <div className="flex justify-between items-start">
                  <h4 className="font-semibold text-xs text-slate-200">Coqui XTTS-v2</h4>
                  <Badge variant="warning" size="sm" icon={<AlertTriangle className="w-3 h-3" />}>CPML</Badge>
                </div>
                <p className="text-[11px] text-amber-400/90 mt-1">UPOZORNENIE: Nekomerčná licencia. Určené len na testovanie.</p>
                <span className="text-[10px] text-slate-500 font-mono mt-2 block">Klonovanie hlasu</span>
              </div>
            </div>
          </Card>

          {/* Section 2: Lip-sync & ROCm Attention Fallback */}
          <Card className="space-y-4">
            <div className="flex items-center justify-between">
              <div>
                <h3 className="text-sm font-semibold text-slate-100">Lip-Sync & ROCm Optimalizácie</h3>
                <p className="text-xs text-slate-400 mt-0.5">Voľba modelu tvárovej animácie a hardvérových fallbackov.</p>
              </div>
            </div>

            <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
              <div
                onClick={() => setConfig({ ...config, lipsync_engine: 'latentsync_1_5' })}
                className={`p-4 rounded-xl border cursor-pointer transition-all ${
                  config.lipsync_engine === 'latentsync_1_5'
                    ? 'border-indigo-500 bg-indigo-950/20'
                    : 'border-slate-800 bg-slate-950/40'
                }`}
              >
                <div className="flex justify-between items-start">
                  <h4 className="font-semibold text-xs text-slate-200">LatentSync 1.5 (Odporúčané)</h4>
                  <Badge variant="primary" size="sm">~7.5 GB VRAM</Badge>
                </div>
                <p className="text-[11px] text-slate-400 mt-1">
                  Vysoká fotorealistická presnosť synchronizácie pier. Beží bezpečne v rámci 12 GB limitu RX 7700 XT.
                </p>
              </div>

              <div
                onClick={() => setConfig({ ...config, lipsync_engine: 'musetalk' })}
                className={`p-4 rounded-xl border cursor-pointer transition-all ${
                  config.lipsync_engine === 'musetalk'
                    ? 'border-indigo-500 bg-indigo-950/20'
                    : 'border-slate-800 bg-slate-950/40'
                }`}
              >
                <div className="flex justify-between items-start">
                  <h4 className="font-semibold text-xs text-slate-200">MuseTalk (Ľahký Fallback)</h4>
                  <Badge variant="secondary" size="sm">~4.5 GB VRAM</Badge>
                </div>
                <p className="text-[11px] text-slate-400 mt-1">
                  Rýchly engine s nízkymi nárokmi na pamäť. Vhodný pri obmedzenej VRAM alebo rýchlom náhľade.
                </p>
              </div>
            </div>

            {/* Checkbox: ROCm SDPA Native fallback */}
            <div className="p-3 bg-slate-950/60 border border-slate-800 rounded-xl flex items-center justify-between">
              <div className="space-y-0.5">
                <span className="text-xs font-semibold text-slate-200 flex items-center gap-1.5">
                  <Cpu className="w-3.5 h-3.5 text-indigo-400" />
                  ROCm SDPA Attention Fallback (Náhrada xFormers)
                </span>
                <p className="text-[11px] text-slate-400">
                  Automaticky nahrádza CUDA xFormers za natívne PyTorch Scaled Dot-Product Attention na AMD ROCm.
                </p>
              </div>
              <input
                type="checkbox"
                checked={config.rocm_sdpa_fallback}
                onChange={(e) => setConfig({ ...config, rocm_sdpa_fallback: e.target.checked })}
                className="w-4 h-4 rounded bg-slate-900 border-slate-700 text-indigo-600 focus:ring-indigo-500 cursor-pointer"
              />
            </div>

            {/* Checkbox: Auto OOM fallback */}
            <div className="p-3 bg-slate-950/60 border border-slate-800 rounded-xl flex items-center justify-between">
              <div className="space-y-0.5">
                <span className="text-xs font-semibold text-slate-200">
                  Automatický MuseTalk Fallback pri OOM
                </span>
                <p className="text-[11px] text-slate-400">
                  Ak LatentSync narazí na vyčerpanie pamäte grafickej karty, automaticky dokončí video cez MuseTalk.
                </p>
              </div>
              <input
                type="checkbox"
                checked={config.lipsync_fallback_on_oom}
                onChange={(e) => setConfig({ ...config, lipsync_fallback_on_oom: e.target.checked })}
                className="w-4 h-4 rounded bg-slate-900 border-slate-700 text-indigo-600 focus:ring-indigo-500 cursor-pointer"
              />
            </div>
          </Card>

          {/* Section 3: ASR & Review Step */}
          <Card className="space-y-4">
            <h3 className="text-sm font-semibold text-slate-100">ASR & Pipeline Workflow</h3>

            <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
              <div>
                <label className="text-xs font-semibold text-slate-300 block mb-1">ASR Model:</label>
                <select
                  value={config.asr_engine}
                  onChange={(e) => setConfig({ ...config, asr_engine: e.target.value as AsrEngine })}
                  className="w-full bg-slate-950 border border-slate-800 rounded-lg px-3 py-2 text-xs text-slate-200 focus:border-indigo-500"
                >
                  <option value="whisper_sk">Whisper-SK Fine-tune (NaiveNeuron/whisper-large-v3-sk)</option>
                  <option value="faster_whisper">faster-whisper (CTranslate2)</option>
                </select>
              </div>

              <div>
                <label className="text-xs font-semibold text-slate-300 block mb-1">ASR Zariadenie:</label>
                <select
                  value={config.asr_device}
                  onChange={(e) => setConfig({ ...config, asr_device: e.target.value as AsrDevice })}
                  className="w-full bg-slate-950 border border-slate-800 rounded-lg px-3 py-2 text-xs text-slate-200 focus:border-indigo-500"
                >
                  <option value="gpu_rocm">AMD ROCm GPU (Radeon RX 7700 XT)</option>
                  <option value="cpu">CPU Only (Šetrí VRAM pre iné procesy)</option>
                </select>
              </div>
            </div>

            <div className="p-3 bg-slate-950/60 border border-slate-800 rounded-xl flex items-center justify-between">
              <div className="space-y-0.5">
                <span className="text-xs font-semibold text-slate-200">
                  Pozastaviť pipeline po ASR+MT pre kontrolu používateľom
                </span>
                <p className="text-[11px] text-slate-400">
                  Umožňuje editovať preložené repliky v tabuľke pred spustením syntézy reči a lip-syncu.
                </p>
              </div>
              <input
                type="checkbox"
                checked={config.auto_pause_for_review}
                onChange={(e) => setConfig({ ...config, auto_pause_for_review: e.target.checked })}
                className="w-4 h-4 rounded bg-slate-900 border-slate-700 text-indigo-600 focus:ring-indigo-500 cursor-pointer"
              />
            </div>
          </Card>

          {/* Section 4: WSL & Path Settings */}
          <Card className="space-y-4">
            <h3 className="text-sm font-semibold text-slate-100">WSL2 & Cesty k Virtuálnemu Prostrediu</h3>
            <div className="grid grid-cols-1 sm:grid-cols-2 gap-4 text-xs font-mono">
              <div>
                <label className="text-slate-400 block mb-1">WSL Distribúcia:</label>
                <input
                  type="text"
                  value={config.wsl_distro}
                  onChange={(e) => setConfig({ ...config, wsl_distro: e.target.value })}
                  className="w-full bg-slate-950 border border-slate-800 rounded-lg px-3 py-2 text-slate-200 focus:border-indigo-500"
                />
              </div>
              <div>
                <label className="text-slate-400 block mb-1">Python Venv Cesta:</label>
                <input
                  type="text"
                  value={config.venv_path}
                  onChange={(e) => setConfig({ ...config, venv_path: e.target.value })}
                  className="w-full bg-slate-950 border border-slate-800 rounded-lg px-3 py-2 text-slate-200 focus:border-indigo-500"
                />
              </div>
            </div>
          </Card>
        </div>
      ) : (
        <Card className="p-0 overflow-hidden bg-slate-950 border-slate-800">
          <div className="p-3 bg-slate-900 border-b border-slate-800 text-xs text-slate-400">
            Priamy editor súboru <code className="text-indigo-400">config.toml</code>
          </div>
          <textarea
            rows={18}
            value={tomlText}
            onChange={(e) => setTomlText(e.target.value)}
            className="w-full p-4 bg-slate-950 text-slate-200 font-mono text-xs focus:outline-none focus:ring-1 focus:ring-indigo-500 leading-relaxed"
          />
        </Card>
      )}
    </div>
  );
};
