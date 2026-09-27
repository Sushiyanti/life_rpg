import type { Effect, EffectLifecycle } from '../../domain/world';

/** Derive display lifecycle from recorded facts only; never write state. */
export function effectLifecycleAt(effect: Effect, now: number = Date.now()): EffectLifecycle {
  const started = Date.parse(effect.startedAt);
  if (Number.isFinite(started) && started > now) return 'scheduled';
  const manuallyDeactivated = effect.deactivatedAt ? Date.parse(effect.deactivatedAt) : Number.NaN;
  const expired = effect.expiresAt ? Date.parse(effect.expiresAt) : Number.NaN;
  const isDeactivated = Number.isFinite(manuallyDeactivated) && manuallyDeactivated <= now;
  const isExpired = Number.isFinite(expired) && expired <= now;
  if (isDeactivated && isExpired) return manuallyDeactivated < expired ? 'manually_deactivated' : 'expired';
  if (isDeactivated) return 'manually_deactivated';
  if (isExpired) return 'expired';
  return 'active';
}
