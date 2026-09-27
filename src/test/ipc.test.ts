/**
 * Tests for the typed IPC layer.
 *
 * The point of these tests is the *boundary*, not the components: command names
 * must be exactly what the Rust `generate_handler!` list registers, and every
 * failure must be normalized into one shape. A typo'd command name or a lost
 * error code is the kind of bug that only shows up at runtime in a packaged app,
 * so it is pinned here.
 */

import { describe, expect, it, vi } from 'vitest';

import { COMMANDS, CoreClient, type InvokeTransport, type QuestSessionContext } from '../domain/ipc';
import { toCommandError } from '../domain/health';
import { failedReport, healthyReport } from './fixtures';

describe('COMMANDS', () => {
  it('matches the Rust command registrations exactly', () => {
    // Keep this list in sync with `invoke_handler(tauri::generate_handler![...])`
    // in src-tauri/src/lib.rs. snake_case on the wire, camelCase in TS.
    expect(COMMANDS.getStatus).toBe('get_status');
    expect(COMMANDS.getWorldLocation).toBe('get_world_location');
    expect(COMMANDS.ping).toBe('ping');
    expect(COMMANDS.createPlayer).toBe('create_player');
    expect(COMMANDS.awardXp).toBe('award_xp');
    expect(COMMANDS.createQuest).toBe('create_quest');
    expect(COMMANDS.completeQuest).toBe('complete_quest');
    expect(COMMANDS.createSkillTree).toBe('create_skill_tree');
    expect(COMMANDS.capturePlayerSnapshot).toBe('capture_player_snapshot');
    expect(COMMANDS.getWorldOverview).toBe('get_world_overview');
    expect(COMMANDS.listConcepts).toBe('list_concepts');
    expect(COMMANDS.listConceptRelationshipTypes).toBe('list_concept_relationship_types');
    expect(COMMANDS.relateConcepts).toBe('relate_concepts');
    expect(COMMANDS.searchWorld).toBe('search_world');
    expect(COMMANDS.listEntityRevisions).toBe('list_entity_revisions');
    expect(COMMANDS.setPresentationPreference).toBe('set_presentation_preference');
    expect(COMMANDS.setPresentationVisibility).toBe('set_presentation_visibility');
    expect(COMMANDS.startQuestSession).toBe('start_quest_session');
    expect(COMMANDS.createWorkspace).toBe('create_workspace');
    expect(COMMANDS.setDefaultWorkspace).toBe('set_default_workspace');
    expect(COMMANDS.listWorkspacePanels).toBe('list_workspace_panels');
    expect(COMMANDS.saveWorkspacePanel).toBe('save_workspace_panel');
  });
});

describe('CoreClient', () => {
	 it('requires a valid world anchor in the Session context type', () => {
	   const validContext: QuestSessionContext = {questId: 'quest-1'};
	   expect(validContext.questId).toBe('quest-1');
	   // @ts-expect-error A context-free Session is not representable at the UI boundary.
	   const invalidContext: QuestSessionContext = {};
	   expect(invalidContext).toEqual({});
	 });

	 it('passes a Quest description and a visibility-only preference update through the typed boundary', async () => {
	   const transport = vi.fn(async () => ({})) as unknown as InvokeTransport;
	   const client = new CoreClient(transport);
	   await client.createQuest('player-1', 'main', 'Prepare the garden', {description: 'Prepare the soil.'});
	   expect(transport).toHaveBeenLastCalledWith('create_quest', expect.objectContaining({description: 'Prepare the soil.'}));
	   await client.setPresentationVisibility({playerId: 'player-1', entityKind: 'quest', entityId: 'quest-1', context: 'dashboard', isVisible: false});
	   expect(transport).toHaveBeenLastCalledWith('set_presentation_visibility', {playerId: 'player-1', entityKind: 'quest', entityId: 'quest-1', context: 'dashboard', isVisible: false});
	 });

  it('passes the command name through and returns the payload', async () => {
    const transport = vi.fn(async () => healthyReport()) as unknown as InvokeTransport;
    const client = new CoreClient(transport);

    const report = await client.getStatus();

    expect(report.status).toBe('ok');
    expect(transport).toHaveBeenCalledWith('get_status', undefined);
  });

  it('passes Player-scoped declarative panel settings through the typed boundary', async () => {
    const transport=vi.fn(async()=>({})) as unknown as InvokeTransport;
    const client=new CoreClient(transport);
    await client.saveWorkspacePanel({playerId:'player-1',workspaceId:'workspace-1',panelId:'panel-1',panelType:'quests',title:'Open objectives',variant:'rows',density:'compact',filterStatus:'active',filterActive:null,filterTypeCode:'main',filterConceptId:'concept-1',filterRecentDays:30,sortBy:'updated_desc',itemLimit:8,sortOrder:2,gridSpan:2,isVisible:true,isPinned:true,isCollapsed:false});
    expect(transport).toHaveBeenCalledWith('save_workspace_panel',expect.objectContaining({playerId:'player-1',workspaceId:'workspace-1',panelType:'quests',filterStatus:'active',filterTypeCode:'main',filterConceptId:'concept-1',filterRecentDays:30,sortBy:'updated_desc',itemLimit:8,gridSpan:2,isVisible:true}));
    await client.setDefaultWorkspace('player-1','workspace-1');
    expect(transport).toHaveBeenLastCalledWith('set_default_workspace',{playerId:'player-1',workspaceId:'workspace-1'});
  });
  it('returns null for an in-memory world location', async () => {
    const transport = vi.fn(async () => null) as unknown as InvokeTransport;
    const client = new CoreClient(transport);
    await expect(client.getWorldLocation()).resolves.toBeNull();
  });

  it('normalizes structured CommandError payloads', async () => {
    const transport = vi.fn(async () => {
      throw { code: 'storage_unreachable', message: 'store is not reachable: disk gone' };
    }) as unknown as InvokeTransport;
    const client = new CoreClient(transport);

    await expect(client.getStatus()).rejects.toMatchObject({
      code: 'storage_unreachable',
      message: 'store is not reachable: disk gone',
    });
  });

  it('normalizes plain Error values into the same shape', async () => {
    const transport = vi.fn(async () => {
      throw new Error('the core is not listening');
    }) as unknown as InvokeTransport;
    const client = new CoreClient(transport);

    await expect(client.getStatus()).rejects.toMatchObject({
      code: 'unknown_error',
      message: 'the core is not listening',
    });
  });

  it('preserves the original thrown value on the wrapped error', async () => {
    const original = { code: 'x', message: 'y' };
    const transport = vi.fn(async () => {
      throw original;
    }) as unknown as InvokeTransport;
    const client = new CoreClient(transport);

    try {
      await client.ping();
      throw new Error('should have thrown');
    } catch (err) {
      expect((err as { cause: unknown }).cause).toEqual(original);
    }
  });
});

describe('toCommandError', () => {
  it('accepts a well-formed payload', () => {
    expect(toCommandError({ code: 'a', message: 'b' })).toEqual({ code: 'a', message: 'b' });
  });

  it('falls back for primitives, null and malformed objects', () => {
    expect(toCommandError('boom').code).toBe('unknown_error');
    expect(toCommandError(undefined).code).toBe('unknown_error');
    expect(toCommandError({ code: 7 }).code).toBe('unknown_error');
  });
});

describe('failed report fixture', () => {
  it('represents an unreachable store without a database or round trip', () => {
    const report = failedReport();
    expect(report.status).toBe('failed');
    expect(report.database).toBeNull();
    expect(report.roundTrip).toBeNull();
    expect(report.problems).toHaveLength(3);
  });
});
