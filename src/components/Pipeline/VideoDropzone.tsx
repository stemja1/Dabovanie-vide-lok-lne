import React, { useRef, useState, useEffect } from 'react';
import {
  UploadCloud,
  Film,
  CheckCircle2,
  FileVideo,
  RefreshCw,
  Sparkles,
  Play,
  Pause,
  FolderOpen,
  Eye,
  AlertCircle,
  XCircle,
} from 'lucide-react';
import { Button } from '../ui/Button';
import { Badge } from '../ui/Badge';
import {
  isTauriEnvironment,
  isVideoFile,
  convertVideoPathToUrl,
  pickVideoFileDialog,
  addTauriListener,
  invokeCommand,
} from '../../utils/tauriBridge';

interface VideoDropzoneProps {
  currentVideoPath: string;
  onVideoSelected: (path: string, previewBlobUrl?: string) => void;
  disabled?: boolean;
}

export const VideoDropzone: React.FC<VideoDropzoneProps> = ({
  currentVideoPath,
  onVideoSelected,
  disabled = false,
}) => {
  const [isDragging, setIsDragging] = useState<boolean>(false);
  const [videoPreviewUrl, setVideoPreviewUrl] = useState<string>('');
  const [isPlayingPreview, setIsPlayingPreview] = useState<boolean>(false);
  const [videoDuration, setVideoDuration] = useState<number | null>(null);
  const fileInputRef = useRef<HTMLInputElement>(null);
  const videoRef = useRef<HTMLVideoElement>(null);

  // Convert current video path to streamable URL for <video> preview
  useEffect(() => {
    let isMounted = true;
    if (currentVideoPath) {
      convertVideoPathToUrl(currentVideoPath).then((url) => {
        if (isMounted) {
          setVideoPreviewUrl(url);
        }
      });
    } else {
      setVideoPreviewUrl('');
      setVideoDuration(null);
    }
    return () => {
      isMounted = false;
    };
  }, [currentVideoPath]);

  // Native Tauri drag-drop window listener
  useEffect(() => {
    let unlistenNative: (() => void) | null = null;
    let isCancelled = false;

    const setupTauriDropListener = async () => {
      if (!isTauriEnvironment()) return;

      try {
        const { getCurrentWebviewWindow } = await import('@tauri-apps/api/webviewWindow');
        const win = getCurrentWebviewWindow();
        const unlisten = await win.onDragDropEvent((event) => {
          if (disabled || isCancelled) return;

          const payload = event.payload;
          if (payload.type === 'enter' || payload.type === 'over') {
            setIsDragging(true);
          } else if (payload.type === 'leave') {
            setIsDragging(false);
          } else if (payload.type === 'drop') {
            setIsDragging(false);
            const paths = payload.paths;
            if (paths && paths.length > 0) {
              const droppedPath = paths[0];
              if (isVideoFile(droppedPath)) {
                onVideoSelected(droppedPath);
              }
            }
          }
        });
        unlistenNative = unlisten;
      } catch (err) {
        // Fallback to tauri://drag-drop event
        unlistenNative = addTauriListener('tauri://drag-drop', (payload: any) => {
          if (disabled || isCancelled) return;
          setIsDragging(false);
          const paths = payload?.paths || (Array.isArray(payload) ? payload : [payload]);
          if (paths && paths.length > 0) {
            const p = typeof paths[0] === 'string' ? paths[0] : paths[0]?.path;
            if (p && isVideoFile(p)) {
              onVideoSelected(p);
            }
          }
        });
      }
    };

    setupTauriDropListener();

    return () => {
      isCancelled = true;
      if (unlistenNative) unlistenNative();
    };
  }, [disabled, onVideoSelected]);

  // Native Windows Open File Dialog or fallback to HTML file input
  const handleOpenFileDialog = async (e?: React.MouseEvent) => {
    if (e) e.stopPropagation();
    if (disabled) return;

    if (isTauriEnvironment()) {
      try {
        const selected = await pickVideoFileDialog();
        if (selected) {
          onVideoSelected(selected);
          return;
        }
      } catch (err) {
        console.warn('Native picker error, fallback to HTML file input', err);
      }
    }
    fileInputRef.current?.click();
  };

  // HTML5 Drag and Drop events (for browser or Webview fallback)
  const handleDragOver = (e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    if (!disabled) setIsDragging(true);
  };

  const handleDragLeave = (e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    setIsDragging(false);
  };

  const handleDrop = (e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    setIsDragging(false);
    if (disabled) return;

    if (e.dataTransfer.files && e.dataTransfer.files.length > 0) {
      const file = e.dataTransfer.files[0];
      const nativePath = (file as any).path;
      const path = nativePath || file.name;
      const blobUrl = URL.createObjectURL(file);
      setVideoPreviewUrl(blobUrl);
      onVideoSelected(path, blobUrl);
    }
  };

  const handleHTMLFileChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    if (e.target.files && e.target.files.length > 0) {
      const file = e.target.files[0];
      const nativePath = (file as any).path;
      const path = nativePath || file.name;
      const blobUrl = URL.createObjectURL(file);
      setVideoPreviewUrl(blobUrl);
      onVideoSelected(path, blobUrl);
    }
  };

  const handleLoadDemoVideo = (e: React.MouseEvent) => {
    e.stopPropagation();
    onVideoSelected('C:\\AI_Dubbing\\Videos\\slovenska_prezentacia_sample.mp4');
  };

  const handleTogglePreviewPlay = (e: React.MouseEvent) => {
    e.stopPropagation();
    if (!videoRef.current) return;
    if (isPlayingPreview) {
      videoRef.current.pause();
      setIsPlayingPreview(false);
    } else {
      videoRef.current.play().then(() => {
        setIsPlayingPreview(true);
      }).catch((err) => {
        console.warn('Video play error', err);
      });
    }
  };

  const handleOpenInExplorer = async (e: React.MouseEvent) => {
    e.stopPropagation();
    if (currentVideoPath) {
      await invokeCommand('open_path_in_explorer', { path: currentVideoPath });
    }
  };

  const filename = currentVideoPath ? currentVideoPath.split(/[/\\]/).pop() : null;

  return (
    <div
      onDragOver={handleDragOver}
      onDragLeave={handleDragLeave}
      onDrop={handleDrop}
      className={`relative rounded-2xl transition-all duration-200 text-center select-none overflow-hidden ${
        isDragging
          ? 'border-2 border-dashed border-indigo-400 bg-indigo-500/20 shadow-xl shadow-indigo-500/20 scale-[1.008]'
          : currentVideoPath
          ? 'border border-slate-700/80 bg-slate-900/90 shadow-lg shadow-indigo-950/20'
          : 'border-2 border-dashed border-slate-800 hover:border-slate-700 bg-slate-900/40 hover:bg-slate-900/60'
      } ${disabled ? 'opacity-60 cursor-not-allowed' : 'cursor-pointer'}`}
      onClick={!currentVideoPath ? () => handleOpenFileDialog() : undefined}
    >
      <input
        type="file"
        ref={fileInputRef}
        onChange={handleHTMLFileChange}
        accept="video/mp4,video/mkv,video/quicktime,video/webm,video/avi"
        className="hidden"
        disabled={disabled}
      />

      {filename ? (
        <div className="p-5">
          <div className="flex flex-col md:flex-row items-stretch md:items-center gap-5">
            {/* Live Video Preview Thumbnail / Player */}
            <div
              className="relative w-full md:w-56 h-36 rounded-xl overflow-hidden bg-black border border-slate-800 flex-shrink-0 group flex items-center justify-center cursor-pointer shadow-inner"
              onClick={handleTogglePreviewPlay}
            >
              {videoPreviewUrl ? (
                <video
                  ref={videoRef}
                  src={videoPreviewUrl}
                  className="w-full h-full object-contain bg-black"
                  onLoadedMetadata={(e) => {
                    const dur = (e.target as HTMLVideoElement).duration;
                    if (!isNaN(dur)) setVideoDuration(dur);
                  }}
                  onEnded={() => setIsPlayingPreview(false)}
                  onPause={() => setIsPlayingPreview(false)}
                  onPlay={() => setIsPlayingPreview(true)}
                  playsInline
                />
              ) : (
                <div className="flex flex-col items-center justify-center text-slate-500 gap-1">
                  <Film className="w-8 h-8" />
                  <span className="text-[10px] font-mono">Náhľad videa</span>
                </div>
              )}

              {/* Play / Pause overlay badge */}
              <div
                className={`absolute inset-0 bg-black/40 flex items-center justify-center transition-opacity ${
                  isPlayingPreview ? 'opacity-0 hover:opacity-100' : 'opacity-100'
                }`}
              >
                <div className="w-10 h-10 rounded-full bg-indigo-600/90 text-white flex items-center justify-center shadow-lg transform group-hover:scale-110 transition-transform">
                  {isPlayingPreview ? <Pause className="w-5 h-5" /> : <Play className="w-5 h-5 ml-0.5" />}
                </div>
              </div>

              {videoDuration && (
                <span className="absolute bottom-1.5 right-1.5 px-1.5 py-0.5 bg-black/80 rounded text-[10px] font-mono text-slate-300">
                  {Math.floor(videoDuration / 60)}:
                  {Math.floor(videoDuration % 60)
                    .toString()
                    .padStart(2, '0')}
                </span>
              )}
            </div>

            {/* Video File Information */}
            <div className="flex-1 text-left flex flex-col justify-between py-1">
              <div>
                <div className="flex items-center gap-2 flex-wrap mb-1.5">
                  <h3 className="font-bold text-base text-slate-100 tracking-tight">{filename}</h3>
                  <span className="text-emerald-400 flex items-center text-xs gap-1 font-semibold bg-emerald-500/10 px-2.5 py-0.5 rounded-full border border-emerald-500/20">
                    <CheckCircle2 className="w-3.5 h-3.5" /> Pripravené na dabing
                  </span>
                </div>

                <p className="text-xs font-mono text-slate-400 break-all bg-slate-950/60 p-2 rounded-lg border border-slate-800/80 mb-2">
                  {currentVideoPath}
                </p>

                <div className="flex items-center gap-4 text-xs text-slate-400">
                  <span>
                    Formát: <strong className="text-slate-200">{filename.split('.').pop()?.toUpperCase()}</strong>
                  </span>
                  {videoDuration && (
                    <span>
                      Dĺžka: <strong className="text-slate-200">{videoDuration.toFixed(1)} s</strong>
                    </span>
                  )}
                  <span>
                    Režim: <strong className="text-indigo-400">Whisper-SK $\rightarrow$ NLLB-200 $\rightarrow$ Piper</strong>
                  </span>
                </div>
              </div>

              {/* Action Buttons */}
              <div className="flex items-center gap-2.5 mt-4 pt-3 border-t border-slate-800/80">
                <Button
                  variant="secondary"
                  size="sm"
                  leftIcon={<RefreshCw className="w-3.5 h-3.5 text-slate-400" />}
                  onClick={(e) => handleOpenFileDialog(e)}
                  disabled={disabled}
                >
                  Vybrať iné video
                </Button>

                <Button
                  variant="outline"
                  size="sm"
                  leftIcon={<FolderOpen className="w-3.5 h-3.5 text-slate-400" />}
                  onClick={handleOpenInExplorer}
                >
                  Priečinok videa
                </Button>

                <Button
                  variant="outline"
                  size="sm"
                  leftIcon={<Eye className="w-3.5 h-3.5 text-indigo-400" />}
                  onClick={handleTogglePreviewPlay}
                >
                  {isPlayingPreview ? 'Zastaviť náhľad' : 'Prehrať náhľad'}
                </Button>
              </div>
            </div>
          </div>
        </div>
      ) : (
        <div className="flex flex-col items-center justify-center space-y-3 py-8 px-4">
          <div className="p-4 rounded-2xl bg-indigo-500/10 text-indigo-400 border border-indigo-500/20 mb-1 animate-pulse">
            <UploadCloud className="w-10 h-10" />
          </div>
          <h4 className="font-semibold text-base text-slate-200">
            Presuňte slovenské video sem alebo kliknite pre výber
          </h4>
          <p className="text-xs text-slate-400 max-w-md leading-relaxed">
            Podporované formáty: <strong>MP4, MKV, MOV, WEBM, AVI</strong>. Aplikácia automaticky
            extrahuje zvuk, vykoná ASR prepis, preklad do čínštiny a lip-sync s akceleráciou AMD
            Radeon RX 7700 XT.
          </p>

          <div className="pt-2 flex items-center gap-3">
            <Button
              variant="primary"
              size="sm"
              leftIcon={<FileVideo className="w-3.5 h-3.5" />}
              onClick={(e) => handleOpenFileDialog(e)}
              disabled={disabled}
            >
              Prehľadávať počítač
            </Button>

            <Button
              variant="secondary"
              size="sm"
              leftIcon={<Sparkles className="w-3.5 h-3.5 text-indigo-400" />}
              onClick={handleLoadDemoVideo}
              disabled={disabled}
              className="text-xs border-indigo-500/30 hover:border-indigo-500/50 bg-indigo-950/30"
            >
              Vložiť ukážkové video
            </Button>
          </div>
        </div>
      )}
    </div>
  );
};
