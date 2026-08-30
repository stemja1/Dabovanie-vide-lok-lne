import React, { useState } from 'react';
import {
  Play,
  Pause,
  Volume2,
  VolumeX,
  Maximize,
  Download,
  FolderOpen,
  Film,
  Sparkles,
  Subtitles,
  Columns,
  Square,
} from 'lucide-react';
import { invokeCommand } from '../../utils/tauriBridge';
import { Button } from '../ui/Button';
import { Badge } from '../ui/Badge';
import { Card } from '../ui/Card';

interface DubbedVideoPlayerProps {
  inputVideoPath?: string;
  outputVideoPath?: string;
  onOpenFolder?: () => void;
}

export const DubbedVideoPlayer: React.FC<DubbedVideoPlayerProps> = ({
  inputVideoPath,
  outputVideoPath,
  onOpenFolder,
}) => {
  const [isPlaying, setIsPlaying] = useState<boolean>(false);
  const [viewMode, setViewMode] = useState<'single' | 'split'>('single');
  const [subtitlesMode, setSubtitlesMode] = useState<'both' | 'zh' | 'sk' | 'none'>('both');
  const [isMuted, setIsMuted] = useState<boolean>(false);
  const [currentTime, setCurrentTime] = useState<number>(3.5);
  const duration = 14.8;

  const currentSubtitle = {
    sk: 'Dobrý deň, vítam vás pri prezentácii nášho nového produktu.',
    zh: '您好，欢迎来到我们新产品的展示会。',
  };

  const handleOpenOutput = async () => {
    if (outputVideoPath) {
      await invokeCommand('open_path_in_explorer', { path: outputVideoPath });
    }
  };

  return (
    <div className="space-y-6 max-w-5xl mx-auto pb-12">
      {/* Top Header */}
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4 pb-4 border-b border-slate-800">
        <div>
          <div className="flex items-center gap-2">
            <Film className="w-5 h-5 text-indigo-400" />
            <h2 className="text-xl font-bold text-slate-100">Náhľad Dabovaného Videa</h2>
            <Badge variant="success">Final MP4 (H.264 + AAC)</Badge>
          </div>
          <p className="text-xs text-slate-400 mt-1">
            Výsledné video s čínskym dabingom a synchronizovaným lip-syncom (LatentSync 1.5).
          </p>
        </div>

        <div className="flex items-center gap-2">
          <Button
            variant="secondary"
            size="sm"
            leftIcon={<Columns className="w-3.5 h-3.5" />}
            onClick={() => setViewMode((m) => (m === 'single' ? 'split' : 'single'))}
          >
            {viewMode === 'single' ? 'Porovnať s originálom' : 'Jednoduchý náhľad'}
          </Button>

          <Button
            variant="primary"
            size="sm"
            leftIcon={<FolderOpen className="w-3.5 h-3.5" />}
            onClick={handleOpenOutput}
          >
            Otvoriť v Prieskumníkovi
          </Button>
        </div>
      </div>

      {/* Video Display Area */}
      <Card className="p-0 overflow-hidden bg-black border-slate-800 shadow-2xl relative group">
        <div className={`grid ${viewMode === 'split' ? 'grid-cols-2 divide-x divide-slate-800' : 'grid-cols-1'} aspect-video bg-slate-950`}>
          {/* Split Mode: Original Video */}
          {viewMode === 'split' && (
            <div className="relative flex items-center justify-center bg-slate-950/90 overflow-hidden">
              <div className="absolute top-3 left-3 z-10">
                <Badge variant="secondary" size="sm">Originál (Slovenčina)</Badge>
              </div>
              <div className="text-center p-6 space-y-2">
                <div className="w-16 h-16 mx-auto rounded-full bg-slate-900 border border-slate-800 flex items-center justify-center text-slate-500">
                  <Film className="w-8 h-8" />
                </div>
                <p className="text-xs text-slate-400 font-mono">vstupna_prezentacia.mp4</p>
              </div>
            </div>
          )}

          {/* Main / Dubbed Video */}
          <div className="relative flex items-center justify-center bg-slate-950 overflow-hidden">
            <div className="absolute top-3 left-3 z-10">
              <Badge variant="primary" size="sm" icon={<Sparkles className="w-3 h-3 text-indigo-400" />}>
                Dabing (Čínština + LatentSync 1.5)
              </Badge>
            </div>

            {/* Simulated Animated Video Canvas */}
            <div className="w-full h-full flex items-center justify-center flex-col p-8 text-center bg-gradient-to-b from-indigo-950/20 to-slate-950">
              <div className="w-20 h-20 rounded-2xl bg-indigo-600/20 border border-indigo-500/40 flex items-center justify-center text-indigo-400 mb-3 shadow-xl">
                <Sparkles className="w-10 h-10 animate-pulse" />
              </div>
              <h4 className="font-semibold text-sm text-slate-200">
                vstupna_prezentacia_dubbed_zh.mp4
              </h4>
              <p className="text-xs text-slate-400 mt-1">
                Rozlíšenie: 1080p • 25 FPS • Piper TTS (zh_CN-huayan)
              </p>
            </div>

            {/* Subtitles Overlay */}
            {subtitlesMode !== 'none' && (
              <div className="absolute bottom-16 left-0 right-0 px-6 text-center z-10 pointer-events-none">
                <div className="inline-block bg-black/85 backdrop-blur-md px-4 py-2 rounded-xl border border-white/10 text-center shadow-lg space-y-0.5">
                  {(subtitlesMode === 'both' || subtitlesMode === 'zh') && (
                    <p className="text-sm font-semibold text-yellow-300 font-sans tracking-wide">
                      {currentSubtitle.zh}
                    </p>
                  )}
                  {(subtitlesMode === 'both' || subtitlesMode === 'sk') && (
                    <p className="text-xs text-slate-200/90 font-sans">
                      {currentSubtitle.sk}
                    </p>
                  )}
                </div>
              </div>
            )}
          </div>
        </div>

        {/* Video Player Controls Bar */}
        <div className="bg-slate-900/95 border-t border-slate-800 p-3 flex flex-col gap-2">
          {/* Progress Timeline */}
          <div className="flex items-center gap-3">
            <span className="text-[11px] font-mono text-slate-400">00:03.5</span>
            <div className="flex-1 h-1.5 bg-slate-800 rounded-full overflow-hidden cursor-pointer">
              <div className="h-full bg-indigo-500 rounded-full w-[24%]" />
            </div>
            <span className="text-[11px] font-mono text-slate-400">00:14.8</span>
          </div>

          {/* Bottom Buttons */}
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2">
              <button
                onClick={() => setIsPlaying(!isPlaying)}
                className="p-2 rounded-lg bg-indigo-600 hover:bg-indigo-500 text-white transition-all shadow-md"
              >
                {isPlaying ? <Pause className="w-4 h-4" /> : <Play className="w-4 h-4" />}
              </button>

              <button
                onClick={() => setIsMuted(!isMuted)}
                className="p-2 text-slate-400 hover:text-slate-100 rounded-lg transition-colors"
              >
                {isMuted ? <VolumeX className="w-4 h-4" /> : <Volume2 className="w-4 h-4" />}
              </button>

              <div className="flex items-center gap-1 bg-slate-800 p-0.5 rounded-lg border border-slate-700/60 ml-2">
                <button
                  onClick={() => setSubtitlesMode('both')}
                  className={`px-2 py-1 rounded text-[11px] font-medium ${
                    subtitlesMode === 'both' ? 'bg-indigo-600 text-white' : 'text-slate-400'
                  }`}
                >
                  Dvojjazyčné
                </button>
                <button
                  onClick={() => setSubtitlesMode('zh')}
                  className={`px-2 py-1 rounded text-[11px] font-medium ${
                    subtitlesMode === 'zh' ? 'bg-indigo-600 text-white' : 'text-slate-400'
                  }`}
                >
                  Čínske
                </button>
                <button
                  onClick={() => setSubtitlesMode('none')}
                  className={`px-2 py-1 rounded text-[11px] font-medium ${
                    subtitlesMode === 'none' ? 'bg-indigo-600 text-white' : 'text-slate-400'
                  }`}
                >
                  Vypnuté
                </button>
              </div>
            </div>

            <div className="flex items-center gap-2">
              <Button
                variant="secondary"
                size="sm"
                leftIcon={<Download className="w-3.5 h-3.5" />}
                onClick={handleOpenOutput}
              >
                Exportovať MP4
              </Button>
            </div>
          </div>
        </div>
      </Card>
    </div>
  );
};
