import React from 'react';
import { CheckCircle2, XCircle, AlertCircle, RefreshCw, Terminal, ArrowRight } from 'lucide-react';
import { DependencyCheckItem, SystemDiagnosticsReport } from '../../types/wizard';
import { Button } from '../ui/Button';
import { Badge } from '../ui/Badge';
import { Card } from '../ui/Card';

interface DiagnosticsReportProps {
  report: SystemDiagnosticsReport | null;
  isLoading: boolean;
  onRefresh: () => void;
  onFixItem: (itemId: string) => void;
}

export const DiagnosticsReport: React.FC<DiagnosticsReportProps> = ({
  report,
  isLoading,
  onRefresh,
  onFixItem,
}) => {
  if (isLoading && !report) {
    return (
      <div className="flex flex-col items-center justify-center p-12 space-y-4">
        <RefreshCw className="w-8 h-8 text-indigo-400 animate-spin" />
        <p className="text-sm text-slate-400">Prebieha hĺbková diagnostika WSL2, ROCm a modelových závislostí...</p>
      </div>
    );
  }

  if (!report) {
    return (
      <div className="p-8 text-center text-slate-400">
        <p>Žiadne diagnostické dáta. Kliknite na 'Spustiť kontrolu'.</p>
        <Button variant="primary" size="sm" className="mt-4" onClick={onRefresh}>
          Spustiť kontrolu
        </Button>
      </div>
    );
  }

  const criticalFailed = report.items.filter((i) => i.is_critical && !i.is_installed);
  const optionalFailed = report.items.filter((i) => !i.is_critical && !i.is_installed);

  const getCategoryTitle = (cat: string) => {
    switch (cat) {
      case 'wsl': return 'WSL2 & Linux Prostredie';
      case 'system': return 'Systémové Balíčky Ubuntu';
      case 'python': return 'Python & PyTorch ROCm';
      case 'repos': return 'AI Repozitáre (LatentSync / MuseTalk)';
      case 'models': return 'Modelové Checkpointy & Váhy';
      default: return 'Ostatné komponenty';
    }
  };

  const categories = Array.from(new Set(report.items.map((i) => i.category)));

  return (
    <div className="space-y-6">
      {/* Top Banner Summary */}
      <div
        className={`p-5 rounded-xl border flex items-center justify-between ${
          report.all_ok
            ? 'bg-emerald-500/10 border-emerald-500/30 text-emerald-300'
            : criticalFailed.length > 0
            ? 'bg-rose-500/10 border-rose-500/30 text-rose-300'
            : 'bg-amber-500/10 border-amber-500/30 text-amber-300'
        }`}
      >
        <div className="flex items-center gap-4">
          <div
            className={`p-2.5 rounded-xl ${
              report.all_ok
                ? 'bg-emerald-500/20 text-emerald-400'
                : criticalFailed.length > 0
                ? 'bg-rose-500/20 text-rose-400'
                : 'bg-amber-500/20 text-amber-400'
            }`}
          >
            {report.all_ok ? (
              <CheckCircle2 className="w-6 h-6" />
            ) : criticalFailed.length > 0 ? (
              <XCircle className="w-6 h-6" />
            ) : (
              <AlertCircle className="w-6 h-6" />
            )}
          </div>
          <div>
            <h4 className="font-semibold text-base text-slate-100">
              {report.all_ok
                ? 'Systém je 100% pripravený na lokálny AI dabing!'
                : `Pripravenosť prostredia: ${report.readiness_percentage.toFixed(0)} %`}
            </h4>
            <p className="text-xs text-slate-300/80 mt-0.5">
              {report.all_ok
                ? 'Všetky kritické komponenty (WSL2, PyTorch ROCm, modely) sú overené a funkčné.'
                : criticalFailed.length > 0
                ? `Chýba ${criticalFailed.length} kritických komponentov. Spustite automatickú inštaláciu nižšie.`
                : `Všetky kritické moduly fungujú. Chýba ${optionalFailed.length} voliteľných rozšírení.`}
            </p>
          </div>
        </div>

        <Button
          variant="secondary"
          size="sm"
          leftIcon={<RefreshCw className={`w-3.5 h-3.5 ${isLoading ? 'animate-spin' : ''}`} />}
          onClick={onRefresh}
          disabled={isLoading}
        >
          Preveriť stav
        </Button>
      </div>

      {/* Items by Category */}
      <div className="space-y-5">
        {categories.map((cat) => {
          const items = report.items.filter((i) => i.category === cat);
          return (
            <Card key={cat} className="space-y-3">
              <h4 className="text-xs font-semibold text-slate-400 uppercase tracking-wider flex items-center justify-between">
                <span>{getCategoryTitle(cat)}</span>
                <span className="text-[11px] font-normal lowercase text-slate-500">
                  {items.filter((i) => i.is_installed).length} z {items.length} pripravené
                </span>
              </h4>

              <div className="divide-y divide-slate-800/80">
                {items.map((item) => (
                  <div key={item.id} className="py-3 flex items-start justify-between gap-4">
                    <div className="flex items-start gap-3">
                      <div className="mt-0.5">
                        {item.is_installed ? (
                          <CheckCircle2 className="w-4 h-4 text-emerald-400" />
                        ) : item.is_critical ? (
                          <XCircle className="w-4 h-4 text-rose-400" />
                        ) : (
                          <AlertCircle className="w-4 h-4 text-amber-400" />
                        )}
                      </div>
                      <div>
                        <div className="flex items-center gap-2">
                          <span className="font-medium text-sm text-slate-200">{item.title}</span>
                          {item.version_detected && (
                            <Badge variant="secondary" size="sm">
                              {item.version_detected}
                            </Badge>
                          )}
                          {!item.is_critical && (
                            <Badge variant="warning" size="sm">
                              Voliteľné
                            </Badge>
                          )}
                        </div>
                        <p className="text-xs text-slate-400 mt-0.5">{item.description}</p>
                        {item.error_message && (
                          <p className="text-xs text-rose-400/90 mt-1 flex items-center gap-1.5 font-mono">
                            <span>Chyba:</span> {item.error_message}
                          </p>
                        )}
                        {item.fix_hint && !item.is_installed && (
                          <p className="text-[11px] text-slate-500 mt-0.5 italic">{item.fix_hint}</p>
                        )}
                      </div>
                    </div>

                    {!item.is_installed && (
                      <Button
                        variant="primary"
                        size="sm"
                        rightIcon={<ArrowRight className="w-3.5 h-3.5" />}
                        onClick={() => onFixItem(item.id)}
                      >
                        Nainštalovať
                      </Button>
                    )}
                  </div>
                ))}
              </div>
            </Card>
          );
        })}
      </div>
    </div>
  );
};
