import { afterEach, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { EntitySurfaceProvider, useEntitySurface } from '../features/entity-surface/EntitySurface';
import type { CoreClient } from '../domain/ipc';
import type { Concept, ConceptAssociation, ConceptRelationship, Effect, Player, Quest, WorldOverview } from '../domain/world';

afterEach(() => { document.body.style.overflow = ''; });
const player:Player={id:'player-1',name:'Rin',description:'A test world',level:3,levelName:'Wayfinder',progressionLabel:'Mapping the unknown',currentXp:920,isActive:true,metadataJson:'{}',createdAt:'',updatedAt:''};
const questA:Quest={id:'quest-a',playerId:player.id,typeCode:'main',parentQuestId:null,skillId:null,title:'Prepare the garden',status:'in_progress',difficulty:null,progress:25,xpReward:0,dueAt:null,startedAt:null,completedAt:null,description:'Quest A'};
const questB:Quest={...questA,id:'quest-b',title:'Wrong quest'};
const conceptB:Concept={id:'concept-b',playerId:player.id,typeCode:'project',name:'Garden',description:'Concept B',isActive:true,metadataJson:'{}',createdAt:'',updatedAt:''};
const conceptC:Concept={...conceptB,id:'concept-c',name:'Harvest',description:'Concept C'};
const conceptWrong:Concept={...conceptB,id:'concept-wrong',name:'Unrelated concept'};
const effectC:Effect={id:'effect-c',playerId:player.id,targetKind:'player',targetConceptId:null,typeCode:'buff',name:'Focused',description:'Effect C',startedAt:'',expiresAt:null,deactivatedAt:null,intensity:2};
const overview:WorldOverview={player,quests:[questA,questB],skillTrees:[],skills:[],effects:[effectC],recentTransactions:[],narratives:[]};
const questAssociation:ConceptAssociation={id:'association-a',playerId:player.id,conceptId:conceptB.id,entityKind:'quest',entityId:questA.id,associationCode:'about',isActive:true,metadataJson:'{}',createdAt:'',updatedAt:''};
const relation:ConceptRelationship={id:'relation-b-c',playerId:player.id,sourceConceptId:conceptB.id,targetConceptId:conceptC.id,relationshipCode:'related_to',isActive:true,createdAt:'',updatedAt:''};
const client={
 getWorldOverview:vi.fn(async()=>overview),listConcepts:vi.fn(async()=>[conceptB,conceptC,conceptWrong]),getConcept:vi.fn(async(id:string)=>[conceptB,conceptC,conceptWrong].find(x=>x.id===id)??null),
 listConceptAssociations:vi.fn(async(id:string)=>id===conceptB.id?[questAssociation]:[]),listConceptRelationships:vi.fn(async(id:string)=>id===conceptB.id?[relation]:[]),
 setPlayerProgression:vi.fn(async()=>player),setConceptActive:vi.fn(async()=>conceptB),
} as unknown as CoreClient;
function Launcher(){const{openSurface}=useEntitySurface();return <><button onClick={()=>openSurface('quest',questA.id)}>Open Quest A</button><button onClick={()=>openSurface('quest',questB.id)}>Open Quest B</button><button onClick={()=>openSurface('effect',effectC.id)}>Open Effect C</button></>}
function renderSurface(){return render(<EntitySurfaceProvider client={client} player={player} overview={overview} concepts={[conceptB,conceptC,conceptWrong]} onRefresh={async()=>{}}><Launcher/></EntitySurfaceProvider>)}

describe('EntitySurface',()=>{
 it('resolves Quest A by its real ID, not array position, and preserves context',async()=>{renderSurface();fireEvent.click(screen.getByRole('button',{name:'Open Quest A'}));expect(await screen.findByRole('heading',{name:'Prepare the garden'})).toBeInTheDocument();expect(screen.getByRole('button',{name:'Open Quest A'})).toBeInTheDocument();expect(screen.queryByRole('heading',{name:'Wrong quest'})).not.toBeInTheDocument();});
 it('resolves Effect C by its real ID and copies human-readable content',async()=>{renderSurface();fireEvent.click(screen.getByRole('button',{name:'Open Effect C'}));expect(await screen.findByRole('heading',{name:'Focused'})).toBeInTheDocument();const write=vi.fn().mockResolvedValue(undefined);Object.defineProperty(navigator,'clipboard',{value:{writeText:write},configurable:true});fireEvent.click(screen.getByRole('button',{name:'Copy'}));await waitFor(()=>expect(write).toHaveBeenCalledWith(expect.stringContaining('Effect: Focused')));expect(write).not.toHaveBeenCalledWith(expect.stringContaining('{"id"'));});
 it('opens the actual Quest A attachment, then Concept B and related Concept C',async()=>{renderSurface();fireEvent.click(screen.getByRole('button',{name:'Open Quest A'}));fireEvent.click(await screen.findByRole('button',{name:/Garden/}));expect(await screen.findByRole('heading',{name:'Garden'})).toBeInTheDocument();fireEvent.click(await screen.findByRole('button',{name:/Harvest/}));expect(await screen.findByRole('heading',{name:'Harvest'})).toBeInTheDocument();fireEvent.click(screen.getByRole('button',{name:/Back to Garden/}));expect(await screen.findByRole('heading',{name:'Garden'})).toBeInTheDocument();fireEvent.click(screen.getByRole('button',{name:/Back to Prepare the garden/}));expect(await screen.findByRole('heading',{name:'Prepare the garden'})).toBeInTheDocument();});
 it('saves and cancels a non-Player Concept edit in the same surface',async()=>{render(<EntitySurfaceProvider client={client} player={player} overview={overview} concepts={[conceptB,conceptC]} onRefresh={async()=>{}}><button onClick={()=>useEntitySurface}>Context</button><OpenConcept/></EntitySurfaceProvider>);fireEvent.click(screen.getByRole('button',{name:'Open Concept B'}));fireEvent.click(await screen.findByRole('button',{name:'Edit'}));fireEvent.click(screen.getByLabelText('Active in this world'));fireEvent.click(screen.getByRole('button',{name:'Save'}));await waitFor(()=>expect(client.setConceptActive).toHaveBeenCalledWith(conceptB.id,false));expect(await screen.findByRole('status')).toHaveTextContent('Concept details saved');});
 it('closes the topmost surface, restores focus, keeps route context, and owns independent scrolling',async()=>{renderSurface();const trigger=screen.getByRole('button',{name:'Open Quest A'});trigger.focus();fireEvent.click(trigger);await screen.findByRole('heading',{name:'Prepare the garden'});expect(document.body.style.overflow).toBe('hidden');expect(screen.getByTestId('entity-surface-layer')).toBeInTheDocument();fireEvent.keyDown(document,{key:'Escape'});await waitFor(()=>expect(screen.queryByTestId('entity-surface-layer')).not.toBeInTheDocument());expect(document.activeElement).toBe(trigger);});
});
function OpenConcept(){const{openSurface}=useEntitySurface();return <button onClick={()=>openSurface('concept',conceptB.id)}>Open Concept B</button>}
