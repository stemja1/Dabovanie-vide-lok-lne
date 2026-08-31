import React from 'react';
import { Cpu, HardDrive, Settings, Wrench, ShieldCheck, AlertTriangle } from 'lucide-react';
import { LiveSystemMetrics } from '../types/pipeline';
import { Badge } from './ui/Badge';
import { Button } from './ui/Button';

interface HeaderProps {
  metrics: LiveSystemMetrics | null;
  onOpenSettings: () => void;
  onOpenWizard: () => void;
  activeTab: string;
}

export const Header: React.FC<HeaderProps> = ({
  metrics,
  onOpenSettings,
  onOpenWizard,
}) => {
  const vramPercent = metrics ? metrics.gpu_vram_percent : 33.5;
  const vramUsedGb = metrics ? (metrics.gpu_vram_used_mb / 1024).toFixed(1) : '4.1';
  const vramTotalGb = metrics ? (metrics.gpu_vram_total_mb / 1024).toFixed(0) : '12';

  const ramPercent = metrics ? metrics.host_ram_percent : 51.4;
  const ramUsedGb = metrics ? (metrics.host_ram_used_mb / 1024).toFixed(1) : '8.4';
  const ramTotalGb = metrics ? (metrics.host_ram_total_mb / 1024).toFixed(0) : '16';

  return (
    <header className="h-16 border-b border-slate-800/80 bg-slate-900/90 backdrop-blur-md px-6 flex items-center justify-between z-30 select-none">
      {/* App Branding */}
      <div className="flex items-center gap-3">
        <div className="w-10 h-10 rounded-xl bg-gradient-to-tr from-indigo-600 via-indigo-500 to-purple-500 p-0.5 shadow-lg shadow-indigo-600/30 flex items-center justify-center">
          <div className="w-full h-full bg-slate-950 rounded-[10px] flex items-center justify-center">
            <span className="font-bold text-sm tracking-tighter bg-gradient-to-r from-indigo-400 to-purple-400 bg-clip-text text-transparent">
              AI-SK
            </span>
          </div>
        </div>
        <div>
          <div className="flex items-center gap-2">
            <h1 className="font-bold text-base text-slate-100 tracking-tight">AI Dabing Štúdio</h1>
            <Badge variant="primary" size="sm">SK → ZH v1.0</Badge>
          </div>
          <p className="text-[11px] text-slate-400 flex items-center gap-1.5">
            <span className="w-1.5 h-1.5 rounded-full bg-emerald-500"></span>
            WSL2 Ubuntu 24.04 • ROCm 6.4.2 • Sekvenčný Pipeline
          </p>
        </div>
      </div>

      {/* Hardware & VRAM Live Monitor Gauges */}
      <div className="hidden md:flex items-center gap-6 bg-slate-950/60 border border-slate-800/80 rounded-xl px-4 py-2">
        {/* GPU VRAM Monitor */}
        <div className="flex items-center gap-3">
          <div className="p-1.5 rounded-lg bg-rose-500/10 text-rose-400 border border-rose-500/20">
            <Cpu className="w-4 h-4" />
          </div>
          <div>
            <div className="flex items-center justify-between gap-2 text-[11px]">
              <span className="text-slate-400 font-medium">RX 7700 XT VRAM</span>
              <span className="font-mono text-slate-200 font-semibold">{vramUsedGb} / {vramTotalGb} GB</span>
            </div>
            <div className="w-28 h-1.5 bg-slate-800 rounded-full overflow-hidden mt-1">
              <div
                className={`h-full rounded-full transition-all duration-300 ${
                  vramPercent > 85 ? 'bg-rose-500' : vramPercent > 70 ? 'bg-amber-500' : 'bg-indigo-500'
                }`}
                style={{ width: `${vramPercent}%` }}
              />
            </div>
          </div>
        </div>

        <div className="w-px h-6 bg-slate-800" />

        {/* System RAM Monitor */}
        <div className="flex items-center gap-3">
          <div className="p-1.5 rounded-lg bg-indigo-500/10 text-indigo-400 border border-indigo-500/20">
            <HardDrive className="w-4 h-4" />
          </div>
          <div>
            <div className="flex items-center justify-between gap-2 text-[11px]">
              <span className="text-slate-400 font-medium">Systémová RAM</span>
              <span className="font-mono text-slate-200 font-semibold">{ramUsedGb} / {ramTotalGb} GB</span>
            </div>
            <div className="w-28 h-1.5 bg-slate-800 rounded-full overflow-hidden mt-1">
              <div
                className={`h-full rounded-full transition-all duration-300 ${
                  ramPercent > 85 ? 'bg-rose-500' : 'bg-emerald-500'
                }`}
                style={{ width: `${ramPercent}%` }}
              />
            </div>
          </div>
        </div>

        <div className="w-px h-6 bg-slate-800" />

        {/* ROCm Native SDPA Status */}
        <div className="flex items-center gap-1.5 text-xs text-slate-300">
          <ShieldCheck className="w-4 h-4 text-emerald-400" />
          <span className="font-medium text-[11px]">ROCm SDPA Aktívny</span>
        </div>
      </div>

      {/* Action Buttons */}
      <div className="flex items-center gap-2">
        <Button
          variant="secondary"
          size="sm"
          leftIcon={<Wrench className="w-3.5 h-3.5 text-indigo-400" />}
          onClick={onOpenWizard}
        >
          Setup & Diagnostika
        </Button>
        <Button
          variant="outline"
          size="sm"
          leftIcon={<Settings className="w-3.5 h-3.5 text-slate-400" />}
          onClick={onOpenSettings}
        >
          Nastavenia
        </Button>
      </div>
    </header>
  );
};
