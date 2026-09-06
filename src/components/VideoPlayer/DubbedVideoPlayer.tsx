import React, { useState, useEffect, useRef } from 'react';
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
  RotateCcw,
} from 'lucide-react';
import { invokeCommand, convertVideoPathToUrl } from '../../utils/tauriBridge';
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
  const [currentTime, setCurrentTime] = useState<number>(0);
  const [duration, setDuration] = useState<number>(0);

  const [inputSrc, setInputSrc] = useState<string>('');
  const [outputSrc, setOutputSrc] = useState<string>('');

  const inputVideoRef = useRef<HTMLVideoElement>(null);
  const outputVideoRef = useRef<HTMLVideoElement>(null);

  useEffect(() => {
    let isMounted = true;
    if (inputVideoPath) {
      convertVideoPathToUrl(inputVideoPath).then((url) => {
        if (isMounted) setInputSrc(url);
      });
    } else {
      setInputSrc('');
    }

    if (outputVideoPath) {
      convertVideoPathToUrl(outputVideoPath).then((url) => {
        if (isMounted) setOutputSrc(url);
      });
    } else {
      setOutputSrc('');
    }

    return () => {
      isMounted = false;
    };
  }, [inputVideoPath, outputVideoPath]);

  const currentSubtitle = {
    sk: 'Dobrý deň, vítam vás pri prezentácii nášho nového produktu.',
    zh: '您好，欢迎来到我们新产品的展示会。',
  };

  const activeVideoSrc = outputSrc || inputSrc;
  const inputFilename = inputVideoPath ? inputVideoPath.split(/[/\\]/).pop() : 'vstupne_video.mp4';
  const outputFilename = outputVideoPath
    ? outputVideoPath.split(/[/\\]/).pop()
    : `${inputFilename?.replace(/\.[^/.]+$/, '')}_dubbed_zh.mp4`;

  const handleOpenOutput = async () => {
    const targetPath = outputVideoPath || inputVideoPath;
    if (targetPath) {
      await invokeCommand('open_path_in_explorer', { path: targetPath });
    }
  };

  const togglePlayPause = () => {
    const nextPlaying = !isPlaying;
    setIsPlaying(nextPlaying);

    if (outputVideoRef.current) {
      if (nextPlaying) outputVideoRef.current.play().catch(console.warn);
      else outputVideoRef.current.pause();
    }
    if (inputVideoRef.current) {
      if (nextPlaying) inputVideoRef.current.play().catch(console.warn);
      else inputVideoRef.current.pause();
    }
  };

  const handleSeek = (e: React.ChangeEvent<HTMLInputElement>) => {
    const target = parseFloat(e.target.value);
    setCurrentTime(target);
    if (outputVideoRef.current) outputVideoRef.current.currentTime = target;
    if (inputVideoRef.current) inputVideoRef.current.currentTime = target;
  };

  const handleTimeUpdate = (e: React.SyntheticEvent<HTMLVideoElement>) => {
    const curr = (e.target as HTMLVideoElement).currentTime;
    setCurrentTime(curr);
    const dur = (e.target as HTMLVideoElement).duration;
    if (!isNaN(dur) && dur > 0 && duration !== dur) {
      setDuration(dur);
    }
  };

  const formatTime = (secs: number) => {
    const m = Math.floor(secs / 60);
    const s = Math.floor(secs % 60);
    const ms = Math.floor((secs % 1) * 10);
    return `${m.toString().padStart(2, '0')}:${s.toString().padStart(2, '0')}.${ms}`;
  };

  return (
    <div className="space-y-6 max-w-5xl mx-auto pb-12">
      {/* Top Header */}
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4 pb-4 border-b border-slate-800">
        <div>
          <div className="flex items-center gap-2">
            <Film className="w-5 h-5 text-indigo-400" />
            <h2 className="text-xl font-bold text-slate-100">Náhľad Dabovaného Videa</h2>
            <Badge variant={outputVideoPath ? 'success' : 'primary'}>
              {outputVideoPath ? 'Finálne video (H.264 + AAC)' : 'Zdrojové video pripravené'}
            </Badge>
          </div>
          <p className="text-xs text-slate-400 mt-1">
            {outputVideoPath
              ? 'Výsledné video s čínskym dabingom a synchronizovaným lip-syncom (LatentSync 1.5).'
              : 'Náhľad načítaného slovenského videa pripraveného na spracovanie.'}
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
        <div
          className={`grid ${
            viewMode === 'split' ? 'grid-cols-2 divide-x divide-slate-800' : 'grid-cols-1'
          } aspect-video bg-slate-950 relative`}
        >
          {/* Split Mode: Original Video */}
          {viewMode === 'split' && (
            <div className="relative flex items-center justify-center bg-slate-950 overflow-hidden">
              <div className="absolute top-3 left-3 z-10">
                <Badge variant="secondary" size="sm">
                  Originál (Slovenčina)
                </Badge>
              </div>

              {inputSrc ? (
                <video
                  ref={inputVideoRef}
                  src={inputSrc}
                  muted={true}
                  playsInline
                  className="w-full h-full object-contain bg-black"
                  onTimeUpdate={handleTimeUpdate}
                  onEnded={() => setIsPlaying(false)}
                />
              ) : (
                <div className="text-center p-6 space-y-2">
                  <div className="w-16 h-16 mx-auto rounded-full bg-slate-900 border border-slate-800 flex items-center justify-center text-slate-500">
                    <Film className="w-8 h-8" />
                  </div>
                  <p className="text-xs text-slate-400 font-mono">{inputFilename}</p>
                </div>
              )}
            </div>
          )}

          {/* Main / Dubbed Video */}
          <div className="relative flex items-center justify-center bg-slate-950 overflow-hidden">
            <div className="absolute top-3 left-3 z-10">
              <Badge
                variant="primary"
                size="sm"
                icon={<Sparkles className="w-3 h-3 text-indigo-400" />}
              >
                {outputVideoPath ? 'Dabing (Čínština + LatentSync 1.5)' : 'Náhľad Videa'}
              </Badge>
            </div>

            {activeVideoSrc ? (
              <video
                ref={outputVideoRef}
                src={activeVideoSrc}
                muted={isMuted}
                playsInline
                className="w-full h-full object-contain bg-black"
                onTimeUpdate={handleTimeUpdate}
                onLoadedMetadata={(e) => {
                  const d = (e.target as HTMLVideoElement).duration;
                  if (!isNaN(d)) setDuration(d);
                }}
                onEnded={() => setIsPlaying(false)}
              />
            ) : (
              <div className="w-full h-full flex items-center justify-center flex-col p-8 text-center bg-gradient-to-b from-indigo-950/20 to-slate-950">
                <div className="w-20 h-20 rounded-2xl bg-indigo-600/20 border border-indigo-500/40 flex items-center justify-center text-indigo-400 mb-3 shadow-xl">
                  <Sparkles className="w-10 h-10 animate-pulse" />
                </div>
                <h4 className="font-semibold text-sm text-slate-200">{outputFilename}</h4>
                <p className="text-xs text-slate-400 mt-1">
                  Rozlíšenie: 1080p • 25 FPS • Piper TTS (zh_CN-huayan)
                </p>
              </div>
            )}

            {/* Subtitles Overlay */}
            {subtitlesMode !== 'none' && (
              <div className="absolute bottom-6 left-0 right-0 px-6 text-center z-10 pointer-events-none">
                <div className="inline-block bg-black/85 backdrop-blur-md px-4 py-2 rounded-xl border border-white/10 text-center shadow-lg space-y-0.5">
                  {(subtitlesMode === 'both' || subtitlesMode === 'zh') && (
                    <p className="text-sm font-semibold text-yellow-300 font-sans tracking-wide">
                      {currentSubtitle.zh}
                    </p>
                  )}
                  {(subtitlesMode === 'both' || subtitlesMode === 'sk') && (
                    <p className="text-xs text-slate-200/90 font-sans">{currentSubtitle.sk}</p>
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
            <span className="text-[11px] font-mono text-slate-400">{formatTime(currentTime)}</span>
            <input
              type="range"
              min={0}
              max={duration || 100}
              step={0.1}
              value={currentTime}
              onChange={handleSeek}
              className="flex-1 h-1.5 bg-slate-800 accent-indigo-500 rounded-full cursor-pointer"
            />
            <span className="text-[11px] font-mono text-slate-400">
              {formatTime(duration || 0)}
            </span>
          </div>

          {/* Bottom Buttons */}
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2">
              <button
                onClick={togglePlayPause}
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
