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
  AwardXpOutcome, Comment, NarrativeEntry, Player, PlayerSnapshot, Quest, Skill,
  SkillTree, Transaction, WorldOverview,
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
  writeNarrative: 'write_narrative',
  getWorldOverview: 'get_world_overview',
  listTransactions: 'list_transactions',
} as const;

export type CommandName = (typeof COMMANDS)[keyof typeof COMMANDS];

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
  async createQuest(playerId:string,typeCode:string,title:string,options:{parentQuestId?:string;skillId?:string;difficulty?:number;xpReward?:number}={}):Promise<Quest>{
    return this.invoke<Quest>(COMMANDS.createQuest,{playerId,typeCode,title,parentQuestId:options.parentQuestId??null,skillId:options.skillId??null,difficulty:options.difficulty??null,xpReward:options.xpReward??null});
  }
  async startQuest(questId:string):Promise<Quest>{return this.invoke<Quest>(COMMANDS.startQuest,{questId});}
  async completeQuest(questId:string):Promise<Quest>{return this.invoke<Quest>(COMMANDS.completeQuest,{questId});}
  async createSkillTree(playerId:string,typeCode:string,name:string):Promise<SkillTree>{return this.invoke<SkillTree>(COMMANDS.createSkillTree,{playerId,typeCode,name});}
  async addSkill(treeId:string,typeCode:string,name:string,parentSkillId?:string):Promise<Skill>{return this.invoke<Skill>(COMMANDS.addSkill,{treeId,typeCode,name,parentSkillId:parentSkillId??null});}
  async investSkillTime(skillId:string,minutes:number):Promise<Skill>{return this.invoke<Skill>(COMMANDS.investSkillTime,{skillId,minutes});}
  async capturePlayerSnapshot(playerId:string):Promise<PlayerSnapshot>{return this.invoke<PlayerSnapshot>(COMMANDS.capturePlayerSnapshot,{playerId});}
  async addComment(targetKind:string,targetId:string,body:string,authorPlayerId?:string):Promise<Comment>{return this.invoke<Comment>(COMMANDS.addComment,{authorPlayerId:authorPlayerId??null,targetKind,targetId,body});}
  async writeNarrative(playerId:string,kind:string,title:string,content:string):Promise<NarrativeEntry>{return this.invoke<NarrativeEntry>(COMMANDS.writeNarrative,{playerId,kind,title,content});}
  async getWorldOverview(playerId:string):Promise<WorldOverview>{return this.invoke<WorldOverview>(COMMANDS.getWorldOverview,{playerId});}
  async listTransactions(playerId:string,limit?:number):Promise<Transaction[]>{return this.invoke<Transaction[]>(COMMANDS.listTransactions,{playerId,limit:limit??null});}

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
