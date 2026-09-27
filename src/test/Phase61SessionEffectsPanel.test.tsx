import { afterEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import { CoreClient } from '../domain/ipc';
import type { Concept, Effect, EffectType, Player, SessionEffect } from '../domain/world';
import { SessionEffectsPanel } from '../features/world/SessionEffectsPanel';

afterEach(cleanup);
const player:Player={id:'p1',name:'Test Player',description:null,level:1,levelName:null,progressionLabel:null,currentXp:0,isActive:true,metadataJson:'{}',createdAt:'2026-01-01T00:00:00Z',updatedAt:'2026-01-01T00:00:00Z'};
const effect:Effect={id:'e1',playerId:'p1',targetKind:'player',targetConceptId:null,typeCode:'buff',name:'Focus',description:'Recorded state',startedAt:'2026-01-01T00:00:00Z',expiresAt:'2026-01-02T00:00:00Z',deactivatedAt:null,intensity:1};
const link:SessionEffect={id:'se1',playerId:'p1',sessionId:'s1',effectId:'e1',role:'relevant',addedAt:'2026-01-01T00:00:00Z',removedAt:null};
const types:EffectType[]=[{code:'buff',label:'Buff',description:null,sortOrder:0}];const concepts:Concept[]=[];

describe('Phase 6.1 Session–Effect context panel',()=>{
  it('removes only the explicit relationship, not the Effect record',async()=>{
    const client=new CoreClient(async()=>{throw new Error('unexpected command')});
    const list=vi.spyOn(client,'listSessionEffects').mockResolvedValueOnce([link]).mockResolvedValueOnce([]);
    vi.spyOn(client,'listEffectTypes').mockResolvedValue(types);
    const unlink=vi.spyOn(client,'unlinkEffectFromSession').mockResolvedValue({...link,removedAt:'2026-01-02T00:00:00Z'});
    const deactivate=vi.spyOn(client,'deactivateEffect').mockResolvedValue(effect);
    const refresh=vi.fn(async()=>{});const open=vi.fn();
    render(<SessionEffectsPanel client={client} player={player} sessionId="s1" effects={[effect]} concepts={concepts} onOpenEffect={open} onRefresh={refresh}/>);
    expect(await screen.findByText('1 links')).toBeInTheDocument();
    expect(screen.getByText('buff · expiry recorded')).toBeInTheDocument();
    fireEvent.click(await screen.findByRole('button',{name:'Focus'}));expect(open).toHaveBeenCalledWith('e1');
    fireEvent.click(screen.getByRole('button',{name:'Remove link'}));
    expect(await screen.findByText('No Effects have been explicitly linked to this Session.')).toBeInTheDocument();
    expect(unlink).toHaveBeenCalledWith('p1','s1','se1');expect(list).toHaveBeenCalledTimes(2);expect(refresh).toHaveBeenCalled();expect(deactivate).not.toHaveBeenCalled();
    expect(screen.getByText('Session link removed; both records remain unchanged.')).toBeInTheDocument();
  });
});
