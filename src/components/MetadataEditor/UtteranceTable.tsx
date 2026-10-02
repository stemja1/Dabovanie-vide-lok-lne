import React, { useState, useEffect, useMemo, useCallback } from 'react';
import {
  Save,
  Plus,
  Play,
  RotateCcw,
  Sparkles,
  FileCheck,
  ArrowRight,
  Download,
  AlertCircle,
  HelpCircle,
  Search,
  Copy,
  Check,
  FileText,
} from 'lucide-react';
import { UtteranceItem, UtteranceMetadataDocument } from '../../types/metadata';
import { invokeCommand } from '../../utils/tauriBridge';
import { formatSrtTimestamp } from '../../utils/formatters';
import { Button } from '../ui/Button';
import { Badge } from '../ui/Badge';
import { Card } from '../ui/Card';
import { UtteranceRow } from './UtteranceRow';

/** Mirrors the `ReviewMetadataPayload` returned by the Rust backend. */
interface ReviewMetadataPayload {
  document: UtteranceMetadataDocument;
  file_path: string | null;
  is_real_run: boolean;
}

interface UtteranceTableProps {
  onConfirmAndContinue?: () => void;
  isPausedForReview?: boolean;
}

export const UtteranceTable: React.FC<UtteranceTableProps> = ({
  onConfirmAndContinue,
  isPausedForReview = false,
}) => {
  const [doc, setDoc] = useState<UtteranceMetadataDocument | null>(null);
  const [playingId, setPlayingId] = useState<string | null>(null);
  const [hasUnsavedChanges, setHasUnsavedChanges] = useState<boolean>(false);
  const [isSaving, setIsSaving] = useState<boolean>(false);
  const [searchQuery, setSearchQuery] = useState<string>('');
  const [copiedNotification, setCopiedNotification] = useState<string | null>(null);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  /**
   * Absolute path returned by the backend alongside the document. Edits must be
   * written back here, otherwise the pipeline keeps reading its own unmodified
   * file and every review change is silently discarded.
   */
  const [savePath, setSavePath] = useState<string | null>(null);
  const [isRealRun, setIsRealRun] = useState<boolean>(false);

  const loadData = useCallback(async () => {
    setErrorMessage(null);
    try {
      const payload = await invokeCommand<ReviewMetadataPayload>('get_review_utterance_metadata');
      setDoc(payload.document);
      setSavePath(payload.file_path ?? null);
      setIsRealRun(payload.is_real_run);
      setHasUnsavedChanges(false);
    } catch (err) {
      console.error('Failed to load utterance metadata', err);
      setErrorMessage(
        err instanceof Error
          ? err.message
          : 'Nepodarilo sa načítať metadáta z pipeline. Spustite fázy 1–3 a skúste znova.'
      );
    }
  }, []);

  useEffect(() => {
    loadData();
  }, [loadData]);

  /**
   * Recomputes the document duration the same way the Rust backend does in
   * `recalculate_timings`: as the latest `end_time`, not the sum of segment
   * durations. Summing durations ignores the silence between segments, so the
   * value drifted lower than the real video length and stage 4 sized the master
   * dubbed track too short, cutting off the tail of the dialogue.
   */
  const withRecalculatedTiming = (utterances: UtteranceItem[]): Partial<UtteranceMetadataDocument> => {
    const sorted = [...utterances].sort((a, b) => a.start_time - b.start_time);
    const maxEnd = sorted.reduce((acc, u) => Math.max(acc, u.end_time), 0);
    return {
      utterances: sorted.map((u) => ({
        ...u,
        duration: Number(Math.max(0.05, u.end_time - u.start_time).toFixed(2)),
      })),
      total_duration: Number(maxEnd.toFixed(2)),
    };
  };

  const handleUpdateUtterance = (updated: UtteranceItem) => {
    if (!doc) return;
    setDoc({ ...doc, ...withRecalculatedTiming(doc.utterances.map((u) => (u.id === updated.id ? updated : u))) });
    setHasUnsavedChanges(true);
  };

  const handleDeleteUtterance = (id: string) => {
    if (!doc) return;
    setDoc({ ...doc, ...withRecalculatedTiming(doc.utterances.filter((u) => u.id !== id)) });
    setHasUnsavedChanges(true);
  };

  const handleAddUtterance = () => {
    if (!doc) return;
    const lastUtt = doc.utterances[doc.utterances.length - 1];
    const newStart = lastUtt ? Number((lastUtt.end_time + 0.5).toFixed(2)) : 0.0;
    const newEnd = Number((newStart + 3.0).toFixed(2));
    const newId = `utt_${(doc.utterances.length + 1).toString().padStart(3, '0')}`;

    const newUtt: UtteranceItem = {
      id: newId,
      start_time: newStart,
      end_time: newEnd,
      duration: 3.0,
      speaker_id: 'SPEAKER_00',
      slovak_text: 'Nová replika zadaná používateľom.',
      chinese_text: '用户输入的新配音台词。',
      target_audio_file: `audio_segments/${newId}.wav`,
      speed_factor: 1.0,
      is_edited: true,
      confidence: 1.0,
      words: [],
    };

    setDoc({ ...doc, ...withRecalculatedTiming([...doc.utterances, newUtt]) });
    setHasUnsavedChanges(true);
  };

  const handleSaveChanges = async (): Promise<boolean> => {
    if (!doc) return false;

    if (!savePath) {
      // No pipeline run has produced a file yet, so there is nowhere correct to
      // write. Previously this saved to 'utterance_metadata.json' relative to the
      // process CWD, which no stage ever read.
      setErrorMessage(
        'Nie je aktívny žiadny dabingový beh, takže nie je kam uložiť metadáta. ' +
          'Najprv spustite fázy 1–3 (Demux, ASR, Preklad).'
      );
      return false;
    }

    setIsSaving(true);
    setErrorMessage(null);
    try {
      await invokeCommand('save_utterance_metadata', {
        file_path: savePath,
        document: { ...doc, is_verified_by_user: true },
      });
      setHasUnsavedChanges(false);
      setCopiedNotification('Metadáta uložené do pipeline súboru.');
      setTimeout(() => setCopiedNotification(null), 3000);
      return true;
    } catch (err) {
      console.error('Failed to save metadata', err);
      setErrorMessage(
        err instanceof Error
          ? err.message
          : 'Uloženie metadát zlyhalo. Skontrolujte, či súbor nie je otvorený v inom programe.'
      );
      return false;
    } finally {
      setIsSaving(false);
    }
  };

  const handleExportSrt = () => {
    if (!doc) return;
    let srt = '';
    let cue = 0;
    doc.utterances.forEach((utt) => {
      const zh = (utt.chinese_text || '').trim();
      const sk = (utt.slovak_text || '').trim();
      if (!zh && !sk) return;
      cue += 1;
      srt += `${cue}\n${formatSrtTimestamp(utt.start_time)} --> ${formatSrtTimestamp(
        Math.max(utt.start_time + 0.2, utt.end_time)
      )}\n${zh}\n${sk}\n\n`;
    });

    navigator.clipboard.writeText(srt);
    setCopiedNotification(`SRT titulky skopírované (${cue} cues).`);
    setTimeout(() => setCopiedNotification(null), 2500);
  };

  const handleTogglePlay = (id: string) => {
    if (playingId === id) {
      setPlayingId(null);
    } else {
      setPlayingId(id);
      setTimeout(() => {
        setPlayingId((curr) => (curr === id ? null : curr));
      }, 3000);
    }
  };

  const filteredUtterances = useMemo(() => {
    if (!doc) return [];
    if (!searchQuery.trim()) return doc.utterances;
    const q = searchQuery.toLowerCase();
    return doc.utterances.filter(
      (u) =>
        u.slovak_text.toLowerCase().includes(q) ||
        u.chinese_text.toLowerCase().includes(q) ||
        u.speaker_id.toLowerCase().includes(q)
    );
  }, [doc, searchQuery]);

  if (!doc) {
    return (
      <div className="max-w-6xl mx-auto p-8 space-y-4">
        {errorMessage ? (
          <div className="p-4 rounded-xl bg-rose-500/10 border border-rose-500/30 text-rose-300 flex items-start gap-3">
            <AlertCircle className="w-5 h-5 flex-shrink-0 mt-0.5" />
            <div>
              <strong className="block font-semibold text-rose-200">Nepodarilo sa načítať metadáta</strong>
              <span className="text-xs text-rose-300/80">{errorMessage}</span>
            </div>
          </div>
        ) : (
          <div className="text-center text-slate-400">Načítavam utterance_metadata...</div>
        )}
        <div className="text-center">
          <Button variant="secondary" size="sm" onClick={loadData}>
            <RotateCcw className="w-3.5 h-3.5 mr-1.5" />
            Skúsiť znova
          </Button>
        </div>
      </div>
    );
  }

  return (
    <div className="space-y-6 max-w-6xl mx-auto pb-12">
      {errorMessage && (
        <div className="p-4 rounded-xl bg-rose-500/10 border border-rose-500/30 text-rose-300 flex items-start gap-3">
          <AlertCircle className="w-5 h-5 flex-shrink-0 mt-0.5" />
          <div>
            <strong className="block font-semibold text-rose-200">Uloženie metadát zlyhalo</strong>
            <span className="text-xs text-rose-300/80">{errorMessage}</span>
          </div>
        </div>
      )}

      {/* Top Header & Actions */}
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4 pb-4 border-b border-slate-800">
        <div>
          <div className="flex items-center gap-2">
            <h2 className="text-xl font-bold text-slate-100">Editor Metadát & Prekladu</h2>
            <Badge variant="primary">utterance_metadata.json</Badge>
            {hasUnsavedChanges && <Badge variant="warning">Neuložené zmeny</Badge>}
            {!isRealRun && <Badge variant="secondary">Ukážkové dáta</Badge>}
            {copiedNotification && (
              <Badge variant="success" className="animate-fadeIn">
                <Check className="w-3 h-3 mr-1 inline" /> {copiedNotification}
              </Badge>
            )}
          </div>
          <p className="text-xs text-slate-400 mt-1">
            Interaktívna kontrola ASR segmentácie, úprava čínštiny, ladenie rýchlosti TTS pred syntézou reči.
          </p>
        </div>

        <div className="flex items-center gap-2 flex-wrap">
          <Button
            variant="ghost"
            size="sm"
            leftIcon={<FileText className="w-3.5 h-3.5 text-slate-400" />}
            onClick={handleExportSrt}
            className="text-xs"
          >
            Kopírovať SRT
          </Button>

          <Button
            variant="secondary"
            size="sm"
            leftIcon={<Plus className="w-3.5 h-3.5" />}
            onClick={handleAddUtterance}
          >
            Pridať repliku
          </Button>

          <Button
            variant="secondary"
            size="sm"
            leftIcon={<Save className="w-3.5 h-3.5" />}
            onClick={handleSaveChanges}
            isLoading={isSaving}
          >
            Uložiť zmeny
          </Button>

          {isPausedForReview && onConfirmAndContinue && (
            <Button
              variant="primary"
              size="sm"
              rightIcon={<ArrowRight className="w-3.5 h-3.5" />}
              onClick={async () => {
                // Only resume the pipeline once the edits actually reached the
                // file stage 4 reads; otherwise TTS would run against the
                // pre-review translations.
                const saved = await handleSaveChanges();
                if (saved && onConfirmAndContinue) {
                  onConfirmAndContinue();
                }
              }}
            >
              Potvrdiť a spustiť TTS
            </Button>
          )}
        </div>
      </div>

      {/* Metadata Header Summary Bar */}
      <div className="grid grid-cols-2 sm:grid-cols-4 gap-3 bg-slate-900/60 p-4 rounded-xl border border-slate-800">
        <div>
          <span className="text-[11px] text-slate-400 block">Zdrojové video:</span>
          <span className="font-semibold text-xs text-slate-200 truncate block">
            {doc.video_source}
          </span>
        </div>
        <div>
          <span className="text-[11px] text-slate-400 block">Jazykový pár:</span>
          <span className="font-semibold text-xs text-indigo-400">
            {doc.source_language} → {doc.target_language}
          </span>
        </div>
        <div>
          <span className="text-[11px] text-slate-400 block">Počet replík:</span>
          <span className="font-semibold text-xs text-slate-200">
            {doc.utterances.length} segmentov {searchQuery && `(${filteredUtterances.length} nájdených)`}
          </span>
        </div>
        <div>
          <span className="text-[11px] text-slate-400 block">Celkové trvanie reči:</span>
          <span className="font-semibold text-xs text-slate-200 font-mono">
            {doc.total_duration.toFixed(2)}s
          </span>
        </div>
      </div>

      {/* Search and filter bar */}
      <div className="relative">
        <Search className="w-4 h-4 text-slate-500 absolute left-3.5 top-1/2 -translate-y-1/2" />
        <input
          type="text"
          value={searchQuery}
          onChange={(e) => setSearchQuery(e.target.value)}
          placeholder="Vyhľadať v slovenskom alebo čínskom texte..."
          className="w-full bg-slate-900/70 border border-slate-800 rounded-xl pl-10 pr-4 py-2 text-xs text-slate-200 placeholder-slate-500 focus:outline-none focus:border-indigo-500 transition-colors"
        />
        {searchQuery && (
          <button
            onClick={() => setSearchQuery('')}
            className="absolute right-3 top-1/2 -translate-y-1/2 text-xs text-slate-400 hover:text-slate-200"
          >
            Vyčistiť
          </button>
        )}
      </div>

      {/* Main Table */}
      <Card className="p-0 overflow-hidden border-slate-800 shadow-xl shadow-slate-950/40">
        <div className="overflow-x-auto">
          <table className="w-full text-left border-collapse">
            <thead>
              <tr className="bg-slate-950/90 border-b border-slate-800 text-[11px] font-semibold text-slate-400 uppercase tracking-wider">
                <th className="py-3 px-3 text-center w-12">Audio</th>
                <th className="py-3 px-3 w-36">Hovorca & Čas</th>
                <th className="py-3 px-4 w-1/3">Slovenský originál (ASR Whisper)</th>
                <th className="py-3 px-4">Čínsky preklad & TTS nastavenie</th>
                <th className="py-3 px-3 text-right w-12">Akcie</th>
              </tr>
            </thead>
            <tbody>
              {filteredUtterances.length === 0 ? (
                <tr>
                  <td colSpan={5} className="py-12 text-center text-slate-500 text-xs">
                    {searchQuery ? 'Žiadne repliky nezodpovedajú vyhľadávaniu.' : 'Žiadne repliky. Kliknite na "Pridať repliku" alebo spustite fázu ASR.'}
                  </td>
                </tr>
              ) : (
                filteredUtterances.map((item, idx) => (
                  <UtteranceRow
                    key={item.id}
                    item={item}
                    index={idx}
                    onUpdate={handleUpdateUtterance}
                    onDelete={handleDeleteUtterance}
                    isPlaying={playingId === item.id}
                    onTogglePlay={handleTogglePlay}
                  />
                ))
              )}
            </tbody>
          </table>
        </div>
      </Card>
    </div>
  );
};

