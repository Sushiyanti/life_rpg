import type { WorkspacePanelType, WorkspaceTemplate } from '../../domain/world';

export type TemplatePanel = { panelType: WorkspacePanelType; title: string; variant: 'cards'|'rows'; density: 'cozy'|'compact'; itemLimit: number; isPinned: boolean };
export const WORKSPACE_TEMPLATES: { id: Exclude<WorkspaceTemplate,'custom'>; name: string; description: string; panels: TemplatePanel[] }[] = [
  { id:'overview', name:'Overview', description:'A balanced view of your world.', panels:[
    {panelType:'quests',title:'Quest board',variant:'cards',density:'cozy',itemLimit:6,isPinned:true},
    {panelType:'activity',title:'Live sessions',variant:'rows',density:'compact',itemLimit:5,isPinned:true},
    {panelType:'skills',title:'Skill growth',variant:'cards',density:'cozy',itemLimit:6,isPinned:false},
    {panelType:'concepts',title:'World concepts',variant:'cards',density:'cozy',itemLimit:6,isPinned:false},
    {panelType:'effects',title:'Current effects',variant:'rows',density:'compact',itemLimit:5,isPinned:false},
    {panelType:'transactions',title:'Recent ledger',variant:'rows',density:'compact',itemLimit:5,isPinned:false},
    {panelType:'journal',title:'Recent chronicle',variant:'rows',density:'compact',itemLimit:4,isPinned:false},
  ]},
  { id:'focus',name:'Focus',description:'Keep objectives and current work close.',panels:[
    {panelType:'quests',title:'Current objectives',variant:'cards',density:'cozy',itemLimit:6,isPinned:true},
    {panelType:'activity',title:'Active sessions',variant:'rows',density:'compact',itemLimit:5,isPinned:true},
    {panelType:'journal',title:'Notes for today',variant:'rows',density:'compact',itemLimit:4,isPinned:false},
  ]},
  { id:'learning',name:'Learning',description:'Bring practice, subjects, and learning notes together.',panels:[
    {panelType:'skills',title:'Skills in practice',variant:'cards',density:'cozy',itemLimit:8,isPinned:true},
    {panelType:'concepts',title:'Subjects & concepts',variant:'cards',density:'cozy',itemLimit:8,isPinned:true},
    {panelType:'quests',title:'Learning objectives',variant:'rows',density:'compact',itemLimit:5,isPinned:false},
    {panelType:'journal',title:'Reading & reflections',variant:'rows',density:'compact',itemLimit:5,isPinned:false},
  ]},
  { id:'health',name:'Health',description:'A calm place for practice, effects, and records.',panels:[
    {panelType:'skills',title:'Health practices',variant:'cards',density:'cozy',itemLimit:6,isPinned:true},
    {panelType:'effects',title:'Current conditions',variant:'rows',density:'compact',itemLimit:5,isPinned:true},
    {panelType:'activity',title:'Recorded sessions',variant:'rows',density:'compact',itemLimit:5,isPinned:false},
    {panelType:'journal',title:'Health notes',variant:'rows',density:'compact',itemLimit:5,isPinned:false},
  ]},
  { id:'review',name:'Review',description:'Review your recorded work and authored history.',panels:[
    {panelType:'transactions',title:'Recent ledger',variant:'rows',density:'compact',itemLimit:10,isPinned:true},
    {panelType:'journal',title:'Chronicle',variant:'rows',density:'compact',itemLimit:8,isPinned:true},
    {panelType:'quests',title:'Quest progress',variant:'cards',density:'cozy',itemLimit:6,isPinned:false},
    {panelType:'concepts',title:'World concepts',variant:'rows',density:'compact',itemLimit:6,isPinned:false},
  ]},
];
