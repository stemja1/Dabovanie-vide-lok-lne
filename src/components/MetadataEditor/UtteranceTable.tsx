import React, { useState, useEffect, useMemo } from 'react';
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
import { Button } from '../ui/Button';
import { Badge } from '../ui/Badge';
import { Card } from '../ui/Card';
import { UtteranceRow } from './UtteranceRow';

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

  const loadData = async () => {
    try {
      const data = await invokeCommand<UtteranceMetadataDocument>('get_demo_utterance_metadata');
      setDoc(data);
      setHasUnsavedChanges(false);
    } catch (err) {
      console.error('Failed to load utterance metadata', err);
    }
  };

  useEffect(() => {
    loadData();
  }, []);

  const handleUpdateUtterance = (updated: UtteranceItem) => {
    if (!doc) return;
    const newUtts = doc.utterances.map((u) => (u.id === updated.id ? updated : u));
    setDoc({
      ...doc,
      utterances: newUtts,
      total_duration: Number(newUtts.reduce((acc, u) => acc + u.duration, 0).toFixed(2)),
    });
    setHasUnsavedChanges(true);
  };

  const handleDeleteUtterance = (id: string) => {
    if (!doc) return;
    const newUtts = doc.utterances.filter((u) => u.id !== id);
    setDoc({
      ...doc,
      utterances: newUtts,
      total_duration: Number(newUtts.reduce((acc, u) => acc + u.duration, 0).toFixed(2)),
    });
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

    const newUtts = [...doc.utterances, newUtt];
    setDoc({
      ...doc,
      utterances: newUtts,
      total_duration: Number(newUtts.reduce((acc, u) => acc + u.duration, 0).toFixed(2)),
    });
    setHasUnsavedChanges(true);
  };

  const handleSaveChanges = async () => {
    if (!doc) return;
    setIsSaving(true);
    try {
      await invokeCommand('save_utterance_metadata', {
        file_path: 'utterance_metadata.json',
        document: { ...doc, is_verified_by_user: true },
      });
      setHasUnsavedChanges(false);
      setCopiedNotification('Metadáta úspešne uložené!');
      setTimeout(() => setCopiedNotification(null), 2500);
    } catch (err) {
      console.error('Failed to save metadata', err);
    } finally {
      setIsSaving(false);
    }
  };

  const handleExportSrt = () => {
    if (!doc) return;
    let srt = '';
    doc.utterances.forEach((utt, idx) => {
      const formatTime = (secs: number) => {
        const h = Math.floor(secs / 3600);
        const m = Math.floor((secs % 3600) / 60);
        const s = Math.floor(secs % 60);
        const ms = Math.round((secs - Math.floor(secs)) * 1000);
        return `${String(h).padStart(2, '0')}:${String(m).padStart(2, '0')}:${String(s).padStart(2, '0')},${String(ms).padStart(3, '0')}`;
      };
      srt += `${idx + 1}\n${formatTime(utt.start_time)} --> ${formatTime(utt.end_time)}\n${utt.chinese_text}\n${utt.slovak_text}\n\n`;
    });

    navigator.clipboard.writeText(srt);
    setCopiedNotification('SRT titulky skopírované do schránky!');
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
    return <div className="p-8 text-center text-slate-400">Načítavam utterance_metadata...</div>;
  }

  return (
    <div className="space-y-6 max-w-6xl mx-auto pb-12">
      {/* Top Header & Actions */}
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4 pb-4 border-b border-slate-800">
        <div>
          <div className="flex items-center gap-2">
            <h2 className="text-xl font-bold text-slate-100">Editor Metadát & Prekladu</h2>
            <Badge variant="primary">utterance_metadata.json</Badge>
            {hasUnsavedChanges && <Badge variant="warning">Neuložené zmeny</Badge>}
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
                await handleSaveChanges();
                onConfirmAndContinue();
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

