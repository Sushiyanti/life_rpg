import type { WorkspacePanelSort, WorkspacePanelType, WorkspacePanelVariant } from '../../domain/world';

export type PanelDefinition = {
  label: string;
  description: string;
  variants: readonly WorkspacePanelVariant[];
  sorts: readonly WorkspacePanelSort[];
  statuses: readonly string[];
  supportsActive: boolean;
  supportsType: boolean;
  supportsConcept: boolean;
  supportsTags: boolean;
  supportsRecent: boolean;
  defaultVariant: WorkspacePanelVariant;
  defaultSort: WorkspacePanelSort;
};

export const PANEL_REGISTRY: Record<WorkspacePanelType, PanelDefinition> = {
  player: {label:'Player status',description:'Authored level, XP, and world attributes.',variants:['metrics','cards','compact','detailed'],sorts:['name_asc','updated_desc'],statuses:[],supportsActive:false,supportsType:false,supportsConcept:false,supportsTags:false,supportsRecent:false,defaultVariant:'metrics',defaultSort:'name_asc'},
  quests: {label:'Quest board',description:'Goals and explicit progress.',variants:['cards','rows','compact','detailed'],sorts:['updated_desc','created_desc','name_asc','status_asc','progress_desc'],statuses:['open','active','completed','abandoned'],supportsActive:false,supportsType:true,supportsConcept:true,supportsTags:true,supportsRecent:true,defaultVariant:'cards',defaultSort:'updated_desc'},
  skills: {label:'Skill growth',description:'Practice areas and player-authored levels.',variants:['cards','rows','compact','tree'],sorts:['name_asc','updated_desc','created_desc','status_asc','level_desc'],statuses:['active','paused','completed','archived'],supportsActive:true,supportsType:true,supportsConcept:true,supportsTags:true,supportsRecent:true,defaultVariant:'cards',defaultSort:'name_asc'},
  concepts: {label:'World concepts',description:'Subjects and explicit relationships.',variants:['cards','rows','compact'],sorts:['name_asc','updated_desc','created_desc','status_asc','progress_desc'],statuses:['active','archived'],supportsActive:true,supportsType:true,supportsConcept:true,supportsTags:true,supportsRecent:true,defaultVariant:'cards',defaultSort:'name_asc'},
  progress: {label:'Concept progress',description:'Recorded tracks across selected Concepts.',variants:['metrics','rows','cards'],sorts:['name_asc','updated_desc','progress_desc','level_desc'],statuses:['active','archived'],supportsActive:true,supportsType:true,supportsConcept:true,supportsTags:false,supportsRecent:true,defaultVariant:'metrics',defaultSort:'progress_desc'},
  effects: {label:'Current effects',description:'Recorded conditions and their current lifecycle.',variants:['rows','compact','detailed'],sorts:['name_asc','updated_desc','created_desc','status_asc'],statuses:['active','inactive'],supportsActive:true,supportsType:true,supportsConcept:true,supportsTags:true,supportsRecent:true,defaultVariant:'rows',defaultSort:'updated_desc'},
  activity: {label:'Recorded sessions',description:'Real periods of activity; missing days remain gaps.',variants:['rows','compact','detailed','timeline'],sorts:['started_desc','created_desc','status_asc'],statuses:['in_progress','completed','interrupted'],supportsActive:false,supportsType:false,supportsConcept:true,supportsTags:true,supportsRecent:true,defaultVariant:'rows',defaultSort:'started_desc'},
  transactions: {label:'Recent ledger',description:'Append-only recorded changes.',variants:['rows','compact','detailed','timeline'],sorts:['occurred_desc','created_desc'],statuses:[],supportsActive:false,supportsType:true,supportsConcept:true,supportsTags:false,supportsRecent:true,defaultVariant:'rows',defaultSort:'occurred_desc'},
  journal: {label:'Recent chronicle',description:'Player-authored narrative entries.',variants:['cards','rows','compact','detailed'],sorts:['created_desc','updated_desc','name_asc'],statuses:[],supportsActive:false,supportsType:true,supportsConcept:true,supportsTags:true,supportsRecent:true,defaultVariant:'rows',defaultSort:'created_desc'},
  timeline: {label:'Recorded Timeline',description:'Bounded, read-only history with source, identity, and inclusive date filters.',variants:['timeline','rows','compact'],sorts:['timeline_newest','timeline_oldest'],statuses:[],supportsActive:false,supportsType:false,supportsConcept:true,supportsTags:false,supportsRecent:false,defaultVariant:'timeline',defaultSort:'timeline_newest'},
};

export const panelLabel = (type: WorkspacePanelType) => PANEL_REGISTRY[type].label;
