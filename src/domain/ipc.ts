/**
 * The IPC boundary, wrapped and typed.
 *
 * Every call into the Rust core goes through this module. Nothing else in the
 * app imports `invoke` directly. Two reasons that matters:
 *
 * 1. **One place to type the boundary.** `invoke` returns `unknown`; this module
 *    is where that `unknown` becomes a `HealthReport`. If the contract changes,
 *    it changes here and the compiler tells us what else broke.
 * 2. **One place to test the boundary.** `src/test/ipc.test.ts` stubs
 *    `__TAURI_INTERNALS__` and exercises the real code paths, including the
 *    failure paths, without booting a webview.
 *
 * Command names are declared once in `COMMANDS` so a typo is a compile error
 * rather than a runtime 404-style rejection.
 */

import { invoke as tauriInvoke } from '@tauri-apps/api/core';
import type { CommandError, HealthReport } from '../domain/health';
import { toCommandError } from '../domain/health';
import type {
  AwardXpOutcome, Comment, Effect, NarrativeEntry, Player, PlayerSnapshot, PlayerStat, Quest, Rule,
  RuleDefinition, RuleExecution, Skill, SkillSnapshot, SkillTree, StatDefinition, Transaction,
  WorldOverview, Concept, ConceptProgressTrack, ConceptRelationship, QuestStage, QuestBranch,
  QuestSession, ContentAttachment, ConceptAssociation, EntityRevision, PresentationPreference,
  ProgressSuggestion, SearchHit, SearchQuery,
  Workspace, WorkspacePanel, WorkspaceTemplate, WorkspacePanelType,
} from '../domain/world';

/** Command names exposed by `src-tauri/src/commands/`. */
export const COMMANDS = {
  getStatus: 'get_status',
  getWorldLocation: 'get_world_location',
  ping: 'ping',
  createPlayer: 'create_player',
  getPlayer: 'get_player',
  awardXp: 'award_xp',
  createQuest: 'create_quest',
  startQuest: 'start_quest',
  completeQuest: 'complete_quest',
  createSkillTree: 'create_skill_tree',
  addSkill: 'add_skill',
  investSkillTime: 'invest_skill_time',
  capturePlayerSnapshot: 'capture_player_snapshot',
  addComment: 'add_comment',
  listComments: 'list_comments',
  writeNarrative: 'write_narrative',
  getWorldOverview: 'get_world_overview',
  listTransactions: 'list_transactions',
  captureSkillSnapshot: 'capture_skill_snapshot',
  listPlayerSnapshots: 'list_player_snapshots',
  listSkillSnapshots: 'list_skill_snapshots',
  defineStat: 'define_stat',
  listStatDefinitions: 'list_stat_definitions',
  setPlayerStat: 'set_player_stat',
  listPlayerStats: 'list_player_stats',
  deactivateEffect: 'deactivate_effect',
  listRules: 'list_rules',
  createRule: 'create_rule',
  setRuleEnabled: 'set_rule_enabled',
  listRuleExecutions: 'list_rule_executions',
  listConcepts: 'list_concepts',
  createConcept: 'create_concept',
  listConceptProgress: 'list_concept_progress',
  setConceptProgress: 'set_concept_progress',
  listConceptRelationships: 'list_concept_relationships',
  listConceptRelationshipTypes: 'list_concept_relationship_types',
  relateConcepts: 'relate_concepts',
  searchWorld: 'search_world',
  listPresentationPreferences: 'list_presentation_preferences',
  setPresentationPreference: 'set_presentation_preference',
  setPresentationVisibility: 'set_presentation_visibility',
  listEntityRevisions: 'list_entity_revisions',
  restoreEntityRevision: 'restore_entity_revision',
  getEntityLifecycle: 'get_entity_lifecycle',
  setEntityLifecycle: 'set_entity_lifecycle',
  createQuestStage: 'create_quest_stage',
  listQuestStages: 'list_quest_stages',
  createQuestBranch: 'create_quest_branch',
  listQuestBranches: 'list_quest_branches',
  startQuestSession: 'start_quest_session',
  finishQuestSession: 'finish_quest_session',
  listQuestSessions: 'list_quest_sessions',
  listAttachedContent: 'list_attached_content',
  attachContent: 'attach_content',
  listConceptAssociations: 'list_concept_associations',
  associateConcept: 'associate_concept',
  listProgressSuggestions: 'list_progress_suggestions',
  acceptProgressSuggestion: 'accept_progress_suggestion',
  rejectProgressSuggestion: 'reject_progress_suggestion',
  setPlayerProgression: 'set_player_progression',
  setSkillProgression: 'set_skill_progression',
  setConceptProgressControl: 'set_concept_progress_control',
  createWorkspace: 'create_workspace', listWorkspaces: 'list_workspaces', renameWorkspace: 'rename_workspace', deleteWorkspace: 'delete_workspace',
  listWorkspacePanels: 'list_workspace_panels', saveWorkspacePanel: 'save_workspace_panel', deleteWorkspacePanel: 'delete_workspace_panel',
} as const;

