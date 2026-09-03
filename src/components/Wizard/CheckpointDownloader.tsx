import React, { useState } from 'react';
import {
  Download,
  CheckCircle2,
  XCircle,
  ShieldCheck,
  AlertTriangle,
  HardDrive,
  RefreshCw,
  Folder,
  Layers,
  Sparkles,
} from 'lucide-react';
import { ModelManifestItem } from '../../types/wizard';
import { formatBytes } from '../../utils/formatters';
import { Button } from '../ui/Button';
import { Badge } from '../ui/Badge';
import { Card } from '../ui/Card';

interface CheckpointDownloaderProps {
  models: ModelManifestItem[];
  installedModelIds: Set<string>;
  downloadingModelId: string | null;
  downloadProgress: number;
  onDownloadModel: (modelId: string) => void;
}

export const CheckpointDownloader: React.FC<CheckpointDownloaderProps> = ({
  models,
  installedModelIds,
  downloadingModelId,
  onDownloadModel,
}) => {
  const [filterCategory, setFilterCategory] = useState<string>('all');
  const [filterStatus, setFilterStatus] = useState<'all' | 'installed' | 'missing'>('all');

  const isModelInstalled = (modelId: string) => {
    return (
      installedModelIds.has(`model_${modelId}`) ||
      installedModelIds.has(modelId)
    );
  };

  const filteredModels = models.filter((m) => {
    const installed = isModelInstalled(m.id);
    if (filterStatus === 'installed' && !installed) return false;
    if (filterStatus === 'missing' && installed) return false;

    if (filterCategory === 'all') return true;
    return m.category === filterCategory;
  });

  const totalSizeMb = models.reduce((acc, m) => acc + m.approximate_size_mb, 0);
  const installedCount = models.filter((m) => isModelInstalled(m.id)).length;
  const missingCount = models.length - installedCount;

  return (
    <div className="space-y-6">
      {/* Top Banner Status Overview */}
      <div className="bg-slate-900/80 p-5 rounded-2xl border border-slate-800 space-y-4">
        <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4 pb-3 border-b border-slate-800/80">
          <div className="flex items-center gap-3">
            <div className="p-2.5 rounded-xl bg-indigo-600/20 text-indigo-400 border border-indigo-500/30">
              <HardDrive className="w-6 h-6" />
            </div>
            <div>
              <h3 className="font-bold text-base text-slate-100">Správca AI Modelov & Checkpointov</h3>
              <p className="text-xs text-slate-400 mt-0.5">
                Stav stiahnutia a pripravenosti offline neurónových sietí pre slovenský dabing.
              </p>
            </div>
          </div>

          <div className="flex items-center gap-3">
            <div className="flex items-center gap-2 bg-slate-950 px-3.5 py-1.5 rounded-xl border border-slate-800 text-xs">
              <span className="text-emerald-400 font-bold">{installedCount}</span>
              <span className="text-slate-400">nainštalovaných</span>
              <span className="text-slate-600">/</span>
              <span className="text-rose-400 font-bold">{missingCount}</span>
              <span className="text-slate-400">chýba</span>
            </div>
          </div>
        </div>

        {/* Filter Toolbar */}
        <div className="flex flex-wrap items-center justify-between gap-3 pt-1">
          {/* Status Filter Tabs */}
          <div className="flex items-center gap-1.5 bg-slate-950 p-1 rounded-xl border border-slate-800 text-xs">
            <button
              onClick={() => setFilterStatus('all')}
              className={`px-3 py-1 rounded-lg font-medium transition-all ${
                filterStatus === 'all'
                  ? 'bg-indigo-600 text-white shadow'
                  : 'text-slate-400 hover:text-slate-200'
              }`}
            >
              Všetky ({models.length})
            </button>
            <button
              onClick={() => setFilterStatus('installed')}
              className={`px-3 py-1 rounded-lg font-medium transition-all ${
                filterStatus === 'installed'
                  ? 'bg-emerald-600 text-white shadow'
                  : 'text-slate-400 hover:text-slate-200'
              }`}
            >
              ✓ Pripravené ({installedCount})
            </button>
            <button
              onClick={() => setFilterStatus('missing')}
              className={`px-3 py-1 rounded-lg font-medium transition-all ${
                filterStatus === 'missing'
                  ? 'bg-rose-600 text-white shadow'
                  : 'text-slate-400 hover:text-slate-200'
              }`}
            >
              ⏳ Chýbajúce ({missingCount})
            </button>
          </div>

          {/* Category Filter Tabs */}
          <div className="flex items-center gap-1.5 overflow-x-auto text-xs">
            {[
              { id: 'all', label: 'Všetky kategórie' },
              { id: 'asr', label: 'ASR Prepis' },
              { id: 'translation', label: 'Preklad' },
              { id: 'tts', label: 'Hlasy (TTS)' },
              { id: 'lipsync', label: 'Lip-Sync' },
            ].map((cat) => (
              <button
                key={cat.id}
                onClick={() => setFilterCategory(cat.id)}
                className={`px-2.5 py-1 rounded-lg transition-all ${
                  filterCategory === cat.id
                    ? 'bg-slate-800 text-indigo-400 font-medium border border-indigo-500/40'
                    : 'text-slate-400 hover:text-slate-200 bg-slate-950/40'
                }`}
              >
                {cat.label}
              </button>
            ))}
          </div>
        </div>
      </div>

      {/* Models Grid */}
      <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
        {filteredModels.map((model) => {
          const isInstalled = isModelInstalled(model.id);
          const isDownloading = downloadingModelId === model.id;

          return (
            <Card
              key={model.id}
              className={`flex flex-col justify-between border transition-all relative overflow-hidden ${
                isInstalled
                  ? 'border-emerald-500/30 bg-emerald-950/15'
                  : 'border-slate-800 bg-slate-900/50 hover:border-slate-700'
              }`}
            >
              {/* Header Info */}
              <div className="space-y-3">
                <div className="flex items-start justify-between gap-3">
                  <div>
                    <div className="flex items-center gap-2">
                      <h4 className="font-bold text-sm text-slate-100">{model.name}</h4>
                    </div>
                    <p className="text-xs text-slate-400 mt-1 leading-relaxed">
                      {model.description}
                    </p>
                  </div>

                  {/* Prominent Status Pill */}
                  {isInstalled ? (
                    <span className="flex-shrink-0 flex items-center gap-1.5 px-2.5 py-1 rounded-full text-[11px] font-bold bg-emerald-500/20 text-emerald-300 border border-emerald-500/40">
                      <CheckCircle2 className="w-3.5 h-3.5 text-emerald-400" />
                      NAINŠTALOVANÉ
                    </span>
                  ) : (
                    <span className="flex-shrink-0 flex items-center gap-1.5 px-2.5 py-1 rounded-full text-[11px] font-bold bg-amber-500/20 text-amber-300 border border-amber-500/40">
                      <XCircle className="w-3.5 h-3.5 text-amber-400" />
                      CHÝBA
                    </span>
                  )}
                </div>

                {/* Metadata tags */}
                <div className="flex flex-wrap items-center gap-2 pt-1">
                  <Badge variant="secondary" size="sm">
                    ~{formatBytes(model.approximate_size_mb)}
                  </Badge>

                  {model.is_commercial_safe ? (
                    <Badge
                      variant="success"
                      size="sm"
                      icon={<ShieldCheck className="w-3 h-3 text-emerald-400" />}
                    >
                      {model.license}
                    </Badge>
                  ) : (
                    <Badge
                      variant="warning"
                      size="sm"
                      icon={<AlertTriangle className="w-3 h-3 text-amber-400" />}
                    >
                      {model.license}
                    </Badge>
                  )}

                  {model.is_required_for_mvp && (
                    <Badge variant="primary" size="sm">
                      Základný model
                    </Badge>
                  )}
                </div>
              </div>

              {/* Bottom Path & Actions */}
              <div className="pt-4 mt-4 border-t border-slate-800 flex items-center justify-between gap-2">
                <div className="flex items-center gap-1.5 text-[11px] font-mono text-slate-400 truncate max-w-[220px]">
                  <Folder className="w-3.5 h-3.5 text-slate-500 flex-shrink-0" />
                  <span className="truncate">{model.local_relative_path}</span>
                </div>

                {isDownloading ? (
                  <Button variant="secondary" size="sm" disabled leftIcon={<RefreshCw className="w-3.5 h-3.5 animate-spin text-indigo-400" />}>
                    Sťahuje sa...
                  </Button>
                ) : isInstalled ? (
                  <Button
                    variant="outline"
                    size="sm"
                    leftIcon={<RefreshCw className="w-3 h-3 text-slate-400" />}
                    onClick={() => onDownloadModel(model.id)}
                  >
                    Overiť / Znovu stiahnuť
                  </Button>
                ) : (
                  <Button
                    variant="primary"
                    size="sm"
                    leftIcon={<Download className="w-3.5 h-3.5" />}
                    onClick={() => onDownloadModel(model.id)}
                  >
                    Stiahnuť model
                  </Button>
                )}
              </div>
            </Card>
          );
        })}
      </div>
    </div>
  );
};
