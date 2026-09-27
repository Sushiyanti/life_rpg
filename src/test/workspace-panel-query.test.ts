import { describe, expect, it } from 'vitest';
import type { WorkspacePanel } from '../domain/world';
import { panelSearchQuery } from '../features/world/WorldWorkspace';

const panel=(panelType:WorkspacePanel['panelType'],patch:Partial<WorkspacePanel>={}):WorkspacePanel=>({
  id:'panel-1',workspaceId:'workspace-1',panelType,title:'View',variant:'rows',density:'cozy',
  filterStatus:null,filterActive:null,filterTypeCode:null,filterConceptId:null,filterRecentDays:null,
  sortBy:'updated_desc',itemLimit:10,sortOrder:0,gridSpan:1,isVisible:true,isPinned:false,isCollapsed:false,
  createdAt:'2026-09-27T00:00:00Z',updatedAt:'2026-09-27T00:00:00Z',...patch,
});

describe('declarative panel search mapping',()=>{
  it('uses only existing singular entity kinds for concepts and sessions',()=>{
    expect(panelSearchQuery(panel('concepts'),'p1').kind).toBe('concept');
    expect(panelSearchQuery(panel('activity'),'p1').kind).toBe('quest_session');
    expect(panelSearchQuery(panel('transactions'),'p1').kind).toBe('transaction');
  });
  it('maps Quest status without treating `active` as a generic boolean filter',()=>{
    const query=panelSearchQuery(panel('quests',{filterStatus:'active'}),'p1');
    expect(query.status).toBe('active');
    expect(query.active).toBeNull();
  });
  it('maps Concept lifecycle to the typed active field and progress filters locally',()=>{
    const archived=panelSearchQuery(panel('concepts',{filterStatus:'archived'}),'p1');
    expect(archived.status).toBeNull();
    expect(archived.active).toBe(false);
    const progress=panelSearchQuery(panel('progress',{filterTypeCode:'reading',filterRecentDays:30}),'p1');
    expect(progress.kind).toBe('concept');
    expect(progress.typeCode).toBeNull();
    expect(progress.from).toBeNull();
  });
  it('scopes every query to its owning Player and Dashboard presentation context',()=>{
    const query=panelSearchQuery(panel('activity',{filterConceptId:'concept-1',filterStatus:'in_progress',filterRecentDays:14}),'player-7');
    expect(query).toMatchObject({playerId:'player-7',conceptId:'concept-1',status:'in_progress',from:expect.any(String),context:'dashboard'});
  });
});
