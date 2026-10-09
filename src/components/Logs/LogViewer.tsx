import React, { useState, useEffect, useRef, useMemo } from 'react';
import { Terminal, Search, Trash2, Download, Copy, Check, ArrowDown } from 'lucide-react';
import { ProcessLogLine } from '../../types/pipeline';
import { addTauriListener } from '../../utils/tauriBridge';
import { Button } from '../ui/Button';
import { Card } from '../ui/Card';

export const LogViewer: React.FC = () => {
  // Žiadne vymyslené riadky. Predtým tu boli tri hardcoded záznamy, ktoré tvrdili
  // "backend pripravený", "ROCm detegovaná, 12 GB VRAM" a "Piper model načítaný" -
  // ešte predtým, než čokoľvek zbehlo. V zabalenej aplikácii tak tab Logy
  // potvrdzoval stav, ktorý nikto neoveril.
  const [logs, setLogs] = useState<ProcessLogLine[]>([]);

  const [searchQuery, setSearchQuery] = useState<string>('');
  const [streamFilter, setStreamFilter] = useState<'all' | 'stdout' | 'stderr' | 'system'>('all');
  const [autoScroll, setAutoScroll] = useState<boolean>(true);
  const [copied, setCopied] = useState<boolean>(false);
  const [copyFailed, setCopyFailed] = useState<boolean>(false);
  const [showJumpButton, setShowJumpButton] = useState<boolean>(false);
  const logsEndRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const unsubPipeline = addTauriListener('pipeline_log_event', (log: ProcessLogLine) => {
      setLogs((prev) => [...prev.slice(-1000), log]);
    });
    const unsubWizard = addTauriListener('wizard_log_event', (log: ProcessLogLine) => {
      setLogs((prev) => [...prev.slice(-1000), log]);
    });

    return () => {
      unsubPipeline();
      unsubWizard();
    };
  }, []);

  useEffect(() => {
    if (autoScroll) {
      logsEndRef.current?.scrollIntoView({ behavior: 'smooth' });
    }
  }, [logs, autoScroll]);

  // Automatický posun nikdy nesmie utiecť používateľovi z chyby, ktorú práve
  // číta. Predtým scroll bežel pri každom novom riadku, takže počas 40-minútového
  // behu nebolo možné odscrollovať 200 riadkov nahor. Keď používateľ odjede
  // od konca, posun sa vypne sám a dá mu tlačidlo na návrat.
  const handleScroll = (e: React.UIEvent<HTMLDivElement>) => {
    const el = e.currentTarget;
    const distanceFromBottom = el.scrollHeight - el.scrollTop - el.clientHeight;
    const atBottom = distanceFromBottom < 40;
    setAutoScroll(atBottom);
    if (atBottom) setShowJumpButton(false);
  };

  const jumpToLatest = () => {
    setAutoScroll(true);
    setShowJumpButton(false);
    logsEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  };

  const filteredLogs = useMemo(() => {
    const q = searchQuery.trim().toLowerCase();
    return logs.filter((l) => {
      if (streamFilter !== 'all' && l.stream !== streamFilter) return false;
      if (q && !l.message.toLowerCase().includes(q)) return false;
      return true;
    });
  }, [logs, searchQuery, streamFilter]);

  const handleCopyLogs = async () => {
    const text = filteredLogs
      .map(
        (l) =>
          `[${new Date(l.timestamp_ms).toISOString()}] [${l.stream.toUpperCase()}] ${l.message}`
      )
      .join('\n');
    try {
      // Predtým sa `writeText` nezavolalo s `await` a bez `catch`, takže
      // odmietnutie schránky vyrobilo neobsluhované výnimku - a tlačidlo
      // ukázalo "Skopírované" aj keď sa nič neskopírovalo.
      await navigator.clipboard.writeText(text);
      setCopied(true);
    } catch (err) {
      console.error('Kopírovanie logov zlyhalo', err);
      setCopyFailed(true);
      return;
    }
    setTimeout(() => setCopied(false), 2000);
  };

  const handleExportLogs = () => {
    // Exportoval `logs` (nefiltrované), kým Skopírovať kopírovalo
    // `filteredLogs`. Po filtrovaní na stderr si používateľ exportoval iný
    // súbor, než mal na obrazovke.
    const text = filteredLogs
      .map(
        (l) =>
          `[${new Date(l.timestamp_ms).toISOString()}] [${l.stream.toUpperCase()}] ${l.message}`
      )
      .join('\n');
    const blob = new Blob([text], { type: 'text/plain' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `ai_dubbing_logs_${Date.now()}.txt`;
    a.click();
    URL.revokeObjectURL(url);
  };

  return (
    <div className="space-y-4 max-w-6xl mx-auto pb-12">
      {/* Header & Controls */}
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4 pb-2 border-b border-slate-800">
        <div className="flex items-center gap-2">
          <Terminal className="w-5 h-5 text-indigo-400" />
          <div>
            <h2 className="text-xl font-bold text-slate-100">Terminál & Diagnostické Logy</h2>
            <p className="text-xs text-slate-400">Reálny výstup Python subprocessov a WSL2 procesov.</p>
          </div>
        </div>

        <div className="flex items-center gap-2">
          <Button
            variant="secondary"
            size="sm"
            leftIcon={copied ? <Check className="w-3.5 h-3.5 text-emerald-400" /> : <Copy className="w-3.5 h-3.5" />}
            onClick={handleCopyLogs}
          >
            {copied ? 'Skopírované' : copyFailed ? 'Kopírovanie zlyhalo' : 'Kopírovať'}
          </Button>

          <Button
            variant="secondary"
            size="sm"
            leftIcon={<Download className="w-3.5 h-3.5" />}
            onClick={handleExportLogs}
          >
            Exportovať
          </Button>

          <Button
            variant="ghost"
            size="sm"
            leftIcon={<Trash2 className="w-3.5 h-3.5" />}
            onClick={() => setLogs([])}
          >
            Vyčistiť
          </Button>
        </div>
      </div>

      {/* Filter Bar */}
      <div className="flex flex-col sm:flex-row items-center justify-between gap-3 bg-slate-900/60 p-3 rounded-xl border border-slate-800">
        <div className="relative flex-1 w-full sm:w-auto">
          <Search className="w-4 h-4 text-slate-500 absolute left-3 top-2.5" />
          <input
            type="text"
            placeholder="Hľadať v logoch (napr. OOM, ROCm, LatentSync)..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className="w-full bg-slate-950 border border-slate-800 rounded-lg pl-9 pr-3 py-1.5 text-xs text-slate-200 focus:outline-none focus:border-indigo-500 font-mono"
          />
        </div>

        <div className="flex items-center gap-2 self-end sm:self-center">
          <div className="flex items-center gap-1 bg-slate-950 p-1 rounded-lg border border-slate-800 text-xs">
            {(['all', 'stdout', 'stderr', 'system'] as const).map((filter) => (
              <button
                key={filter}
                onClick={() => setStreamFilter(filter)}
                className={`px-2.5 py-1 rounded text-[11px] font-medium transition-all ${
                  streamFilter === filter ? 'bg-indigo-600 text-white shadow-sm' : 'text-slate-400 hover:text-slate-200'
                }`}
              >
                {filter === 'all' ? 'Všetky' : filter.toUpperCase()}
              </button>
            ))}
          </div>

          <label className="flex items-center gap-1.5 text-xs text-slate-400 cursor-pointer select-none">
            <input
              type="checkbox"
              checked={autoScroll}
              onChange={(e) => setAutoScroll(e.target.checked)}
              className="rounded bg-slate-950 border-slate-800 text-indigo-600 focus:ring-indigo-500"
            />
            Auto-scroll
          </label>
        </div>
      </div>

      {/* Terminal View */}
      <Card className="p-0 overflow-hidden bg-slate-950 border-slate-800 shadow-xl relative">
        <div
          onScroll={handleScroll}
          role="log"
          aria-live={autoScroll ? 'polite' : 'off'}
          aria-relevant="additions"
          className="p-4 h-[550px] overflow-y-auto font-mono text-xs leading-relaxed space-y-1 custom-scrollbar"
        >
          {filteredLogs.length === 0 ? (
            <div className="text-slate-500 text-center py-24">
              {logs.length === 0
                ? 'Zatiaľ žiadne logy. Spustite pipeline alebo inštaláciu — výstup sa zobrazí tu.'
                : 'Žiadne logy nezodpovedajú filtru.'}
            </div>
          ) : (
            filteredLogs.map((log, index) => (
              <div
                key={index}
                className={`flex items-start gap-2 ${
                  log.stream === 'stderr'
                    ? 'text-rose-400 bg-rose-950/15 px-2 py-0.5 rounded'
                    : log.stream === 'system'
                    ? 'text-indigo-400 bg-indigo-950/20 px-2 py-0.5 rounded font-semibold'
                    : 'text-slate-300'
                }`}
              >
                <span className="text-slate-600 select-none text-[11px] flex-shrink-0">
                  [{new Date(log.timestamp_ms).toLocaleTimeString()}]
                </span>
                <span className="break-all">{log.message}</span>
              </div>
            ))
          )}
          <div ref={logsEndRef} />
        </div>

        {showJumpButton && !autoScroll && (
          <button
            type="button"
            onClick={jumpToLatest}
            className="absolute bottom-4 left-1/2 -translate-x-1/2 inline-flex items-center gap-1.5 rounded-full bg-indigo-600 px-3 py-1.5 text-xs font-medium text-white shadow-lg hover:bg-indigo-500"
          >
            <ArrowDown className="w-3.5 h-3.5" />
            Skočiť na koniec
          </button>
        )}
      </Card>
    </div>
  );
};
