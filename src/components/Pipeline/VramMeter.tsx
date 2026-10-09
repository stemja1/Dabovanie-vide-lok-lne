import React from 'react';
import { Cpu, ShieldCheck, AlertTriangle, HardDrive, Info } from 'lucide-react';
import { FullPipelineResourceBudget } from '../../types/pipeline';
import { formatBytes } from '../../utils/formatters';
import { Card } from '../ui/Card';
import { Badge } from '../ui/Badge';

interface VramMeterProps {
  budget: FullPipelineResourceBudget | null;
}

export const VramMeter: React.FC<VramMeterProps> = ({ budget }) => {
  if (!budget) return null;

  const vramPercent = (budget.peak_vram_mb / budget.total_gpu_vram_mb) * 100;
  const ramPercent = (budget.peak_ram_mb / budget.total_system_ram_mb) * 100;

  return (
    <Card className="space-y-4 bg-slate-900/50 border-slate-800">
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-2">
        <div className="flex items-center gap-2">
          <Cpu className="w-4 h-4 text-indigo-400" />
          <h4 className="font-semibold text-xs text-slate-300 uppercase tracking-wider">
            Alokácia VRAM & RAM (Sekvenčný Režim)
          </h4>
        </div>
        <div className="flex items-center gap-2">
          {budget.is_overall_safe ? (
            <Badge variant="success" size="sm" icon={<ShieldCheck className="w-3 h-3" />}>
              {/* Predtým tu bolo natvrdo "100% Bezpečné pre 12 GB VRAM & 16 GB RAM".
                  Tieto hodnoty sú konštanty z `vram_estimator.rs`, nie údaje o
                  hardvéri používateľa - na karte s 8 GB VRAM aplikácia tvrdila,
                  že má 12 GB a všetko je v poriadku. Teraz používame hodnoty
                  z payloadu a hovoríme, že je to odhad. */}
              Odhad: bezpečné pre {formatBytes(budget.total_gpu_vram_mb)} VRAM &amp;{' '}
              {formatBytes(budget.total_system_ram_mb)} RAM
            </Badge>
          ) : (
            <Badge variant="warning" size="sm" icon={<AlertTriangle className="w-3 h-3" />}>
              Vysoké zaťaženie VRAM
            </Badge>
          )}
        </div>
      </div>

      <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
        {/* Peak VRAM */}
        <div className="p-3 rounded-lg bg-slate-950/60 border border-slate-800 space-y-2">
          <div className="flex justify-between text-xs">
            <span className="text-slate-400 flex items-center gap-1.5">
              <span className="w-2 h-2 rounded-full bg-rose-500"></span>
              Odhad najvyššej alokácie VRAM
            </span>
            <span className="font-mono text-slate-200 font-semibold">
              {formatBytes(budget.peak_vram_mb)} / {formatBytes(budget.total_gpu_vram_mb)}
            </span>
          </div>
          <div className="w-full h-2 bg-slate-800 rounded-full overflow-hidden">
            <div
              className={`h-full rounded-full transition-all duration-300 ${
                vramPercent > 80 ? 'bg-amber-500' : 'bg-rose-500'
              }`}
              style={{ width: `${vramPercent}%` }}
            />
          </div>
          <p className="text-[10px] text-slate-500">
            Odhad podľa nastavenia, nie meranie. Fázy bežia sekvenčne a pamäť sa
            uvoľňuje po každom modeli. Reálne využitie VRAM sa dá zistiť cez
            kontrolu ROCm.
          </p>
        </div>

        {/* Peak RAM */}
        <div className="p-3 rounded-lg bg-slate-950/60 border border-slate-800 space-y-2">
          <div className="flex justify-between text-xs">
            <span className="text-slate-400 flex items-center gap-1.5">
              <span className="w-2 h-2 rounded-full bg-indigo-500"></span>
              Odhad najvyššej alokácie RAM
            </span>
            <span className="font-mono text-slate-200 font-semibold">
              {formatBytes(budget.peak_ram_mb)} / {formatBytes(budget.total_system_ram_mb)}
            </span>
          </div>
          <div className="w-full h-2 bg-slate-800 rounded-full overflow-hidden">
            <div
              className="h-full bg-indigo-500 rounded-full transition-all duration-300"
              style={{ width: `${ramPercent}%` }}
            />
          </div>
          <p className="text-[10px] text-slate-500">
            Odhad pre nastavenie; porovnania s {formatBytes(budget.total_system_ram_mb)} je
            orientačné, nie meranie vášho počítača.
          </p>
        </div>
      </div>
    </Card>
  );
};
