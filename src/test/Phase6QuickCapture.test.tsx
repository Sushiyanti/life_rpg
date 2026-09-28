import { afterEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import type { CoreClient } from '../domain/ipc';
import type { Concept, Player, Quest, QuestSession, Skill, SkillTree } from '../domain/world';
import { QuickCapture } from '../features/world/QuickCapture';

afterEach(() => { cleanup(); vi.restoreAllMocks(); });
const player: Player = { id: 'player-1', name: 'Rin', description: null, level: 1, levelName: null, progressionLabel: null, currentXp: 0, isActive: true, metadataJson: '{}', createdAt: '2026-09-27T00:00:00Z', updatedAt: '2026-09-27T00:00:00Z' };
const quest: Quest = { id: 'quest-1', playerId: player.id, typeCode: 'main', parentQuestId: null, skillId: null, title: 'Write the outline', status: 'active', difficulty: null, progress: 0, xpReward: 0, dueAt: null, startedAt: null, completedAt: null, description: null };
const tree: SkillTree = { id: 'tree-1', playerId: player.id, typeCode: 'life', name: 'Learning', description: null, isActive: true };
const skill: Skill = { id: 'skill-1', skillTreeId: tree.id, parentSkillId: null, typeCode: 'core', name: 'Writing', story: null, instructions: null, level: 1, levelName: null, progressionLabel: null, currentXp: 0, investedMinutes: 0, status: 'active', availability: 'available', availabilityControl: 'manual' };
const concept: Concept = { id: 'concept-1', playerId: player.id, transferKey: 'stable-key', typeCode: 'project', name: 'Novel', description: null, isActive: true, metadataJson: '{}', createdAt: '', updatedAt: '' };
const session: QuestSession = { id: 'session-1', playerId: player.id, questId: quest.id, stageId: null, branchId: null, skillId: null, conceptId: null, startedAt: '2026-09-27T10:00:00Z', endedAt: null, status: 'in_progress', progressBefore: null, progressAfter: null, result: null, notes: null, isActive: true, metadataJson: '{}', createdAt: '', updatedAt: '' };
const entry = { id: 'entry-1', playerId: player.id, kind: 'note', title: 'Seedlings', content: 'Water the seedlings on Friday.', author: null, createdAt: '2026-09-27T11:00:00Z' };
function renderCapture(overrides: Record<string, unknown> = {}) {
  const writeNarrative = vi.fn(async () => entry);
  const attachContent = vi.fn(async () => ({ contentId: entry.id }));
  const onSaved = vi.fn(async () => {});
  const client = { writeNarrative, attachContent, ...overrides } as unknown as CoreClient;
  const actualAttach = client.attachContent as unknown as ReturnType<typeof vi.fn>;
  const onClose = vi.fn();
  render(<QuickCapture client={client} player={player} quests={[quest]} skills={[skill]} concepts={[concept]} sessions={[session]} onClose={onClose} onSaved={onSaved} />);
  return { writeNarrative, attachContent: actualAttach, onSaved, onClose };
}

describe('Quick Capture', () => {
  it('saves only the written note and explicitly selected Quest context', async () => {
    const { writeNarrative, attachContent, onSaved } = renderCapture();
    fireEvent.change(screen.getByLabelText('Title optional'), { target: { value: 'Garden note' } });
    fireEvent.change(screen.getByLabelText('What do you want to keep?'), { target: { value: 'Water the seedlings on Friday.' } });
    fireEvent.change(screen.getByLabelText('Attach to context optional'), { target: { value: `quest|${quest.id}` } });
    fireEvent.click(screen.getByRole('button', { name: 'Save entry' }));
    await waitFor(() => expect(writeNarrative).toHaveBeenCalledWith(player.id, 'note', 'Garden note', 'Water the seedlings on Friday.'));
    expect(attachContent).toHaveBeenCalledWith(player.id, entry.id, 'quest', quest.id, 'notes');
    await waitFor(() => expect(onSaved).toHaveBeenCalledTimes(1));
    expect(await screen.findByRole('status')).toHaveTextContent('Saved and attached to Quest · Write the outline.');
  });

  it('preserves a successfully saved note and clearly reports if the optional link fails', async () => {
    const linkError = new Error('Attachment permission rejected');
    const { writeNarrative, attachContent, onSaved } = renderCapture({ attachContent: vi.fn(async () => { throw linkError; }) });
    fireEvent.change(screen.getByLabelText('What do you want to keep?'), { target: { value: 'One useful observation.' } });
    fireEvent.change(screen.getByLabelText('Attach to context optional'), { target: { value: `concept|${concept.id}` } });
    fireEvent.click(screen.getByRole('button', { name: 'Save entry' }));
    await waitFor(() => expect(writeNarrative).toHaveBeenCalled());
    expect(attachContent).toHaveBeenCalledWith(player.id, entry.id, 'concept', concept.id, 'notes');
    await waitFor(() => expect(onSaved).toHaveBeenCalledTimes(1));
    expect(await screen.findByRole('status')).toHaveTextContent('Your note was saved, but the optional link to Concept · Novel could not be added.');
    expect(await screen.findByRole('alert')).toHaveTextContent('Attachment permission rejected');
  });

  it('keeps notes standalone when no context is selected', async () => {
    const { writeNarrative, attachContent } = renderCapture();
    fireEvent.change(screen.getByLabelText('What do you want to keep?'), { target: { value: 'A standalone thought.' } });
    fireEvent.click(screen.getByRole('button', { name: 'Save entry' }));
    await waitFor(() => expect(writeNarrative).toHaveBeenCalledWith(player.id, 'note', 'A standalone thought.', 'A standalone thought.'));
    expect(attachContent).not.toHaveBeenCalled();
  });

  it('creates a Journal entry using the existing narrative service', async () => {
    const { writeNarrative } = renderCapture();
    fireEvent.change(screen.getByLabelText('Entry kind'), { target: { value: 'journal' } });
    fireEvent.change(screen.getByLabelText('What do you want to keep?'), { target: { value: 'Today I noticed the garden after rain.' } });
    fireEvent.click(screen.getByRole('button', { name: 'Save entry' }));
    await waitFor(() => expect(writeNarrative).toHaveBeenCalledWith(player.id, 'journal', 'Today I noticed the garden after rain.', 'Today I noticed the garden after rain.'));
  });

  it('traps keyboard focus within the dialog and closes on Escape', () => {
    const { onClose } = renderCapture();
    const close = screen.getByRole('button', { name: 'Close quick capture' });
    const lastEnabledAction = screen.getByRole('button', { name: 'Cancel' });
    close.focus();
    fireEvent.keyDown(document, { key: 'Tab', shiftKey: true });
    expect(document.activeElement).toBe(lastEnabledAction);
    fireEvent.keyDown(document, { key: 'Escape' });
    expect(onClose).toHaveBeenCalledOnce();
  });
});
