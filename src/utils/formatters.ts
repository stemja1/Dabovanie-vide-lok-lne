export function formatBytes(megabytes: number): string {
  if (megabytes >= 1024) {
    return `${(megabytes / 1024).toFixed(1)} GB`;
  }
  return `${megabytes} MB`;
}

export function formatTimeSeconds(seconds: number): string {
  const mins = Math.floor(seconds / 60);
  const secs = Math.floor(seconds % 60);
  const millis = Math.floor((seconds % 1) * 100);
  return `${mins.toString().padStart(2, '0')}:${secs.toString().padStart(2, '0')}.${millis.toString().padStart(2, '0')}`;
}

/**
 * Formats a timestamp as an SRT cue time (`HH:MM:SS,mmm`).
 *
 * Rounds the whole value to milliseconds before splitting it into fields.
 * Rounding each field independently produced out-of-range values such as
 * `00:00:03,1000`, which is not a valid SRT timestamp.
 */
export function formatSrtTimestamp(seconds: number): string {
  const totalMs = Math.max(0, Math.round((Number.isFinite(seconds) ? seconds : 0) * 1000));
  const hrs = Math.floor(totalMs / 3_600_000);
  const mins = Math.floor((totalMs % 3_600_000) / 60_000);
  const secs = Math.floor((totalMs % 60_000) / 1000);
  const millis = totalMs % 1000;
  return `${String(hrs).padStart(2, '0')}:${String(mins).padStart(2, '0')}:${String(secs).padStart(
    2,
    '0'
  )},${String(millis).padStart(3, '0')}`;
}

export function formatDurationMs(startMs: number | null, endMs: number | null): string {
  if (!startMs) return '--:--';
  const finish = endMs || Date.now();
  const diffSec = Math.max(0, Math.floor((finish - startMs) / 1000));
  const m = Math.floor(diffSec / 60);
  const s = diffSec % 60;
  return `${m}m ${s}s`;
}
