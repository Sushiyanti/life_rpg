import type { Concept, TimelineCategory, TimelineEntityKind, Workspace, WorkspacePanel, WorkspacePanelSort, WorkspacePanelType, WorkspacePanelVariant, WorkspaceTemplate } from '../../domain/world';
import { PANEL_REGISTRY } from './panelRegistry';

export type ConceptReference = { key: string; name: string; typeCode: string };
export type TimelineEntityReference = { kind: TimelineEntityKind; label: 'Specific local record' };
export type PortablePanel = Pick<WorkspacePanel,
  'panelType'|'title'|'variant'|'density'|'filterStatus'|'filterActive'|'filterTypeCode'|'filterRecentDays'|'filterTimelineCategory'|'filterTimelineEntityKind'|'filterTimelineFrom'|'filterTimelineThrough'|'sortBy'|'itemLimit'|'sortOrder'|'gridSpan'|'isVisible'|'isPinned'|'isCollapsed'> & {
    relatedConcept: ConceptReference | null;
    timelineEntityIdentity: TimelineEntityReference | null;
  };
export type PortableWorkspace = {
  format: 'life-rpg-workspace';
  version: 3;
  workspace: { name: string; template: WorkspaceTemplate };
  panels: PortablePanel[];
};
type V2Panel = Pick<WorkspacePanel,'panelType'|'title'|'variant'|'density'|'filterStatus'|'filterActive'|'filterTypeCode'|'filterRecentDays'|'sortBy'|'itemLimit'|'sortOrder'|'gridSpan'|'isVisible'|'isPinned'|'isCollapsed'> & {relatedConcept:ConceptReference|null};
type V2Workspace = {format:'life-rpg-workspace';version:2;workspace:PortableWorkspace['workspace'];panels:V2Panel[]};
export type WorkspaceImportPreview = {
  portable: PortableWorkspace;
  references: { panelIndex: number; reference: ConceptReference; autoResolvedId: string | null; candidates: Concept[] }[];
  legacyConceptPanels: number[];
  unresolvedTimelineIdentityPanels: number[];
};
export type ResolvedWorkspaceImport = {
  workspace: PortableWorkspace['workspace'];
  panels: (Omit<PortablePanel, 'relatedConcept'|'timelineEntityIdentity'> & { filterConceptId: string | null; filterTimelineEntityId: string | null })[];
  resolvedConcepts: number;
  unresolvedConcepts: number;
  unresolvedTimelineIdentities: number;
};

const record = (value: unknown): value is Record<string, unknown> => !!value && typeof value === 'object' && !Array.isArray(value);
const exactKeys = (value: Record<string, unknown>, keys: string[]) => Object.keys(value).every(k => keys.includes(k)) && keys.every(k => Object.hasOwn(value,k));
const nullableString = (v: unknown, max=160): v is string|null => v === null || (typeof v === 'string' && v.length > 0 && v.length <= max);
const templates: WorkspaceTemplate[] = ['overview','focus','learning','health','review','custom'];
const timelineCategories: TimelineCategory[] = ['session','transaction','effect_history','content','comment','concept_progress','revision','snapshot','record_change','lifecycle','relationship_history'];
const timelineEntityKinds: TimelineEntityKind[] = ['player','concept','quest','quest_stage','quest_branch','quest_session','skill_tree','skill','effect','transaction','comment','narrative_entry','concept_progress'];
const v3Keys = ['panelType','title','variant','density','filterStatus','filterActive','filterTypeCode','relatedConcept','filterRecentDays','filterTimelineCategory','filterTimelineEntityKind','filterTimelineFrom','filterTimelineThrough','timelineEntityIdentity','sortBy','itemLimit','sortOrder','gridSpan','isVisible','isPinned','isCollapsed'];
const v2Keys = ['panelType','title','variant','density','filterStatus','filterActive','filterTypeCode','relatedConcept','filterRecentDays','sortBy','itemLimit','sortOrder','gridSpan','isVisible','isPinned','isCollapsed'];
const legacyPanelKeys = v2Keys.filter(k => k !== 'relatedConcept').concat('filterConceptId');
const normalizeName = (name: string) => name.trim().normalize('NFKC').toLowerCase();

