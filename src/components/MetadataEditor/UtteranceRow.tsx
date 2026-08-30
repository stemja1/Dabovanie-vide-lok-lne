import React, { useState } from 'react';
import { Play, Pause, Trash2, Edit2, Check, Sparkles, Volume2 } from 'lucide-react';
import { UtteranceItem } from '../../types/metadata';
import { formatTimeSeconds } from '../../utils/formatters';
import { Badge } from '../ui/Badge';

interface UtteranceRowProps {
  item: UtteranceItem;
  index: number;
  onUpdate: (updated: UtteranceItem) => void;
  onDelete: (id: string) => void;
  isPlaying: boolean;
  onTogglePlay: (id: string) => void;
}

export const UtteranceRow: React.FC<UtteranceRowProps> = ({
  item,
  index,
  onUpdate,
  onDelete,
  isPlaying,
  onTogglePlay,
}) => {
  const [isEditingZh, setIsEditingZh] = useState<boolean>(false);
  const [zhText, setZhText] = useState<string>(item.chinese_text);

  const handleSaveZh = () => {
    setIsEditingZh(false);
    if (zhText !== item.chinese_text) {
      onUpdate({
        ...item,
        chinese_text: zhText,
        is_edited: true,
      });
    }
  };

  const handleTimingChange = (field: 'start_time' | 'end_time', val: number) => {
    const updated = { ...item, [field]: val };
    updated.duration = Math.max(0.1, Number((updated.end_time - updated.start_time).toFixed(2)));
    onUpdate({ ...updated, is_edited: true });
  };

  const handleSpeedChange = (delta: number) => {
    const newSpeed = Math.min(2.0, Math.max(0.5, Number((item.speed_factor + delta).toFixed(2))));
    onUpdate({ ...item, speed_factor: newSpeed, is_edited: true });
  };

  return (
    <tr className="border-b border-slate-800/60 hover:bg-slate-900/40 transition-colors group">
      {/* Index & Audio Play */}
      <td className="py-3 px-3 align-top text-center w-12">
        <button
          onClick={() => onTogglePlay(item.id)}
          className={`p-2 rounded-lg border transition-all ${
            isPlaying
              ? 'bg-indigo-600 text-white border-indigo-500 shadow-md shadow-indigo-600/30'
              : 'bg-slate-800/80 text-slate-300 border-slate-700 hover:bg-slate-700'
          }`}
          title="Prehrať segment"
        >
          {isPlaying ? <Pause className="w-3.5 h-3.5" /> : <Play className="w-3.5 h-3.5" />}
        </button>
        <div className="text-[10px] text-slate-500 font-mono mt-1">#{index + 1}</div>
      </td>

      {/* Speaker & Timestamp */}
      <td className="py-3 px-3 align-top w-36 space-y-1">
        <Badge variant="secondary" size="sm" className="font-mono text-[10px]">
          {item.speaker_id}
        </Badge>
        <div className="text-xs font-mono text-slate-300">
          <div className="flex items-center gap-1 text-[11px] text-slate-400">
            <span>Od:</span>
            <input
              type="number"
              step="0.1"
              value={item.start_time}
              onChange={(e) => handleTimingChange('start_time', parseFloat(e.target.value) || 0)}
              className="w-14 bg-slate-950 border border-slate-800 rounded px-1 text-slate-200 font-mono text-xs focus:border-indigo-500"
            />
            <span>s</span>
          </div>
          <div className="flex items-center gap-1 text-[11px] text-slate-400 mt-1">
            <span>Do:</span>
            <input
              type="number"
              step="0.1"
              value={item.end_time}
              onChange={(e) => handleTimingChange('end_time', parseFloat(e.target.value) || 0)}
              className="w-14 bg-slate-950 border border-slate-800 rounded px-1 text-slate-200 font-mono text-xs focus:border-indigo-500"
            />
            <span>s</span>
          </div>
          <div className="text-[10px] text-indigo-400/90 mt-1 font-semibold">
            Trvanie: {item.duration.toFixed(2)}s
          </div>
        </div>
      </td>

      {/* Slovak Transcript */}
      <td className="py-3 px-4 align-top w-1/3">
        <div className="p-2.5 rounded-lg bg-slate-950/60 border border-slate-800/80 text-xs text-slate-300 leading-relaxed">
          {item.slovak_text}
        </div>
        {item.words && item.words.length > 0 && (
          <div className="mt-1.5 flex flex-wrap gap-1">
            {item.words.slice(0, 6).map((w, wIdx) => (
              <span key={wIdx} className="text-[10px] text-slate-500 bg-slate-900 px-1.5 py-0.5 rounded font-mono">
                {w.word} ({w.start.toFixed(1)}s)
              </span>
            ))}
            {item.words.length > 6 && (
              <span className="text-[10px] text-slate-600 font-mono">+{item.words.length - 6} slov</span>
            )}
          </div>
        )}
      </td>

      {/* Chinese Translation (Editable) */}
      <td className="py-3 px-4 align-top">
        <div className="space-y-1.5">
          <textarea
            rows={3}
            value={zhText}
            onChange={(e) => setZhText(e.target.value)}
            onBlur={handleSaveZh}
            className="w-full bg-slate-950/90 border border-slate-800 rounded-lg p-2.5 text-xs text-slate-100 focus:outline-none focus:border-indigo-500 focus:ring-1 focus:ring-indigo-500 transition-all font-sans leading-relaxed"
            placeholder="Zadajte alebo upravte čínsky preklad..."
          />
          <div className="flex items-center justify-between text-[11px] text-slate-500">
            <span className="flex items-center gap-1">
              {item.is_edited ? (
                <Badge variant="warning" size="sm">Upravené používateľom</Badge>
              ) : (
                <Badge variant="primary" size="sm">NLLB-200 Automatický preklad</Badge>
              )}
            </span>

            {/* Speed Factor Tuning */}
            <div className="flex items-center gap-1.5">
              <span className="text-slate-400">Rýchlosť TTS:</span>
              <button
                onClick={() => handleSpeedChange(-0.05)}
                className="px-1 bg-slate-800 hover:bg-slate-700 rounded text-slate-300 text-xs"
              >
                -
              </button>
              <span className="font-mono text-slate-200 font-semibold">{item.speed_factor.toFixed(2)}x</span>
              <button
                onClick={() => handleSpeedChange(0.05)}
                className="px-1 bg-slate-800 hover:bg-slate-700 rounded text-slate-300 text-xs"
              >
                +
              </button>
            </div>
          </div>
        </div>
      </td>

      {/* Actions */}
      <td className="py-3 px-3 align-top text-right w-12">
        <button
          onClick={() => onDelete(item.id)}
          className="p-1.5 text-slate-500 hover:text-rose-400 hover:bg-rose-950/30 rounded-lg transition-colors"
          title="Odstrániť repliku"
        >
          <Trash2 className="w-4 h-4" />
        </button>
      </td>
    </tr>
  );
};
