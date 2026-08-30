import React, { useState } from 'react';
import { Download, CheckCircle2, ShieldCheck, AlertTriangle, HardDrive, RefreshCw } from 'lucide-react';
import { ModelManifestItem } from '../../types/wizard';
import { formatBytes } from '../../utils/formatters';
import { Button } from '../ui/Button';
import { Badge } from '../ui/Badge';
import { Card } from '../ui/Card';
import { ProgressBar } from '../ui/ProgressBar';

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
  downloadProgress,
  onDownloadModel,
}) => {
  const [filterCategory, setFilterCategory] = useState<string>('all');

  const filteredModels = models.filter((m) => {
    if (filterCategory === 'all') return true;
    return m.category === filterCategory;
  });

  const totalSizeMb = models.reduce((acc, m) => acc + m.approximate_size_mb, 0);
  const installedCount = models.filter((m) => installedModelIds.has(`model_${m.id}`)).length;

  return (
    <div className="space-y-6">
      {/* Category Filter & Storage Summary */}
      <div className="flex flex-col sm:flex-row items-start sm:items-center justify-between gap-4 bg-slate-900/60 p-4 rounded-xl border border-slate-800">
        <div className="flex items-center gap-2 overflow-x-auto pb-1 sm:pb-0">
          {[
            { id: 'all', label: 'Všetky modely' },
            { id: 'asr', label: 'ASR (Whisper)' },
            { id: 'translation', label: 'Preklad (NLLB)' },
            { id: 'tts', label: 'Syntéza (TTS)' },
            { id: 'lipsync', label: 'Lip-Sync (LatentSync/MuseTalk)' },
          ].map((cat) => (
            <button
              key={cat.id}
              onClick={() => setFilterCategory(cat.id)}
              className={`px-3 py-1.5 rounded-lg text-xs font-medium transition-all ${
                filterCategory === cat.id
                  ? 'bg-indigo-600 text-white shadow-sm'
                  : 'bg-slate-800 text-slate-400 hover:text-slate-200'
              }`}
            >
              {cat.label}
            </button>
          ))}
        </div>

        <div className="flex items-center gap-3 text-xs text-slate-400">
          <HardDrive className="w-4 h-4 text-slate-500" />
          <span>
            Nainštalované: <strong className="text-slate-200">{installedCount} / {models.length}</strong> (celkovo ~{formatBytes(totalSizeMb)})
          </span>
        </div>
      </div>

      {/* Models Grid */}
      <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
        {filteredModels.map((model) => {
          const isInstalled = installedModelIds.has(`model_${model.id}`);
          const isDownloading = downloadingModelId === model.id;

          return (
            <Card
              key={model.id}
              className={`flex flex-col justify-between border transition-all ${
                isInstalled
                  ? 'border-emerald-500/20 bg-emerald-950/10'
                  : 'border-slate-800 hover:border-slate-700'
              }`}
            >
              <div className="space-y-3">
                <div className="flex items-start justify-between gap-2">
                  <div>
                    <div className="flex items-center gap-2">
                      <h4 className="font-semibold text-sm text-slate-100">{model.name}</h4>
                      {isInstalled && (
                        <CheckCircle2 className="w-4 h-4 text-emerald-400 flex-shrink-0" />
                      )}
                    </div>
                    <p className="text-xs text-slate-400 mt-1 leading-relaxed">{model.description}</p>
                  </div>
                </div>

                <div className="flex flex-wrap items-center gap-2 pt-1">
                  <Badge variant="secondary" size="sm">
                    ~{formatBytes(model.approximate_size_mb)}
                  </Badge>

                  {model.is_commercial_safe ? (
                    <Badge variant="success" size="sm" icon={<ShieldCheck className="w-3 h-3" />}>
                      {model.license}
                    </Badge>
                  ) : (
                    <Badge variant="warning" size="sm" icon={<AlertTriangle className="w-3 h-3" />}>
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

              {/* Action / Progress */}
              <div className="pt-4 mt-3 border-t border-slate-800/80 flex items-center justify-between">
                <span className="text-[11px] font-mono text-slate-500 truncate max-w-[200px]">
                  {model.local_relative_path}
                </span>

                {isDownloading ? (
                  <div className="w-32">
                    <ProgressBar progress={downloadProgress} size="sm" variant="gradient" showLabel />
                  </div>
                ) : isInstalled ? (
                  <Button
                    variant="outline"
                    size="sm"
                    leftIcon={<RefreshCw className="w-3 h-3" />}
                    onClick={() => onDownloadModel(model.id)}
                  >
                    Overiť súbory
                  </Button>
                ) : (
                  <Button
                    variant="primary"
                    size="sm"
                    leftIcon={<Download className="w-3.5 h-3.5" />}
                    onClick={() => onDownloadModel(model.id)}
                  >
                    Stiahnuť
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