export type CommandName = (typeof COMMANDS)[keyof typeof COMMANDS];

type QuestSessionContextFields = {
  questId?: string;
  stageId?: string;
  branchId?: string;
  skillId?: string;
  conceptId?: string;
};

/** A Session must be anchored to at least one world entity. */
export type QuestSessionContext =
  | ({questId: string} & Omit<QuestSessionContextFields, 'questId'>)
  | ({stageId: string} & Omit<QuestSessionContextFields, 'stageId'>)
  | ({branchId: string} & Omit<QuestSessionContextFields, 'branchId'>)
  | ({skillId: string} & Omit<QuestSessionContextFields, 'skillId'>)
  | ({conceptId: string} & Omit<QuestSessionContextFields, 'conceptId'>);

/** Injectable transport so tests can drive the API without a webview. */
export type InvokeTransport = <T>(command: string, args?: Record<string, unknown>) => Promise<T>;

const defaultTransport: InvokeTransport = async <T,>(
  command: string,
  args?: Record<string, unknown>,
): Promise<T> => {
  // `invoke` is untyped at the boundary on purpose; this is the single place
  // where an `unknown` payload becomes a contract type.
  return (await tauriInvoke(command, args)) as T;
};

/**
 * The typed client for the Life RPG core.
 *
 * Construct one with the default transport in the app; construct one with a
 * fake in tests.
 */
export class CoreClient {
  private readonly transport: InvokeTransport;

  constructor(transport: InvokeTransport = defaultTransport) {
    this.transport = transport;
  }

  /** Liveness check that never touches storage. */
  async ping(): Promise<string> {
    return this.invoke<string>(COMMANDS.ping);
  }

  /** Full status, including a real persistence round trip. */
  async getStatus(): Promise<HealthReport> {
    return this.invoke<HealthReport>(COMMANDS.getStatus);
  }

  /** Path of the world database on disk, if file-backed. */
  async getWorldLocation(): Promise<string | null> {
    return this.invoke<string | null>(COMMANDS.getWorldLocation);
  }

