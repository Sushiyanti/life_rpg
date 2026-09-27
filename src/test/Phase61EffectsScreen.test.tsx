import { afterEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { CoreClient } from '../domain/ipc';
import type { Concept, Effect, EffectHistoryEntry, EffectType, Player, QuestSession } from '../domain/world';
import { EffectsScreen } from '../features/world/EffectsScreen';

afterEach(cleanup);
const player:Player={id:'p1',name:'Test Player',description:null,level:1,levelName:null,progressionLabel:null,currentXp:0,isActive:true,metadataJson:'{}',createdAt:'2026-01-01T00:00:00Z',updatedAt:'2026-01-01T00:00:00Z'};
const types:EffectType[]=[{code:'buff',label:'Buff',description:'An authored state',sortOrder:0}];
const effect:Effect={id:'e1',playerId:'p1',targetKind:'player',targetConceptId:null,typeCode:'buff',name:'Focus',description:'A recorded state',startedAt:'2026-01-01T00:00:00Z',expiresAt:null,deactivatedAt:null,intensity:2};
const concepts:Concept[]=[];const sessions:QuestSession[]=[];
function setup(effects:Effect[]=[]){const client=new CoreClient(async()=>{throw new Error('unexpected command')});const typesSpy=vi.spyOn(client,'listEffectTypes').mockResolvedValue(types);const create=vi.spyOn(client,'createEffect').mockResolvedValue(effect);const history:EffectHistoryEntry[]=[{id:'h1',playerId:'p1',effectId:'e1',sessionId:null,eventKind:'created',recordedAt:'2026-01-01T00:00:00Z',previousStateJson:null,currentStateJson:'{"name":"Focus"}'}];const listHistory=vi.spyOn(client,'listEffectHistory').mockResolvedValue(history);const refresh=vi.fn(async()=>{});render(<EffectsScreen client={client} player={player} effects={effects} concepts={concepts} sessions={sessions} isVisible={()=>true} onVisibility={async()=>{}} showHidden={false} onShowHidden={()=>{}} onRefresh={refresh}/>);return{client,typesSpy,create,listHistory,refresh}}

describe('Phase 6.1 Effects manager',()=>{
  it('creates an indefinite Effect without inventing an expiry or Concept target',async()=>{
    const {create}=setup();await screen.findByRole('option',{name:/Buff/});expect(screen.getByText(/Never — no expiry recorded/i)).toBeInTheDocument();fireEvent.change(screen.getByLabelText('Name'),{target:{value:'Focus before a talk'}});fireEvent.click(screen.getByRole('button',{name:'Create Effect'}));await waitFor(()=>expect(create).toHaveBeenCalledWith('p1',expect.objectContaining({name:'Focus before a talk',expiresAt:null,targetConceptId:null,intensity:1}),undefined));
  });
  it('loads and shows append-only Effect history without mutating the Effect',async()=>{
    const {listHistory}=setup([effect]);fireEvent.click(await screen.findByRole('button',{name:'View history'}));expect(await screen.findByText('Effect history · 1 recorded event')).toBeInTheDocument();expect(await screen.findByText('created')).toBeInTheDocument();expect(listHistory).toHaveBeenCalledWith('p1','e1');
  });
});
