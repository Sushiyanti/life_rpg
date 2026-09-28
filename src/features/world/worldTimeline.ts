/** Formats only an explicitly persisted Session start/end pair. */
export function formatSessionDuration(startedAt: string, endedAt: string | null): string | null {
  if (!endedAt) return null;
  const start = Date.parse(startedAt);
  const end = Date.parse(endedAt);
  if (!Number.isFinite(start) || !Number.isFinite(end) || end < start) return null;
  const totalMinutes = Math.round((end - start) / 60_000);
  if (totalMinutes < 1) return 'under 1 min';
  if (totalMinutes < 60) return `${totalMinutes} min`;
  const hours = Math.floor(totalMinutes / 60);
  const minutes = totalMinutes % 60;
  return minutes ? `${hours} hr ${minutes} min` : `${hours} hr`;
}
