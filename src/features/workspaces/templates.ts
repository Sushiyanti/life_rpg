import type { TimelineCategory, TimelineEntityKind, WorkspacePanelSort, WorkspacePanelType, WorkspacePanelVariant, WorkspaceTemplate } from '../../domain/world';

export type TemplatePanel = {
  panelType: WorkspacePanelType;
  title: string;
  variant: WorkspacePanelVariant;
  density: 'cozy'|'compact';
  filterStatus?: string|null;
  filterActive?: boolean|null;
  filterTypeCode?: string|null;
  filterRecentDays?: number|null;
  filterTimelineCategory?: TimelineCategory|null;
  filterTimelineEntityKind?: TimelineEntityKind|null;
  filterTimelineFrom?: string|null;
  filterTimelineThrough?: string|null;
  sortBy: WorkspacePanelSort;
  itemLimit: number;
  gridSpan?: 1|2;
  isPinned: boolean;
};
export type WorkspaceTemplateDefinition = {id:Exclude<WorkspaceTemplate,'custom'>;name:string;description:string;panels:TemplatePanel[]};
const p=(panelType:WorkspacePanelType,title:string,options:Partial<Omit<TemplatePanel,'panelType'|'title'>>={}):TemplatePanel=>({panelType,title,variant:'rows',density:'cozy',filterStatus:null,filterActive:null,filterTypeCode:null,filterRecentDays:null,filterTimelineCategory:null,filterTimelineEntityKind:null,filterTimelineFrom:null,filterTimelineThrough:null,sortBy:panelType==='timeline'?'timeline_newest':'updated_desc',itemLimit:6,gridSpan:1,isPinned:false,...options});
export const WORKSPACE_TEMPLATES: WorkspaceTemplateDefinition[] = [
  {id:'overview',name:'Overview',description:'A balanced, responsive view of your world.',panels:[
    p('player','Player status',{variant:'metrics',sortBy:'name_asc',isPinned:true}),
    p('quests','In progress',{variant:'cards',filterStatus:'active',sortBy:'updated_desc',isPinned:true}),
    p('quests','Up next',{variant:'rows',filterStatus:'open',sortBy:'created_desc'}),
    p('skills','Skill growth',{variant:'cards',sortBy:'level_desc'}),
    p('effects','Current effects',{variant:'compact',filterActive:true,sortBy:'updated_desc'}),
    p('activity','Recent sessions',{variant:'timeline',filterRecentDays:7,sortBy:'started_desc'}),
    p('timeline','Recorded Timeline',{variant:'timeline',filterTimelineCategory:'session',sortBy:'timeline_newest',itemLimit:8}),
    p('journal','Recent chronicle',{variant:'rows',filterRecentDays:14,sortBy:'created_desc'}),
  ]},
  {id:'focus',name:'Focus',description:'Keep active objectives and real work close.',panels:[
    p('quests','In progress',{variant:'detailed',filterStatus:'active',sortBy:'updated_desc',isPinned:true}),
    p('activity','Active sessions',{variant:'timeline',filterStatus:'in_progress',sortBy:'started_desc',isPinned:true}),
    p('quests','Waiting for later',{variant:'compact',filterStatus:'open',sortBy:'created_desc'}),
    p('journal','Recent notes',{variant:'compact',filterRecentDays:7,sortBy:'created_desc'}),
  ]},
  {id:'learning',name:'Learning',description:'Bring practice, subjects, and learning notes together.',panels:[
    p('skills','Skills tracked',{variant:'tree',filterActive:true,sortBy:'level_desc',isPinned:true}),
    p('concepts','Subjects & concepts',{variant:'cards',filterActive:true,sortBy:'name_asc',isPinned:true}),
    p('progress','Concept progress',{variant:'metrics',filterActive:true,sortBy:'progress_desc'}),
    p('quests','Open learning objectives',{variant:'rows',filterStatus:'active',sortBy:'updated_desc'}),
    p('journal','Recent reading & reflections',{variant:'detailed',filterRecentDays:14,sortBy:'created_desc'}),
  ]},
  {id:'health',name:'Health',description:'A calm view of practice, conditions, and records.',panels:[
    p('player','World attributes',{variant:'metrics',sortBy:'name_asc',isPinned:true}),
    p('skills','Health practices',{variant:'cards',filterActive:true,sortBy:'level_desc',isPinned:true}),
    p('effects','Current conditions',{variant:'detailed',filterActive:true,sortBy:'updated_desc'}),
    p('activity','Recorded sessions',{variant:'timeline',filterRecentDays:14,sortBy:'started_desc'}),
    p('journal','Health notes',{variant:'rows',filterRecentDays:30,sortBy:'created_desc'}),
  ]},
  {id:'review',name:'Review',description:'Review recorded work and authored history.',panels:[
    p('activity','Recent activity',{variant:'timeline',filterRecentDays:30,sortBy:'started_desc',isPinned:true}),
    p('timeline','Recorded history',{variant:'timeline',sortBy:'timeline_newest',itemLimit:10,isPinned:true}),
    p('transactions','Recent ledger',{variant:'detailed',filterRecentDays:30,sortBy:'occurred_desc',isPinned:true}),
    p('journal','Recent chronicle',{variant:'cards',filterRecentDays:30,sortBy:'created_desc'}),
    p('quests','Completed quests',{variant:'rows',filterStatus:'completed',sortBy:'updated_desc'}),
    p('progress','Concept progress',{variant:'metrics',sortBy:'progress_desc'}),
  ]},
];
