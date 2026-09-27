import type { Workspace, WorkspacePanel, WorkspacePanelType, WorkspacePanelVariant, WorkspacePanelSort, WorkspaceTemplate } from '../../domain/world';
import { PANEL_REGISTRY } from './panelRegistry';

export type PortablePanel = Pick<WorkspacePanel,
  'panelType'|'title'|'variant'|'density'|'filterStatus'|'filterActive'|'filterTypeCode'|'filterConceptId'|'filterRecentDays'|'sortBy'|'itemLimit'|'sortOrder'|'gridSpan'|'isVisible'|'isPinned'|'isCollapsed'>;
export type PortableWorkspace = {
  format: 'life-rpg-workspace';
  version: 1;
  workspace: { name: string; template: WorkspaceTemplate };
  panels: PortablePanel[];
};

const record = (value: unknown): value is Record<string, unknown> => !!value && typeof value === 'object' && !Array.isArray(value);
const exactKeys = (value: Record<string, unknown>, keys: string[]) => Object.keys(value).every(k => keys.includes(k)) && keys.every(k => Object.hasOwn(value,k));
const nullableString = (v: unknown, max=160): v is string|null => v === null || (typeof v === 'string' && v.length > 0 && v.length <= max);
const templates: WorkspaceTemplate[] = ['overview','focus','learning','health','review','custom'];

export function validateWorkspaceImport(value: unknown): PortableWorkspace {
  if (!record(value) || !exactKeys(value,['format','version','workspace','panels']) || value.format !== 'life-rpg-workspace' || value.version !== 1 || !record(value.workspace) || !exactKeys(value.workspace,['name','template']) || typeof value.workspace.name !== 'string' || !value.workspace.name.trim() || value.workspace.name.trim().length > 80 || typeof value.workspace.template !== 'string' || !templates.includes(value.workspace.template as WorkspaceTemplate) || !Array.isArray(value.panels) || value.panels.length > 100) throw new Error('This file is not a supported Life RPG workspace export.');
  const allowed = ['panelType','title','variant','density','filterStatus','filterActive','filterTypeCode','filterConceptId','filterRecentDays','sortBy','itemLimit','sortOrder','gridSpan','isVisible','isPinned','isCollapsed'];
  const panels = value.panels.map((raw, i): PortablePanel => {
    if (!record(raw) || !exactKeys(raw,allowed)) throw new Error(`Panel ${i+1} has missing or executable/unknown configuration fields.`);
    const type = raw.panelType as WorkspacePanelType;
    const def = PANEL_REGISTRY[type];
    if (!def || typeof raw.variant !== 'string' || !def.variants.includes(raw.variant as WorkspacePanelVariant) || (raw.density !== 'cozy' && raw.density !== 'compact') || (typeof raw.title !== 'string' && raw.title !== null) || (typeof raw.title === 'string' && (!raw.title.trim() || raw.title.length > 120)) || (raw.filterStatus !== null && (typeof raw.filterStatus !== 'string' || !def.statuses.includes(raw.filterStatus))) || (raw.filterActive !== null && typeof raw.filterActive !== 'boolean') || (raw.filterTypeCode !== null && (!nullableString(raw.filterTypeCode) || !/^[a-z0-9_]{1,160}$/.test(raw.filterTypeCode))) || (raw.filterConceptId !== null && !nullableString(raw.filterConceptId)) || (raw.filterRecentDays !== null && (!Number.isInteger(raw.filterRecentDays) || (raw.filterRecentDays as number)<1 || (raw.filterRecentDays as number)>365)) || typeof raw.sortBy !== 'string' || !def.sorts.includes(raw.sortBy as WorkspacePanelSort) || !Number.isInteger(raw.itemLimit) || (raw.itemLimit as number)<1 || (raw.itemLimit as number)>50 || !Number.isInteger(raw.sortOrder) || (raw.sortOrder as number)<0 || (raw.sortOrder as number)>999 || ![1,2].includes(raw.gridSpan as number) || typeof raw.isVisible !== 'boolean' || typeof raw.isPinned !== 'boolean' || typeof raw.isCollapsed !== 'boolean') throw new Error(`Panel ${i+1} contains a value not allowed for its source.`);
    if ((!def.supportsActive && raw.filterActive !== null) || (!def.supportsType && raw.filterTypeCode !== null) || (!def.supportsConcept && raw.filterConceptId !== null) || (!def.supportsRecent && raw.filterRecentDays !== null)) throw new Error(`Panel ${i+1} uses a filter unsupported by its source.`);
    return raw as unknown as PortablePanel;
  });
  return {format:'life-rpg-workspace',version:1,workspace:{name:(value.workspace.name as string).trim(),template:value.workspace.template as WorkspaceTemplate},panels};
}

export function exportWorkspace(workspace: Workspace, panels: WorkspacePanel[]): PortableWorkspace {
  const value: PortableWorkspace = {format:'life-rpg-workspace',version:1,workspace:{name:workspace.name,template:workspace.template},panels:panels.map(({panelType,title,variant,density,filterStatus,filterActive,filterTypeCode,filterConceptId,filterRecentDays,sortBy,itemLimit,sortOrder,gridSpan,isVisible,isPinned,isCollapsed})=>({panelType,title,variant,density,filterStatus,filterActive,filterTypeCode,filterConceptId,filterRecentDays,sortBy,itemLimit,sortOrder,gridSpan,isVisible,isPinned,isCollapsed}))};
  return validateWorkspaceImport(value);
}
