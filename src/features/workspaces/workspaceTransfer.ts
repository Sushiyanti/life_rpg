import type { Concept, Workspace, WorkspacePanel, WorkspacePanelSort, WorkspacePanelType, WorkspacePanelVariant, WorkspaceTemplate } from '../../domain/world';
import { PANEL_REGISTRY } from './panelRegistry';

export type ConceptReference = { key: string; name: string; typeCode: string };
export type PortablePanel = Pick<WorkspacePanel,
  'panelType'|'title'|'variant'|'density'|'filterStatus'|'filterActive'|'filterTypeCode'|'filterRecentDays'|'sortBy'|'itemLimit'|'sortOrder'|'gridSpan'|'isVisible'|'isPinned'|'isCollapsed'> & { relatedConcept: ConceptReference | null };
export type PortableWorkspace = {
  format: 'life-rpg-workspace';
  version: 2;
  workspace: { name: string; template: WorkspaceTemplate };
  panels: PortablePanel[];
};
export type WorkspaceImportPreview = {
  portable: PortableWorkspace;
  references: { panelIndex: number; reference: ConceptReference; autoResolvedId: string | null; candidates: Concept[] }[];
  legacyConceptPanels: number[];
};
export type ResolvedWorkspaceImport = {
  workspace: PortableWorkspace['workspace'];
  panels: (Omit<PortablePanel, 'relatedConcept'> & { filterConceptId: string | null })[];
  resolvedConcepts: number;
  unresolvedConcepts: number;
};

const record = (value: unknown): value is Record<string, unknown> => !!value && typeof value === 'object' && !Array.isArray(value);
const exactKeys = (value: Record<string, unknown>, keys: string[]) => Object.keys(value).every(k => keys.includes(k)) && keys.every(k => Object.hasOwn(value,k));
const nullableString = (v: unknown, max=160): v is string|null => v === null || (typeof v === 'string' && v.length > 0 && v.length <= max);
const templates: WorkspaceTemplate[] = ['overview','focus','learning','health','review','custom'];
const panelKeys = ['panelType','title','variant','density','filterStatus','filterActive','filterTypeCode','relatedConcept','filterRecentDays','sortBy','itemLimit','sortOrder','gridSpan','isVisible','isPinned','isCollapsed'];
const legacyPanelKeys = panelKeys.filter(k => k !== 'relatedConcept').concat('filterConceptId');
const normalizeName = (name: string) => name.trim().normalize('NFKC').toLowerCase();

function validateReference(value: unknown, panelNumber: number): ConceptReference | null {
  if (value === null) return null;
  if (!record(value) || !exactKeys(value,['key','name','typeCode']) || typeof value.key !== 'string' || !/^concept-ref-[a-zA-Z0-9-]{8,112}$/.test(value.key) || typeof value.name !== 'string' || !value.name.trim() || value.name.length > 512 || typeof value.typeCode !== 'string' || !/^[a-z0-9_]{1,160}$/.test(value.typeCode)) {
    throw new Error(`Panel ${panelNumber} has an invalid Concept reference.`);
  }
  return {key:value.key,name:value.name.trim(),typeCode:value.typeCode};
}

function validateV2(value: unknown): PortableWorkspace {
  if (!record(value) || value.format !== 'life-rpg-workspace') throw new Error('This file is not a Life RPG workspace export.');
  if (value.version !== 2) throw new Error(`Workspace format version ${String(value.version)} is not supported.`);
  if (!exactKeys(value,['format','version','workspace','panels']) || !record(value.workspace) || !exactKeys(value.workspace,['name','template']) || typeof value.workspace.name !== 'string' || !value.workspace.name.trim() || value.workspace.name.trim().length > 80 || typeof value.workspace.template !== 'string' || !templates.includes(value.workspace.template as WorkspaceTemplate) || !Array.isArray(value.panels) || value.panels.length > 100) throw new Error('The workspace export is malformed or exceeds supported limits.');
  const panels = value.panels.map((raw, i): PortablePanel => {
    if (!record(raw) || !exactKeys(raw,panelKeys)) throw new Error(`Panel ${i+1} has missing or unknown configuration fields.`);
    const type = raw.panelType as WorkspacePanelType;
    const def = PANEL_REGISTRY[type];
    const reference = validateReference(raw.relatedConcept, i+1);
    if (!def || typeof raw.variant !== 'string' || !def.variants.includes(raw.variant as WorkspacePanelVariant) || (raw.density !== 'cozy' && raw.density !== 'compact') || (typeof raw.title !== 'string' && raw.title !== null) || (typeof raw.title === 'string' && (!raw.title.trim() || raw.title.length > 120)) || (raw.filterStatus !== null && (typeof raw.filterStatus !== 'string' || !def.statuses.includes(raw.filterStatus))) || (raw.filterActive !== null && typeof raw.filterActive !== 'boolean') || (raw.filterTypeCode !== null && (!nullableString(raw.filterTypeCode) || !/^[a-z0-9_]{1,160}$/.test(raw.filterTypeCode))) || (raw.filterRecentDays !== null && (!Number.isInteger(raw.filterRecentDays) || (raw.filterRecentDays as number)<1 || (raw.filterRecentDays as number)>365)) || typeof raw.sortBy !== 'string' || !def.sorts.includes(raw.sortBy as WorkspacePanelSort) || !Number.isInteger(raw.itemLimit) || (raw.itemLimit as number)<1 || (raw.itemLimit as number)>50 || !Number.isInteger(raw.sortOrder) || (raw.sortOrder as number)<0 || (raw.sortOrder as number)>999 || ![1,2].includes(raw.gridSpan as number) || typeof raw.isVisible !== 'boolean' || typeof raw.isPinned !== 'boolean' || typeof raw.isCollapsed !== 'boolean') throw new Error(`Panel ${i+1} contains a value not allowed for its source.`);
    if ((!def.supportsActive && raw.filterActive !== null) || (!def.supportsType && raw.filterTypeCode !== null) || (!def.supportsConcept && reference !== null) || (!def.supportsRecent && raw.filterRecentDays !== null)) throw new Error(`Panel ${i+1} uses a filter unsupported by its source.`);
    return {...raw, relatedConcept:reference} as PortablePanel;
  });
  return {format:'life-rpg-workspace',version:2,workspace:{name:(value.workspace.name as string).trim(),template:value.workspace.template as WorkspaceTemplate},panels};
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
    const filterStatus = rest.panelType === 'quests' && rest.filterStatus === 'in_progress'
      ? 'active'
      : rest.panelType === 'quests' && rest.filterStatus === 'pending'
        ? 'open'
        : rest.panelType === 'activity' && rest.filterStatus === 'active'
          ? 'in_progress'
          : rest.filterStatus;
    return {...rest,filterStatus,relatedConcept:null};
  });
  const portable=validateV2({format:'life-rpg-workspace',version:2,workspace:value.workspace,panels});
  return {portable,legacyConceptPanels};
}