  async createPlayer(name: string, description?: string): Promise<Player> {
    return this.invoke<Player>(COMMANDS.createPlayer, { name, description: description ?? null });
  }
  async getPlayer(id: string): Promise<Player | null> { return this.invoke<Player | null>(COMMANDS.getPlayer, { id }); }
  async awardXp(playerId: string, amount: number, reason?: string, description?: string): Promise<AwardXpOutcome> {
    return this.invoke<AwardXpOutcome>(COMMANDS.awardXp, { playerId, amount, reason: reason ?? null, description: description ?? null });
  }
  async createQuest(playerId:string,typeCode:string,title:string,options:{description?:string;parentQuestId?:string;skillId?:string;difficulty?:number;xpReward?:number}={}):Promise<Quest>{
    return this.invoke<Quest>(COMMANDS.createQuest,{playerId,typeCode,title,description:options.description??null,parentQuestId:options.parentQuestId??null,skillId:options.skillId??null,difficulty:options.difficulty??null,xpReward:options.xpReward??null});
  }
  async startQuest(questId:string):Promise<Quest>{return this.invoke<Quest>(COMMANDS.startQuest,{questId});}
  async completeQuest(questId:string):Promise<Quest>{return this.invoke<Quest>(COMMANDS.completeQuest,{questId});}
  async createSkillTree(playerId:string,typeCode:string,name:string):Promise<SkillTree>{return this.invoke<SkillTree>(COMMANDS.createSkillTree,{playerId,typeCode,name});}
  async addSkill(treeId:string,typeCode:string,name:string,parentSkillId?:string):Promise<Skill>{return this.invoke<Skill>(COMMANDS.addSkill,{treeId,typeCode,name,parentSkillId:parentSkillId??null});}
  async investSkillTime(skillId:string,minutes:number):Promise<Skill>{return this.invoke<Skill>(COMMANDS.investSkillTime,{skillId,minutes});}
  async capturePlayerSnapshot(playerId:string):Promise<PlayerSnapshot>{return this.invoke<PlayerSnapshot>(COMMANDS.capturePlayerSnapshot,{playerId});}
  async addComment(targetKind:string,targetId:string,body:string,authorPlayerId?:string):Promise<Comment>{return this.invoke<Comment>(COMMANDS.addComment,{authorPlayerId:authorPlayerId??null,targetKind,targetId,body});}
  async listComments(targetKind:string,targetId:string):Promise<Comment[]>{return this.invoke<Comment[]>(COMMANDS.listComments,{targetKind,targetId});}
  async writeNarrative(playerId:string,kind:string,title:string,content:string):Promise<NarrativeEntry>{return this.invoke<NarrativeEntry>(COMMANDS.writeNarrative,{playerId,kind,title,content});}
  async getWorldOverview(playerId:string):Promise<WorldOverview>{return this.invoke<WorldOverview>(COMMANDS.getWorldOverview,{playerId});}
  async listTransactions(playerId:string,limit?:number):Promise<Transaction[]>{return this.invoke<Transaction[]>(COMMANDS.listTransactions,{playerId,limit:limit??null});}
  async listConcepts(playerId:string):Promise<Concept[]>{return this.invoke<Concept[]>(COMMANDS.listConcepts,{playerId});}
  async createConcept(playerId:string,typeCode:string,name:string,description?:string):Promise<Concept>{return this.invoke<Concept>(COMMANDS.createConcept,{playerId,typeCode,name,description:description??null});}
  async listConceptProgress(conceptId:string):Promise<ConceptProgressTrack[]>{return this.invoke<ConceptProgressTrack[]>(COMMANDS.listConceptProgress,{conceptId});}
  async setConceptProgress(conceptId:string,trackCode:string,value:number,level?:number):Promise<ConceptProgressTrack>{return this.invoke<ConceptProgressTrack>(COMMANDS.setConceptProgress,{conceptId,trackCode,value,level:level??null,occurredAt:null});}
  async listConceptRelationships(conceptId:string):Promise<ConceptRelationship[]>{return this.invoke<ConceptRelationship[]>(COMMANDS.listConceptRelationships,{conceptId});}
  async listConceptRelationshipTypes():Promise<string[]>{return this.invoke<string[]>(COMMANDS.listConceptRelationshipTypes);}
  async relateConcepts(sourceConceptId:string,targetConceptId:string,relationshipCode:string):Promise<ConceptRelationship>{return this.invoke<ConceptRelationship>(COMMANDS.relateConcepts,{sourceConceptId,targetConceptId,relationshipCode});}
  async searchWorld(query:SearchQuery):Promise<SearchHit[]>{return this.invoke<SearchHit[]>(COMMANDS.searchWorld,{query});}
  async listPresentationPreferences(playerId:string,context:string):Promise<PresentationPreference[]>{return this.invoke<PresentationPreference[]>(COMMANDS.listPresentationPreferences,{playerId,context});}
  async createWorkspace(playerId:string,name:string,template:WorkspaceTemplate,isDefault=false):Promise<Workspace>{return this.invoke(COMMANDS.createWorkspace,{playerId,name,template,isDefault});}
  async listWorkspaces(playerId:string):Promise<Workspace[]>{return this.invoke(COMMANDS.listWorkspaces,{playerId});}
  async renameWorkspace(playerId:string,workspaceId:string,name:string):Promise<void>{return this.invoke(COMMANDS.renameWorkspace,{playerId,workspaceId,name});}
  async deleteWorkspace(playerId:string,workspaceId:string):Promise<void>{return this.invoke(COMMANDS.deleteWorkspace,{playerId,workspaceId});}
  async listWorkspacePanels(playerId:string,workspaceId:string):Promise<WorkspacePanel[]>{return this.invoke(COMMANDS.listWorkspacePanels,{playerId,workspaceId});}
  async saveWorkspacePanel(value:{playerId:string;workspaceId:string;panelId?:string;panelType:WorkspacePanelType;title:string|null;variant:'cards'|'rows';density:'cozy'|'compact';filterStatus:WorkspacePanel['filterStatus'];itemLimit:number;sortOrder:number;isPinned:boolean;isCollapsed:boolean}):Promise<WorkspacePanel>{return this.invoke(COMMANDS.saveWorkspacePanel,{...value,panelId:value.panelId??null});}
  async deleteWorkspacePanel(playerId:string,workspaceId:string,panelId:string):Promise<void>{return this.invoke(COMMANDS.deleteWorkspacePanel,{playerId,workspaceId,panelId});}
  async setPresentationPreference(value:Pick<PresentationPreference,'playerId'|'entityKind'|'entityId'|'context'|'isVisible'|'sortOrder'|'isPinned'|'isCollapsed'|'variant'|'density'>):Promise<PresentationPreference>{return this.invoke<PresentationPreference>(COMMANDS.setPresentationPreference,value);}
  async setPresentationVisibility(value:Pick<PresentationPreference,'playerId'|'entityKind'|'entityId'|'context'|'isVisible'>):Promise<void>{return this.invoke<void>(COMMANDS.setPresentationVisibility,value);}
  async listEntityRevisions(targetKind:string,targetId:string):Promise<EntityRevision[]>{return this.invoke<EntityRevision[]>(COMMANDS.listEntityRevisions,{targetKind,targetId});}
  async restoreEntityRevision(revisionId:string,reason?:string):Promise<EntityRevision>{return this.invoke<EntityRevision>(COMMANDS.restoreEntityRevision,{revisionId,reason:reason??null});}
  async getEntityLifecycle(targetKind:string,targetId:string):Promise<{state:'active'|'archived'|'trashed'}>{return this.invoke(COMMANDS.getEntityLifecycle,{targetKind,targetId});}
  async setEntityLifecycle(targetKind:string,targetId:string,playerId:string,lifecycle:'active'|'archived'|'trashed',reason?:string):Promise<void>{return this.invoke(COMMANDS.setEntityLifecycle,{targetKind,targetId,playerId,lifecycle,reason:reason??null});}
  async listQuestStages(questId:string):Promise<QuestStage[]>{return this.invoke(COMMANDS.listQuestStages,{questId});}
  async createQuestStage(playerId:string,questId:string,title:string,sortOrder:number):Promise<QuestStage>{return this.invoke(COMMANDS.createQuestStage,{playerId,questId,title,sortOrder});}
  async listQuestBranches(stageId:string):Promise<QuestBranch[]>{return this.invoke(COMMANDS.listQuestBranches,{stageId});}
  async createQuestBranch(stageId:string,title:string,sortOrder:number):Promise<QuestBranch>{return this.invoke(COMMANDS.createQuestBranch,{stageId,title,sortOrder});}
  async listQuestSessions(playerId:string,questId?:string,stageId?:string):Promise<QuestSession[]>{return this.invoke(COMMANDS.listQuestSessions,{playerId,questId:questId??null,stageId:stageId??null});}
  async startQuestSession(playerId:string,context:QuestSessionContext):Promise<QuestSession>{return this.invoke(COMMANDS.startQuestSession,{playerId,questId:context.questId??null,stageId:context.stageId??null,branchId:context.branchId??null,skillId:context.skillId??null,conceptId:context.conceptId??null,startedAt:null});}
  async finishQuestSession(sessionId:string,status:'completed'|'interrupted',result?:string,notes?:string):Promise<QuestSession>{return this.invoke(COMMANDS.finishQuestSession,{sessionId,endedAt:null,status,result:result??null,notes:notes??null});}
  async listAttachedContent(targetKind:string,targetId:string):Promise<ContentAttachment[]>{return this.invoke(COMMANDS.listAttachedContent,{targetKind,targetId});}
  async attachContent(playerId:string,contentId:string,targetKind:string,targetId:string,role:string):Promise<ContentAttachment>{return this.invoke(COMMANDS.attachContent,{playerId,contentId,targetKind,targetId,role});}
  async listConceptAssociations(conceptId:string):Promise<ConceptAssociation[]>{return this.invoke(COMMANDS.listConceptAssociations,{conceptId,entityKind:null,entityId:null});}
  async associateConcept(conceptId:string,entityKind:string,entityId:string,role:string):Promise<ConceptAssociation>{return this.invoke(COMMANDS.associateConcept,{conceptId,entityKind,entityId,role});}
  async listProgressSuggestions(conceptId:string,includeResolved=false):Promise<ProgressSuggestion[]>{return this.invoke(COMMANDS.listProgressSuggestions,{conceptId,includeResolved});}
  async acceptProgressSuggestion(playerId:string,suggestionId:string):Promise<ProgressSuggestion>{return this.invoke(COMMANDS.acceptProgressSuggestion,{playerId,suggestionId});}
  async rejectProgressSuggestion(playerId:string,suggestionId:string):Promise<ProgressSuggestion>{return this.invoke(COMMANDS.rejectProgressSuggestion,{playerId,suggestionId});}
  async setPlayerProgression(playerId:string,level:number,levelName?:string,progressionLabel?:string):Promise<Player>{return this.invoke(COMMANDS.setPlayerProgression,{playerId,level,levelName:levelName??null,progressionLabel:progressionLabel??null});}
  async setSkillProgression(skillId:string,level:number,levelName?:string,progressionLabel?:string):Promise<Skill>{return this.invoke(COMMANDS.setSkillProgression,{skillId,level,levelName:levelName??null,progressionLabel:progressionLabel??null});}
  async setConceptProgressControl(conceptId:string,trackCode:string,control:'manual'|'rule_controlled'):Promise<ConceptProgressTrack>{return this.invoke(COMMANDS.setConceptProgressControl,{conceptId,trackCode,control});}
  async captureSkillSnapshot(skillId:string):Promise<SkillSnapshot>{return this.invoke<SkillSnapshot>(COMMANDS.captureSkillSnapshot,{skillId});}
  async listPlayerSnapshots(playerId:string):Promise<PlayerSnapshot[]>{return this.invoke<PlayerSnapshot[]>(COMMANDS.listPlayerSnapshots,{playerId});}
  async listSkillSnapshots(skillId:string):Promise<SkillSnapshot[]>{return this.invoke<SkillSnapshot[]>(COMMANDS.listSkillSnapshots,{skillId});}
  async defineStat(code:string,name:string,options:{description?:string;unit?:string;minimum?:number;maximum?:number}={}):Promise<StatDefinition>{return this.invoke<StatDefinition>(COMMANDS.defineStat,{code,name,description:options.description??null,unit:options.unit??null,minimum:options.minimum??null,maximum:options.maximum??null});}
  async listStatDefinitions():Promise<StatDefinition[]>{return this.invoke<StatDefinition[]>(COMMANDS.listStatDefinitions);}
  async setPlayerStat(playerId:string,statCode:string,value:number):Promise<PlayerStat>{return this.invoke<PlayerStat>(COMMANDS.setPlayerStat,{playerId,statCode,value});}
  async listPlayerStats(playerId:string):Promise<PlayerStat[]>{return this.invoke<PlayerStat[]>(COMMANDS.listPlayerStats,{playerId});}
  async deactivateEffect(effectId:string):Promise<Effect>{return this.invoke<Effect>(COMMANDS.deactivateEffect,{effectId});}
  async listRules():Promise<Rule[]>{return this.invoke<Rule[]>(COMMANDS.listRules);}
  async createRule(name:string,priority:number,definition:RuleDefinition,description?:string):Promise<Rule>{return this.invoke<Rule>(COMMANDS.createRule,{name,description:description??null,priority,definition});}
  async setRuleEnabled(ruleId:string,enabled:boolean):Promise<Rule>{return this.invoke<Rule>(COMMANDS.setRuleEnabled,{ruleId,enabled});}
  async listRuleExecutions(limit=100):Promise<RuleExecution[]>{return this.invoke<RuleExecution[]>(COMMANDS.listRuleExecutions,{limit});}

  private async invoke<T>(command: CommandName, args?: Record<string, unknown>): Promise<T> {
    try {
      return await this.transport<T>(command, args);
    } catch (error) {
      // Normalize everything into a CommandError so callers have exactly one
      // failure shape to handle. The original value is preserved on `cause`
      // for the console.
      const normalized: CommandError = toCommandError(error);
      const wrapped = new Error(normalized.message) as Error & { code: string; cause: unknown };
      wrapped.code = normalized.code;
      wrapped.cause = error;
      throw wrapped;
    }
  }
}

/** Convenience singleton for app code. Tests should build their own client. */
export const coreClient = new CoreClient();
