/** Hand-mirrored Phase 2 world DTOs from `lr-contracts/src/world.rs`. */
export type Player={id:string;name:string;description:string|null;level:number;currentXp:number;isActive:boolean;metadataJson:string;createdAt:string;updatedAt:string};
export type Quest={id:string;playerId:string;typeCode:string;parentQuestId:string|null;skillId:string|null;title:string;status:string;difficulty:number|null;progress:number;xpReward:number;dueAt:string|null;startedAt:string|null;completedAt:string|null;description:string|null};
export type SkillTree={id:string;playerId:string;typeCode:string;name:string;description:string|null;isActive:boolean};
export type Skill={id:string;skillTreeId:string;parentSkillId:string|null;typeCode:string;name:string;level:number;currentXp:number;investedMinutes:number;status:string};
export type Effect={id:string;playerId:string;typeCode:string;name:string;description:string|null;startedAt:string;expiresAt:string|null;deactivatedAt:string|null;intensity:number};
export type Transaction={id:number|null;playerId:string;typeCode:string;resource:string;amount:number;appliedAmount:number|null;occurredAt:string;reason:string|null;description:string|null;sourceKind:string|null;sourceId:string|null};
export type PlayerSnapshot={id:number|null;playerId:string;snapshotDate:string;level:number;currentXp:number;stateJson:string;createdAt:string};
export type SkillSnapshot={id:number|null;skillId:string;snapshotDate:string;level:number;currentXp:number;status:string;investedMinutes:number;stateJson:string;createdAt:string};
export type StatDefinition={id:string;code:string;name:string;description:string|null;unit:string|null;minimum:number|null;maximum:number|null;isActive:boolean};
export type PlayerStat={playerId:string;statCode:string;currentValue:number;updatedAt:string};
export type Comment={id:number|null;authorPlayerId:string|null;targetKind:string;targetId:string;body:string;createdAt:string};
export type NarrativeEntry={id:string;playerId:string;kind:string;title:string;content:string;author:string|null;createdAt:string};
export type AwardXpOutcome={player:Player;transaction:Transaction};
export type WorldOverview={player:Player;quests:Quest[];skillTrees:SkillTree[];skills:Skill[];effects:Effect[];recentTransactions:Transaction[];narratives:NarrativeEntry[]};
export const XP_PER_LEVEL=1000;
export const xpProgress=(player:Player):number=>((player.currentXp%XP_PER_LEVEL)+XP_PER_LEVEL)%XP_PER_LEVEL/XP_PER_LEVEL;