function validateReference(value: unknown, panelNumber: number): ConceptReference | null {
  if (value === null) return null;
  if (!record(value) || !exactKeys(value,['key','name','typeCode']) || typeof value.key !== 'string' || !/^concept-ref-[a-zA-Z0-9-]{8,112}$/.test(value.key) || typeof value.name !== 'string' || !value.name.trim() || value.name.length > 512 || typeof value.typeCode !== 'string' || !/^[a-z0-9_]{1,160}$/.test(value.typeCode)) throw new Error(`Panel ${panelNumber} has an invalid Concept reference.`);
  return {key:value.key,name:value.name.trim(),typeCode:value.typeCode};
}
function validateCommonPanel(raw:Record<string,unknown>,i:number,reference:ConceptReference|null, timeline: boolean): PortablePanel {
  const type = raw.panelType as WorkspacePanelType;
  const def = PANEL_REGISTRY[type];
  const category = timeline ? raw.filterTimelineCategory : null;
  const entityKind = timeline ? raw.filterTimelineEntityKind : null;
  const from = timeline ? raw.filterTimelineFrom : null;
  const through = timeline ? raw.filterTimelineThrough : null;
  const identity = timeline ? raw.timelineEntityIdentity : null;
  if (!def || typeof raw.variant !== 'string' || !def.variants.includes(raw.variant as WorkspacePanelVariant) || (raw.density !== 'cozy' && raw.density !== 'compact') || (typeof raw.title !== 'string' && raw.title !== null) || (typeof raw.title === 'string' && (!raw.title.trim() || raw.title.length > 120)) || (raw.filterStatus !== null && (typeof raw.filterStatus !== 'string' || !def.statuses.includes(raw.filterStatus))) || (raw.filterActive !== null && typeof raw.filterActive !== 'boolean') || (raw.filterTypeCode !== null && (!nullableString(raw.filterTypeCode) || !/^[a-z0-9_]{1,160}$/.test(raw.filterTypeCode))) || (raw.filterRecentDays !== null && (!Number.isInteger(raw.filterRecentDays) || (raw.filterRecentDays as number)<1 || (raw.filterRecentDays as number)>365)) || typeof raw.sortBy !== 'string' || !def.sorts.includes(raw.sortBy as WorkspacePanelSort) || !Number.isInteger(raw.itemLimit) || (raw.itemLimit as number)<1 || (raw.itemLimit as number)>50 || !Number.isInteger(raw.sortOrder) || (raw.sortOrder as number)<0 || (raw.sortOrder as number)>999 || ![1,2].includes(raw.gridSpan as number) || typeof raw.isVisible !== 'boolean' || typeof raw.isPinned !== 'boolean' || typeof raw.isCollapsed !== 'boolean') throw new Error(`Panel ${i+1} contains a value not allowed for its source.`);
  if ((!def.supportsActive && raw.filterActive !== null) || (!def.supportsType && raw.filterTypeCode !== null) || (!def.supportsConcept && reference !== null) || (!def.supportsRecent && raw.filterRecentDays !== null)) throw new Error(`Panel ${i+1} uses a filter unsupported by its source.`);
  if (timeline) {
    const validDate = (v: unknown) => v === null || (typeof v === 'string' && /^\d{4}-\d{2}-\d{2}$/.test(v) && !Number.isNaN(Date.parse(`${v}T00:00:00Z`)));
    if (type !== 'timeline' && (category !== null || entityKind !== null || from !== null || through !== null || identity !== null)) throw new Error(`Panel ${i+1} uses Timeline filters unsupported by its source.`);
    if (category !== null && (typeof category !== 'string' || !timelineCategories.includes(category as TimelineCategory))) throw new Error(`Panel ${i+1} has an invalid Timeline category.`);
    if (entityKind !== null && (typeof entityKind !== 'string' || !timelineEntityKinds.includes(entityKind as TimelineEntityKind))) throw new Error(`Panel ${i+1} has an invalid Timeline entity kind.`);
    if (!validDate(from) || !validDate(through) || (from !== null && through !== null && (from as string) > (through as string))) throw new Error(`Panel ${i+1} has an invalid Timeline date range.`);
    if (identity !== null && (!record(identity) || !exactKeys(identity,['kind','label']) || !timelineEntityKinds.includes(identity.kind as TimelineEntityKind) || identity.label !== 'Specific local record' || identity.kind !== entityKind)) throw new Error(`Panel ${i+1} has an invalid Timeline identity reference.`);
  }
  return {...raw, relatedConcept:reference} as PortablePanel;
}
function validateV3(value: unknown): PortableWorkspace {
  if (!record(value) || value.format !== 'life-rpg-workspace') throw new Error('This file is not a Life RPG workspace export.');
  if (value.version !== 3) throw new Error(`Workspace format version ${String(value.version)} is not supported.`);
  if (!exactKeys(value,['format','version','workspace','panels']) || !record(value.workspace) || !exactKeys(value.workspace,['name','template']) || typeof value.workspace.name !== 'string' || !value.workspace.name.trim() || value.workspace.name.trim().length > 80 || typeof value.workspace.template !== 'string' || !templates.includes(value.workspace.template as WorkspaceTemplate) || !Array.isArray(value.panels) || value.panels.length > 100) throw new Error('The workspace export is malformed or exceeds supported limits.');
  const panels = value.panels.map((raw,i)=>{
    if (!record(raw) || !exactKeys(raw,v3Keys)) throw new Error(`Panel ${i+1} has missing or unknown configuration fields.`);
    const reference=validateReference(raw.relatedConcept,i+1);
    return validateCommonPanel(raw,i,reference,true);
  });
  return {format:'life-rpg-workspace',version:3,workspace:{name:(value.workspace.name as string).trim(),template:value.workspace.template as WorkspaceTemplate},panels};
}
function validateV2(value: unknown): V2Workspace {
  if (!record(value) || value.format !== 'life-rpg-workspace') throw new Error('This file is not a Life RPG workspace export.');
  if (value.version !== 2) throw new Error(`Workspace format version ${String(value.version)} is not supported.`);
  if (!exactKeys(value,['format','version','workspace','panels']) || !record(value.workspace) || !exactKeys(value.workspace,['name','template']) || typeof value.workspace.name !== 'string' || !value.workspace.name.trim() || value.workspace.name.trim().length > 80 || typeof value.workspace.template !== 'string' || !templates.includes(value.workspace.template as WorkspaceTemplate) || !Array.isArray(value.panels) || value.panels.length > 100) throw new Error('The workspace export is malformed or exceeds supported limits.');
  const panels=value.panels.map((raw,i)=>{
    if(!record(raw)||!exactKeys(raw,v2Keys))throw new Error(`Panel ${i+1} has missing or unknown configuration fields.`);
    const reference=validateReference(raw.relatedConcept,i+1);
    const panel=validateCommonPanel({...raw,filterTimelineCategory:null,filterTimelineEntityKind:null,filterTimelineFrom:null,filterTimelineThrough:null,timelineEntityIdentity:null},i,reference,false);
    const {filterTimelineCategory:_a,filterTimelineEntityKind:_b,filterTimelineFrom:_c,filterTimelineThrough:_d,timelineEntityIdentity:_e,...legacy}=panel;
    return legacy as V2Panel;
  });
  return {format:'life-rpg-workspace',version:2,workspace:{name:(value.workspace.name as string).trim(),template:value.workspace.template as WorkspaceTemplate},panels};
}
function upgradeV2(value:unknown):PortableWorkspace {
  const old=validateV2(value);
  return validateV3({format:'life-rpg-workspace',version:3,workspace:old.workspace,panels:old.panels.map(p=>({...p,filterTimelineCategory:null,filterTimelineEntityKind:null,filterTimelineFrom:null,filterTimelineThrough:null,timelineEntityIdentity:null}))});
}
function migrateV1(value: unknown): { portable: PortableWorkspace; legacyConceptPanels: number[] } {
  if (!record(value) || value.format !== 'life-rpg-workspace') throw new Error('This file is not a Life RPG workspace export.');
  if (value.version !== 1) throw new Error(`Workspace format version ${String(value.version)} is not supported.`);
  if (!exactKeys(value,['format','version','workspace','panels']) || !record(value.workspace) || !exactKeys(value.workspace,['name','template']) || !Array.isArray(value.panels) || value.panels.length > 100) throw new Error('The legacy workspace export is malformed or exceeds supported limits.');
  const legacyConceptPanels:number[]=[];
  const panels=value.panels.map((raw,i)=>{
    if(!record(raw)||!exactKeys(raw,legacyPanelKeys)||!Object.hasOwn(raw,'filterConceptId')||(raw.filterConceptId!==null&&typeof raw.filterConceptId!=='string'))throw new Error(`Legacy panel ${i+1} contains unknown or invalid fields.`);
    if(raw.filterConceptId!==null)legacyConceptPanels.push(i);
    const {filterConceptId:_untrustedLocalId,...rest}=raw;
    const filterStatus = rest.panelType === 'quests' && rest.filterStatus === 'in_progress' ? 'active' : rest.panelType === 'quests' && rest.filterStatus === 'pending' ? 'open' : rest.panelType === 'activity' && rest.filterStatus === 'active' ? 'in_progress' : rest.filterStatus;
    return {...rest,filterStatus,relatedConcept:null};
  });
  const portable=upgradeV2({format:'life-rpg-workspace',version:2,workspace:value.workspace,panels});
  return {portable,legacyConceptPanels};
}
export function validateWorkspaceImport(value: unknown): PortableWorkspace { return validateV3(value); }
export function previewWorkspaceImport(value: unknown, concepts: Concept[]): WorkspaceImportPreview {
  let portable:PortableWorkspace;let legacyConceptPanels:number[]=[];
  if(record(value)&&value.version===1){const migrated=migrateV1(value);portable=migrated.portable;legacyConceptPanels=migrated.legacyConceptPanels;}
  else if(record(value)&&value.version===2)portable=upgradeV2(value);
  else portable=validateV3(value);
  const references:WorkspaceImportPreview['references']=[];const unresolvedTimelineIdentityPanels:number[]=[];
  portable.panels.forEach((panel,panelIndex)=>{
    if(panel.timelineEntityIdentity)unresolvedTimelineIdentityPanels.push(panelIndex);
    const reference=panel.relatedConcept;if(!reference)return;
    const exact=concepts.filter(c=>c.transferKey===reference.key&&c.typeCode===reference.typeCode);
    const candidates=[...new Map([...exact,...concepts.filter(c=>c.typeCode===reference.typeCode&&normalizeName(c.name)===normalizeName(reference.name))].map(c=>[c.id,c])).values()];
    references.push({panelIndex,reference,autoResolvedId:exact.length===1?exact[0]!.id:null,candidates});
  });
  return {portable,references,legacyConceptPanels,unresolvedTimelineIdentityPanels};
}
export function resolveWorkspaceImport(preview:WorkspaceImportPreview,selections:Record<number,string>,concepts:Concept[]):ResolvedWorkspaceImport {
  const refs=new Map(preview.references.map(ref=>[ref.panelIndex,ref]));let resolvedConcepts=0;let unresolvedConcepts=preview.legacyConceptPanels.length;
  const panels=preview.portable.panels.map((panel,panelIndex)=>{
    const ref=refs.get(panelIndex);let filterConceptId:string|null=null;
    if(ref){const selected=selections[panelIndex];if(ref.autoResolvedId)filterConceptId=selected===undefined?ref.autoResolvedId:selected||null;else if(selected)filterConceptId=selected;if(filterConceptId&&!ref.candidates.some(c=>c.id===filterConceptId))throw new Error(`Panel ${panelIndex+1} selected a Concept that was not offered by the preview.`);if(filterConceptId&&!concepts.some(c=>c.id===filterConceptId))throw new Error(`Panel ${panelIndex+1} references a Concept outside this Player world.`);if(filterConceptId)resolvedConcepts++;else unresolvedConcepts++;}
    const {relatedConcept:_portableReference,timelineEntityIdentity:_identity,...configuration}=panel;
    return {...configuration,filterConceptId,filterTimelineEntityId:null};
  });
  return {workspace:preview.portable.workspace,panels,resolvedConcepts,unresolvedConcepts,unresolvedTimelineIdentities:preview.unresolvedTimelineIdentityPanels.length};
}
export function exportWorkspace(workspace:Workspace,panels:WorkspacePanel[],concepts:Concept[]):PortableWorkspace {
  const byId=new Map(concepts.filter(c=>c.playerId===workspace.playerId).map(c=>[c.id,c]));
  const portablePanels=panels.map(({panelType,title,variant,density,filterStatus,filterActive,filterTypeCode,filterConceptId,filterRecentDays,filterTimelineCategory,filterTimelineEntityKind,filterTimelineEntityId,filterTimelineFrom,filterTimelineThrough,sortBy,itemLimit,sortOrder,gridSpan,isVisible,isPinned,isCollapsed})=>{
    const concept=filterConceptId?byId.get(filterConceptId):undefined;if(filterConceptId&&!concept)throw new Error('A panel Concept filter is not available in this workspace Player world.');
    if(filterTimelineEntityId&&!filterTimelineEntityKind)throw new Error('A Timeline exact identity filter requires an entity kind.');
    return {panelType,title,variant,density,filterStatus,filterActive,filterTypeCode,relatedConcept:concept?{key:concept.transferKey,name:concept.name,typeCode:concept.typeCode}:null,filterRecentDays,filterTimelineCategory,filterTimelineEntityKind,filterTimelineFrom,filterTimelineThrough,timelineEntityIdentity:filterTimelineEntityId?{kind:filterTimelineEntityKind!,label:'Specific local record' as const}:null,sortBy,itemLimit,sortOrder,gridSpan,isVisible,isPinned,isCollapsed};
  });
  const stablePanels=portablePanels.sort((a,b)=>{const order=a.sortOrder-b.sortOrder;if(order!==0)return order;const left=JSON.stringify(a),right=JSON.stringify(b);return left<right?-1:left>right?1:0;});
  return validateV3({format:'life-rpg-workspace',version:3,workspace:{name:workspace.name,template:workspace.template},panels:stablePanels});
}
