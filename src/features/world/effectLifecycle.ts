import type { Effect, EffectLifecycle } from '../../domain/world';

/** Derive display lifecycle from recorded facts only; never write state. */
export function effectLifecycleAt(effect: Effect, now: number = Date.now()): EffectLifecycle {
  const started = Date.parse(effect.startedAt);
  if (Number.isFinite(started) && started > now) return 'scheduled';
  const deactivatedAt = effect.deactivatedAt ? Date.parse(effect.deactivatedAt) : Number.NaN;
  const expired = effect.expiresAt ? Date.parse(effect.expiresAt) : Number.NaN;
  const isDeactivated = Number.isFinite(deactivatedAt) && deactivatedAt <= now;
  const isExpired = Number.isFinite(expired) && expired <= now;
  const deactivationState = effect.deactivationSource === 'rule' ? 'rule_deactivated' : 'manually_deactivated';
  if (isDeactivated && isExpired) return deactivatedAt < expired ? deactivationState : 'expired';
  if (isDeactivated) return deactivationState;
  if (isExpired) return 'expired';
  return 'active';
}
