import React, { useRef, useState } from 'react';
import { UploadCloud, Film, CheckCircle, FileVideo, RefreshCw, Sparkles, Video } from 'lucide-react';
import { Button } from '../ui/Button';

interface VideoDropzoneProps {
  currentVideoPath: string;
  onVideoSelected: (path: string) => void;
  disabled?: boolean;
}

export const VideoDropzone: React.FC<VideoDropzoneProps> = ({
  currentVideoPath,
  onVideoSelected,
  disabled = false,
}) => {
  const [isDragging, setIsDragging] = useState<boolean>(false);
  const fileInputRef = useRef<HTMLInputElement>(null);

  const handleDragOver = (e: React.DragEvent) => {
    e.preventDefault();
    if (!disabled) setIsDragging(true);
  };

  const handleDragLeave = () => {
    setIsDragging(false);
  };

  const handleDrop = (e: React.DragEvent) => {
    e.preventDefault();
    setIsDragging(false);
    if (disabled) return;

    if (e.dataTransfer.files && e.dataTransfer.files.length > 0) {
      const file = e.dataTransfer.files[0];
      const path = (file as any).path || `C:\\AI_Dubbing\\Videos\\${file.name}`;
      onVideoSelected(path);
    }
  };

  const handleFileChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    if (e.target.files && e.target.files.length > 0) {
      const file = e.target.files[0];
      const path = (file as any).path || `C:\\AI_Dubbing\\Videos\\${file.name}`;
      onVideoSelected(path);
    }
  };

  const handleLoadDemoVideo = (e: React.MouseEvent) => {
    e.stopPropagation();
    onVideoSelected('C:\\AI_Dubbing\\Videos\\slovenska_prezentacia_sample.mp4');
  };

  const filename = currentVideoPath ? currentVideoPath.split(/[/\\]/).pop() : null;

  return (
    <div
      onDragOver={handleDragOver}
      onDragLeave={handleDragLeave}
      onDrop={handleDrop}
      className={`relative border-2 border-dashed rounded-2xl p-6 transition-all duration-200 text-center select-none ${
        isDragging
          ? 'border-indigo-500 bg-indigo-500/15 shadow-lg shadow-indigo-500/10 scale-[1.005]'
          : currentVideoPath
          ? 'border-indigo-500/40 bg-slate-900/80 shadow-md shadow-indigo-950/20'
          : 'border-slate-800 hover:border-slate-700 bg-slate-900/40 hover:bg-slate-900/60'
      } ${disabled ? 'opacity-60 cursor-not-allowed' : 'cursor-pointer'}`}
      onClick={() => !disabled && fileInputRef.current?.click()}
    >
      <input
        type="file"
        ref={fileInputRef}
        onChange={handleFileChange}
        accept="video/mp4,video/mkv,video/quicktime,video/webm"
        className="hidden"
        disabled={disabled}
      />

      {filename ? (
        <div className="flex flex-col sm:flex-row items-center justify-between gap-4">
          <div className="flex items-center gap-3 text-left">
            <div className="p-3.5 rounded-xl bg-indigo-600/20 text-indigo-400 border border-indigo-500/30">
              <FileVideo className="w-6 h-6" />
            </div>
            <div>
              <div className="flex items-center gap-2 flex-wrap">
                <span className="font-semibold text-sm text-slate-100">{filename}</span>
                <span className="text-emerald-400 flex items-center text-xs gap-1 font-medium bg-emerald-500/10 px-2 py-0.5 rounded-full border border-emerald-500/20">
                  <CheckCircle className="w-3.5 h-3.5" /> Pripravené na dabing
                </span>
              </div>
              <p className="text-xs font-mono text-slate-400 mt-1 truncate max-w-md">
                {currentVideoPath}
              </p>
            </div>
          </div>

          <div className="flex items-center gap-2">
            <Button
              variant="secondary"
              size="sm"
              leftIcon={<RefreshCw className="w-3.5 h-3.5 text-slate-400" />}
              onClick={(e) => {
                e.stopPropagation();
                fileInputRef.current?.click();
              }}
              disabled={disabled}
            >
              Zmeniť video
            </Button>
          </div>
        </div>
      ) : (
        <div className="flex flex-col items-center justify-center space-y-3 py-4">
          <div className="p-3.5 rounded-2xl bg-indigo-500/10 text-indigo-400 border border-indigo-500/20 mb-1">
            <UploadCloud className="w-8 h-8" />
          </div>
          <h4 className="font-semibold text-sm text-slate-200">
            Presuňte slovenské video sem alebo kliknite pre výber
          </h4>
          <p className="text-xs text-slate-400 max-w-md leading-relaxed">
            Podporované formáty: <strong>MP4, MKV, MOV, WEBM</strong>. Aplikácia automaticky extrahuje zvuk, vykoná ASR prepis, preklad do čínštiny a lip-sync s akceleráciou AMD Radeon.
          </p>

          <div className="pt-2">
            <Button
              variant="secondary"
              size="sm"
              leftIcon={<Sparkles className="w-3.5 h-3.5 text-indigo-400" />}
              onClick={handleLoadDemoVideo}
              disabled={disabled}
              className="text-xs border-indigo-500/30 hover:border-indigo-500/50 bg-indigo-950/30"
            >
              Vložiť ukážkové testovacie video
            </Button>
          </div>
        </div>
      )}
    </div>
  );
};

