import { describe, expect, it } from 'vitest';
import type { Effect } from '../domain/world';
import { effectLifecycleAt } from '../features/world/effectLifecycle';

const base:Effect={id:'e1',playerId:'p1',targetKind:'player',targetConceptId:null,typeCode:'buff',name:'Focus',description:null,startedAt:'2026-01-01T00:00:00Z',expiresAt:null,deactivatedAt:null,deactivationSource:null,intensity:1};

describe('Phase 6.1 Effect lifecycle presentation',()=>{
  const now=Date.parse('2026-01-03T00:00:00Z');
  it('derives scheduled before the stored start without changing the record',()=>{
    const effect={...base,startedAt:'2026-01-04T00:00:00Z'};
    expect(effectLifecycleAt(effect,now)).toBe('scheduled');
    expect(effect.deactivatedAt).toBeNull();
  });
  it('keeps no-expiry Effects active until a manual deactivation is recorded',()=>{
    expect(effectLifecycleAt(base,now)).toBe('active');
    expect(effectLifecycleAt({...base,deactivatedAt:'2026-01-02T00:00:00Z'},now)).toBe('manually_deactivated');
  });
  it('derives expiry at the exact expiry timestamp without mutating deactivatedAt',()=>{
    const effect={...base,expiresAt:'2026-01-03T00:00:00Z'};
    expect(effectLifecycleAt(effect,now)).toBe('expired');
    expect(effect.deactivatedAt).toBeNull();
  });
  it('distinguishes whichever lifecycle fact occurred first',()=>{
    expect(effectLifecycleAt({...base,expiresAt:'2026-01-04T00:00:00Z',deactivatedAt:'2026-01-02T00:00:00Z'},now)).toBe('manually_deactivated');
    expect(effectLifecycleAt({...base,expiresAt:'2026-01-02T00:00:00Z',deactivatedAt:'2026-01-03T00:00:00Z'},now)).toBe('expired');
  });
});
