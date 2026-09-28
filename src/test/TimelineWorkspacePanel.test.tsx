import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import type { CoreClient } from '../domain/ipc';
import type { TimelineItem, TimelineQuery, Workspace, WorkspacePanel } from '../domain/world';
import { TimelineWorkspacePanel } from '../features/workspaces/TimelineWorkspacePanel';
import { WorkspaceBuilder } from '../features/workspaces/WorkspaceBuilder';

const panel=(patch:Partial<WorkspacePanel>={}):WorkspacePanel=>({id:'timeline-panel',workspaceId:'workspace-1',panelType:'timeline',title:'Recent activity',variant:'timeline',density:'cozy',filterStatus:null,filterActive:null,filterTypeCode:null,filterConceptId:null,filterRecentDays:null,filterTimelineCategory:'relationship_history',filterTimelineEntityKind:'quest',filterTimelineEntityId:'quest-7',filterTimelineFrom:'2026-09-01',filterTimelineThrough:'2026-09-28',sortBy:'timeline_oldest',itemLimit:12,sortOrder:0,gridSpan:2,isVisible:true,isPinned:false,isCollapsed:false,createdAt:'created',updatedAt:'updated',...patch});
const relation:TimelineItem={sourceId:'relationship:link-1:removed',playerId:'player-1',category:'relationship_history',entityKind:'quest',entityId:'quest-7',timestamp:'2026-09-28T10:00:00Z',secondaryTimestamp:null,timestampKind:'removed',secondaryTimestampKind:null,title:'Content relationship removed · guidance',summary:'Garden guide · Quest',conceptId:null,typeCode:null,state:'removed',relationshipContext:{relationshipId:'link-1',contentId:'content-3',contentTitle:'Garden guide',roleCode:'guidance',createdAt:'2026-09-21T10:00:00Z',removedAt:'2026-09-28T10:00:00Z'}};
const workspace:Workspace={id:'workspace-1',playerId:'player-1',name:'Review',template:'review',sortOrder:0,isDefault:true,createdAt:'created',updatedAt:'updated'};

describe('Timeline Workspace panel',()=>{
  it('sends closed persisted filters, inclusive date bounds, exact identity, and the configured page bound',async()=>{
    const queryTimeline=vi.fn(async(_query:TimelineQuery)=>[relation]);const client={queryTimeline} as unknown as CoreClient;render(<TimelineWorkspacePanel client={client} panel={panel()} playerId="player-1" onOpenEntity={vi.fn()}/>);
    expect(await screen.findByRole('button',{name:/Content relationship removed/})).toBeInTheDocument();
    expect(queryTimeline).toHaveBeenCalledWith(expect.objectContaining({playerId:'player-1',category:'relationship_history',entityKind:'quest',entityId:'quest-7',from:new Date('2026-09-01T00:00:00.000').toISOString(),through:new Date('2026-09-28T23:59:59.999').toISOString(),sort:'oldest',limit:12,offset:0}));
  });
  it('shows relationship context and navigates to the exact Content and target',async()=>{
    const onOpenEntity=vi.fn();const client={queryTimeline:vi.fn(async()=>[relation])} as unknown as CoreClient;render(<TimelineWorkspacePanel client={client} panel={panel()} playerId="player-1" onOpenEntity={onOpenEntity}/>);
    fireEvent.click(await screen.findByText(/Relationship · Garden guide · guidance/));expect(screen.getByText(/Removed/)).toBeInTheDocument();fireEvent.click(screen.getByRole('button',{name:'Open Content'}));fireEvent.click(screen.getByRole('button',{name:'Open target'}));expect(onOpenEntity).toHaveBeenNthCalledWith(1,'narrative_entry','content-3');expect(onOpenEntity).toHaveBeenNthCalledWith(2,'quest','quest-7');
  });
  it('lets the Workspace Builder save exact identity and inclusive date filters declaratively',async()=>{
    const saved=panel({filterTimelineEntityId:'session-88',filterTimelineEntityKind:'quest_session'});const saveWorkspacePanel=vi.fn(async()=>saved);const client={saveWorkspacePanel} as unknown as CoreClient;render(<WorkspaceBuilder client={client} playerId="player-1" workspace={workspace} workspaces={[workspace]} panels={[panel()]} concepts={[]} onPanelsChange={vi.fn()} onCreate={vi.fn(async()=>{})} onRename={vi.fn(async()=>{})} onDefault={vi.fn(async()=>{})} onDelete={vi.fn(async()=>{})} onDuplicate={vi.fn(async()=>{})} onImport={vi.fn(async()=>{})}/>);
    fireEvent.change(screen.getByLabelText('Timeline category'),{target:{value:'session'}});fireEvent.change(screen.getByLabelText('Timeline entity kind'),{target:{value:'quest_session'}});fireEvent.change(screen.getByLabelText('Exact entity ID'),{target:{value:'session-88'}});fireEvent.change(screen.getByLabelText('Timeline from'),{target:{value:'2026-09-04'}});fireEvent.change(screen.getByLabelText('Timeline through'),{target:{value:'2026-09-14'}});fireEvent.click(screen.getByRole('button',{name:'Save panel'}));
    expect(await screen.findByRole('status')).toHaveTextContent('Recent activity saved.');expect(saveWorkspacePanel).toHaveBeenCalledWith(expect.objectContaining({filterTimelineCategory:'session',filterTimelineEntityKind:'quest_session',filterTimelineEntityId:'session-88',filterTimelineFrom:'2026-09-04',filterTimelineThrough:'2026-09-14'}));
  });
});
