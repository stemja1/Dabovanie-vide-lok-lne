import React, { useRef, useState } from 'react';
import { UploadCloud, Film, CheckCircle, FileVideo, RefreshCw } from 'lucide-react';
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
      // In web fallback, use file name; in Tauri, full path is available
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

  const filename = currentVideoPath ? currentVideoPath.split(/[/\\]/).pop() : null;

  return (
    <div
      onDragOver={handleDragOver}
      onDragLeave={handleDragLeave}
      onDrop={handleDrop}
      className={`relative border-2 border-dashed rounded-2xl p-6 transition-all duration-200 text-center select-none ${
        isDragging
          ? 'border-indigo-500 bg-indigo-500/10'
          : currentVideoPath
          ? 'border-slate-700 bg-slate-900/60'
          : 'border-slate-800 hover:border-slate-700 bg-slate-900/30'
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
            <div className="p-3 rounded-xl bg-indigo-600/15 text-indigo-400 border border-indigo-500/30">
              <FileVideo className="w-6 h-6" />
            </div>
            <div>
              <div className="flex items-center gap-2">
                <span className="font-semibold text-sm text-slate-100">{filename}</span>
                <span className="text-emerald-400 flex items-center text-xs gap-1 font-medium">
                  <CheckCircle className="w-3.5 h-3.5" /> Pripravené na dabing
                </span>
              </div>
              <p className="text-xs font-mono text-slate-500 mt-0.5 truncate max-w-md">
                {currentVideoPath}
              </p>
            </div>
          </div>

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
      ) : (
        <div className="flex flex-col items-center justify-center space-y-2 py-4">
          <div className="p-3 rounded-2xl bg-slate-800/80 text-indigo-400 mb-1">
            <UploadCloud className="w-8 h-8" />
          </div>
          <h4 className="font-semibold text-sm text-slate-200">
            Presuňte slovenské video sem alebo kliknite pre výber
          </h4>
          <p className="text-xs text-slate-400 max-w-sm">
            Podporované formáty: MP4, MKV, MOV. Aplikácia automaticky extrahuje zvuk, vykoná ASR prepis, preklad do čínštiny a lip-sync.
          </p>
        </div>
      )}
    </div>
  );
};
