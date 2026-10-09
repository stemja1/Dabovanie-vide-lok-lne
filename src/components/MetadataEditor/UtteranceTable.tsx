import React, { useState, useEffect, useMemo, useCallback, useRef } from 'react';
import {
  Save,
  Plus,
  RotateCcw,
  Volume2,
  ArrowRight,
  AlertCircle,
  Search,
  Check,
  FileText,
} from 'lucide-react';
import { UtteranceItem, UtteranceMetadataDocument } from '../../types/metadata';
import { invokeCommand, convertVideoPathToUrl } from '../../utils/tauriBridge';
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
  // Mirrors `doc` outside React state so a `beforeunload` guard can read it
  // without re-subscribing (and re-registering the listener) on every keystroke.
  const unsavedRef = useRef(false);
  const [hasUnsavedChanges, setHasUnsavedChanges] = useState(false);
  const markDirty = useCallback(() => {
    unsavedRef.current = true;
    setHasUnsavedChanges(true);
  }, []);
  const markClean = useCallback(() => {
    unsavedRef.current = false;
    setHasUnsavedChanges(false);
  }, []);

  // `App.tsx` renders this editor conditionally, so switching tabs unmounts it and
  // the document - including unsaved Chinese translations - was discarded on the
  // next mount with no prompt. Warn before losing work.
  useEffect(() => {
    const handler = (e: BeforeUnloadEvent) => {
      if (!unsavedRef.current) return;
      e.preventDefault();
      e.returnValue = '';
    };
    window.addEventListener('beforeunload', handler);
    return () => window.removeEventListener('beforeunload', handler);
  }, []);

  const loadData = useCallback(async () => {
    setErrorMessage(null);
    try {
      const payload = await invokeCommand<ReviewMetadataPayload>('get_review_utterance_metadata');
      setDoc(payload.document);
      setSavePath(payload.file_path ?? null);
      setIsRealRun(payload.is_real_run);
      markClean();
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
  const withRecalculatedTiming = (
    utterances: UtteranceItem[],
    sort = true
  ): Partial<UtteranceMetadataDocument> => {
    // `total_duration` sa počíta z najneskoršieho `end_time` nezávisle na
    // poradí, takže na výpočet netreba sortovať. Ten bol príčinou toho, že
    // každý stlačený kláves v poli "Od:" preusporiadal tabuľku a riadok, do
    // ktorého sa práve písalo, mohol skočiť na vrch - mid-word.
    const maxEnd = utterances.reduce((acc, u) => Math.max(acc, u.end_time), 0);
    const ordered = sort ? [...utterances].sort((a, b) => a.start_time - b.start_time) : utterances;
    return {
      utterances: ordered.map((u) => ({
        ...u,
        duration: Number(Math.max(0.05, u.end_time - u.start_time).toFixed(2)),
      })),
      total_duration: Number(maxEnd.toFixed(2)),
    };
  };

  const handleUpdateUtterance = (updated: UtteranceItem) => {
    if (!doc) return;
    // Bez `sort` - editácia poľa nesmie preskupovať tabuľku pod rukami.
    setDoc({
      ...doc,
      ...withRecalculatedTiming(
        doc.utterances.map((u) => (u.id === updated.id ? updated : u)),
        false
      ),
    });
    markDirty();
  };

  const handleDeleteUtterance = (id: string) => {
    if (!doc) return;
    setDoc({ ...doc, ...withRecalculatedTiming(doc.utterances.filter((u) => u.id !== id)) });
    markDirty();
  };

  const handleAddUtterance = () => {
    if (!doc) return;
    // Anchor on the furthest END time, not on the last array element. After a
    // delete the array length no longer matches the numbering, and overlapping
    // cues were inserted inside an existing utterance.
    const maxEnd = doc.utterances.reduce((acc, u) => Math.max(acc, u.end_time || 0), 0);
    const newStart = Number((maxEnd + 0.5).toFixed(2));
    const newEnd = Number((newStart + 3.0).toFixed(2));

    // `length + 1` collided with an existing id after any deletion (e.g. delete
    // utt_001 from a 3-item list -> new id utt_003). The backend rejects duplicate
    // ids outright, so from that point on EVERY save failed with
    // "Duplicitný identifikátor repliky". Derive the next free number instead.
    const usedNumbers = new Set<number>();
    doc.utterances.forEach((u) => {
      const m = /^utt_(\d+)$/.exec(u.id);
      if (m) usedNumbers.add(Number(m[1]));
    });
    let nextNum = doc.utterances.length + 1;
    while (usedNumbers.has(nextNum)) nextNum += 1;
    const newId = `utt_${nextNum.toString().padStart(3, '0')}`;

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
    markDirty();
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

    // `is_real_run === false` means the backend had no real document and returned
    // 3 hardcoded demo sentences. Saving those over the pipeline's metadata file
    // made the fake data indistinguishable from real ASR output afterwards.
    if (!isRealRun) {
      setErrorMessage(
        'Tieto metadáta sú ukážkové (nie sú výstupom rozpoznania reči). ' +
          'Spustite najprv fázy 1–3 (Demux, ASR, Preklad), aby sa vytvoril ' +
          'skutočný preklad na úpravu.'
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
      markClean();
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

  // Tlačidlo ▶ predtým len prepínalo ikonu na ⏸ a po 3 sekundách späť. V
  // `UtteranceTable` ani `UtteranceRow` nebol žiadny `<audio>` element, takže
  // používateľ si myslel, že počúva vlastný hlas - a počunal ticho. Teraz
  // prehrávame reálny segment z disku a `isPlaying` sa odvádza z udalostí
  // prehrávača (`play` / `ended` / `error`), nie z časovača.
  const audioRef = useRef<HTMLAudioElement | null>(null);
  const [audioError, setAudioError] = useState<string | null>(null);

  const handleTogglePlay = async (id: string) => {
    if (playingId === id) {
      audioRef.current?.pause();
      setPlayingId(null);
      return;
    }
    if (!doc) return;

    const utt = doc.utterances.find((u) => u.id === id);
    if (!utt) return;

    // `target_audio_file` je cesta relatívna voči workspacetu vo WSL
    // (`audio_segments/utt_001.wav`); `savePath` ukazuje na ten istý
    // workspace, takže z neho vieme odvodiť absolútnu cestu.
    if (!savePath) {
      setAudioError('Najprv spustite fázy 1–3, aby vznikli audio segmenty.');
      return;
    }
    const workspace = savePath.replace(/[\\/][^\\/]*_utterance_metadata\.json$/i, '');
    const rel = (utt.target_audio_file || `audio_segments/${utt.id}.wav`).replace(/\\/g, '/');
    const absPath = `${workspace}/${rel}`;

    if (!audioRef.current) {
      audioRef.current = new Audio();
    }
    const audio = audioRef.current;

    audio.onended = () => setPlayingId(null);
    audio.onerror = () => {
      setPlayingId(null);
      setAudioError(
        `Segment ${utt.id} sa nepodarilo prehrať (${absPath}). Spustením fázy 4 vznikne TTS audio.`
      );
    };

    try {
      setAudioError(null);
      // `convertVideoPathToUrl` je async (pre Tauri asset protocol), takže
      // `src` sa nesmie nastaviť na Promise.
      audio.src = await convertVideoPathToUrl(absPath);
      await audio.play();
      setPlayingId(id);
    } catch (err) {
      console.error('Prehratie segmentu zlyhalo', err);
      setPlayingId(null);
      setAudioError(`Prehratie zlyhalo: ${err instanceof Error ? err.message : String(err)}`);
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

      {audioError && (
        <div
          role="status"
          aria-live="polite"
          className="flex items-start gap-2.5 rounded-lg border border-amber-800/60 bg-amber-950/40 px-4 py-3 text-sm text-amber-200"
        >
          <Volume2 className="w-4 h-4 mt-0.5 flex-shrink-0" />
          <div className="flex-1">
            <span className="text-amber-300/90">{audioError}</span>
          </div>
          <button
            type="button"
            onClick={() => setAudioError(null)}
            className="text-amber-400 hover:text-amber-200 text-xs"
          >
            Zavrieť
          </button>
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