/** Strict v2 validator. Legacy v1 files are migrated only by previewWorkspaceImport, never by trusting their local IDs. */
export function validateWorkspaceImport(value: unknown): PortableWorkspace { return validateV2(value); }

/** Resolve exact semantic keys automatically; similar names are suggestions only and require an explicit user choice. */
export function previewWorkspaceImport(value: unknown, concepts: Concept[]): WorkspaceImportPreview {
  if(record(value)&&value.version===1){
    const migrated=migrateV1(value);
    return {portable:migrated.portable,references:[],legacyConceptPanels:migrated.legacyConceptPanels};
  }
  const portable=validateV2(value);
  const references:WorkspaceImportPreview['references']=[];
  portable.panels.forEach((panel,panelIndex)=>{
    const reference=panel.relatedConcept;
    if(!reference)return;
    const exact=concepts.filter(c=>c.transferKey===reference.key&&c.typeCode===reference.typeCode);
    const candidates=[...new Map([...exact,...concepts.filter(c=>c.typeCode===reference.typeCode&&normalizeName(c.name)===normalizeName(reference.name))].map(c=>[c.id,c])).values()];
    references.push({panelIndex,reference,autoResolvedId:exact.length===1?exact[0]!.id:null,candidates});
  });
  return {portable,references,legacyConceptPanels:[]};
}

/** The returned local IDs are taken only from this Player's loaded Concept list and are validated again by Rust. */
export function resolveWorkspaceImport(preview:WorkspaceImportPreview, selections:Record<number,string>, concepts:Concept[]):ResolvedWorkspaceImport {
  const refs=new Map(preview.references.map(ref=>[ref.panelIndex,ref]));
  let resolvedConcepts=0;
  let unresolvedConcepts=preview.legacyConceptPanels.length;
  const panels=preview.portable.panels.map((panel,panelIndex)=>{
    const ref=refs.get(panelIndex);
    let filterConceptId:string|null=null;
    if(ref){
      const selected=selections[panelIndex];
      if(ref.autoResolvedId){
        filterConceptId=selected === undefined ? ref.autoResolvedId : selected || null;
      }else if(selected){
        filterConceptId=selected;
      }
      if(filterConceptId&&!ref.candidates.some(c=>c.id===filterConceptId))throw new Error(`Panel ${panelIndex+1} selected a Concept that was not offered by the preview.`);
      if(filterConceptId&&!concepts.some(c=>c.id===filterConceptId))throw new Error(`Panel ${panelIndex+1} references a Concept outside this Player world.`);
      if(filterConceptId)resolvedConcepts++;else unresolvedConcepts++;
    }
    const {relatedConcept:_portableReference,...configuration}=panel;
    return {...configuration,filterConceptId};
  });
  return {workspace:preview.portable.workspace,panels,resolvedConcepts,unresolvedConcepts};
}

export function exportWorkspace(workspace: Workspace, panels: WorkspacePanel[], concepts: Concept[]): PortableWorkspace {
  const byId=new Map(concepts.filter(c=>c.playerId===workspace.playerId).map(c=>[c.id,c]));
  const portablePanels=panels.map(({panelType,title,variant,density,filterStatus,filterActive,filterTypeCode,filterConceptId,filterRecentDays,sortBy,itemLimit,sortOrder,gridSpan,isVisible,isPinned,isCollapsed})=>{
    const concept=filterConceptId?byId.get(filterConceptId):undefined;
    if(filterConceptId&&!concept)throw new Error('A panel Concept filter is not available in this workspace Player world.');
    return {panelType,title,variant,density,filterStatus,filterActive,filterTypeCode,relatedConcept:concept?{key:concept.transferKey,name:concept.name,typeCode:concept.typeCode}:null,filterRecentDays,sortBy,itemLimit,sortOrder,gridSpan,isVisible,isPinned,isCollapsed};
  });
  const stablePanels=portablePanels.sort((a,b)=>{
    const order=a.sortOrder-b.sortOrder;
    if(order!==0)return order;
    const left=JSON.stringify(a),right=JSON.stringify(b);
    return left<right?-1:left>right?1:0;
  });
  return validateV2({format:'life-rpg-workspace',version:2,workspace:{name:workspace.name,template:workspace.template},panels:stablePanels});
}
