import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import type { CoreClient } from '../domain/ipc';
import type { Player, Tag, TagRelationship, TagTargetReference, TaggedRecord } from '../domain/world';
import { TagAssignments } from '../features/world/TagAssignments';
import { TagManager } from '../features/world/TagManager';

const player: Player = {
  id: 'player-1', name: 'Ada', description: null, level: 1, levelName: null,
  progressionLabel: null, currentXp: 0, isActive: true, metadataJson: '{}',
  createdAt: '2026-09-28T12:00:00Z', updatedAt: '2026-09-28T12:00:00Z',
};
const makeTag = (overrides: Partial<Tag> = {}): Tag => ({
  id: 'tag-1', playerId: player.id, transferKey: 'tag-ref-1', name: 'Study',
  normalizedName: 'study', description: null, lifecycle: 'active', usageCount: 1,
  createdAt: '2026-09-28T12:00:00Z', updatedAt: '2026-09-28T12:00:00Z', ...overrides,
});
const relationship: TagRelationship = {
  id: 'rel-1', playerId: player.id, tagId: 'tag-1', targetKind: 'quest', targetId: 'quest-1',
  addedAt: '2026-09-28T12:00:00Z', removedAt: null,
};
const reference: TagTargetReference = {
  relationship, targetName: 'Learn Rust', targetLifecycle: 'active',
};
const removedReference: TagTargetReference = {
  relationship: { ...relationship, id: 'rel-removed', targetId: 'old-quest', addedAt: '2026-09-27T12:00:00Z', removedAt: '2026-09-28T12:30:00Z' },
  targetName: 'Retired study plan', targetLifecycle: 'archived',
};
const taggedRecord: TaggedRecord = { relationship, tag: makeTag() };

function clientOf(methods: Record<string, unknown>): CoreClient {
  return methods as unknown as CoreClient;
}

describe('Tag Manager', () => {
  it('creates and renames a Tag, archives/restores organization only, and opens the exact related record', async () => {
    let tags: Tag[] = [];
    const listTags = vi.fn(async (_playerId: string, _query?: string, includeArchived = false, includeTrashed = false) =>
      tags.filter(tag => tag.lifecycle === 'active' || (tag.lifecycle === 'archived' && includeArchived) || (tag.lifecycle === 'trashed' && includeTrashed)));
    const client = clientOf({
      listTags,
      listTagTargets: vi.fn(async () => tags.length ? [reference, removedReference] : []),
      createTag: vi.fn(async (_playerId: string, name: string, description?: string) => {
        const created = makeTag({ name, normalizedName: name.toLowerCase(), description: description ?? null });
        tags = [created];
        return created;
      }),
      renameTag: vi.fn(async (_playerId: string, id: string, name: string, description?: string) => {
        tags = tags.map(tag => tag.id === id ? { ...tag, name, normalizedName: name.toLowerCase(), description: description ?? null } : tag);
        return tags[0];
      }),
      setTagLifecycle: vi.fn(async (_playerId: string, id: string, lifecycle: Tag['lifecycle']) => {
        tags = tags.map(tag => tag.id === id ? { ...tag, lifecycle } : tag);
        return tags[0];
      }),
    });
    const onOpenEntity = vi.fn();
    render(<TagManager client={client} player={player} onOpenEntity={onOpenEntity} />);

    fireEvent.change(await screen.findByPlaceholderText('e.g. learning'), { target: { value: 'Study' } });
    fireEvent.click(screen.getByRole('button', { name: 'Create Tag' }));
    expect(await screen.findByText('Tag created in this Player world.')).toBeInTheDocument();
    expect(client.createTag).toHaveBeenCalledWith(player.id, 'Study', undefined);
    expect(await screen.findByText('Retired study plan')).toBeInTheDocument();

    const nameInput = await screen.findByLabelText('Tag name');
    fireEvent.change(nameInput, { target: { value: 'Focused study' } });
    fireEvent.click(screen.getByRole('button', { name: 'Save Tag' }));
    expect(await screen.findByText('Tag updated; its identity and existing relationships were preserved.')).toBeInTheDocument();
    expect(client.renameTag).toHaveBeenCalledWith(player.id, 'tag-1', 'Focused study', undefined);
    expect(tags[0]!.id).toBe('tag-1');

    fireEvent.click(screen.getByRole('button', { name: 'Archive' }));
    expect(await screen.findByText('Tag archived; target records and relationship history remain unchanged.')).toBeInTheDocument();
    fireEvent.click(screen.getByLabelText('Include archived'));
    fireEvent.click(await screen.findByRole('button', { name: 'Restore Tag' }));
    expect(await screen.findByText('Tag restored. Its prior relationships are unchanged.')).toBeInTheDocument();
    expect(client.setTagLifecycle).toHaveBeenLastCalledWith(player.id, 'tag-1', 'active', 'active from Tag Manager');

    fireEvent.click(screen.getByRole('button', { name: 'Move to trash' }));
    expect(await screen.findByText('Tag trashed; target records and relationship history remain unchanged.')).toBeInTheDocument();
    fireEvent.click(screen.getByLabelText('Include trash'));
    fireEvent.click(await screen.findByRole('button', { name: 'Restore Tag' }));
    expect(await screen.findByText('Tag restored. Its prior relationships are unchanged.')).toBeInTheDocument();

    fireEvent.click((await screen.findAllByRole('button', { name: 'Open exact record' }))[0]!);
    expect(onOpenEntity).toHaveBeenCalledWith('quest', 'quest-1');
    expect(reference.relationship.removedAt).toBeNull();
  });

  it('renders empty and error states without changing world data', async () => {
    const client = clientOf({
      listTags: vi.fn(async () => []),
      listTagTargets: vi.fn(async () => []),
    });
    render(<TagManager client={client} player={player} onOpenEntity={vi.fn()} />);
    expect(await screen.findByText('No Tags match. Create a Player-defined label below.')).toBeInTheDocument();

    const failingClient = clientOf({ listTags: vi.fn(async () => { throw new Error('offline'); }) });
    const view = render(<TagManager client={failingClient} player={player} onOpenEntity={vi.fn()} />);
    expect(await screen.findByRole('status')).toHaveTextContent('offline');
    view.unmount();
  });
});

describe('record Tag assignments', () => {
  it('assigns, opens, and removes an explicit relationship without mutating the target', async () => {
    const tag = makeTag();
    let assigned: TaggedRecord[] = [];
    const listTags = vi.fn(async () => [tag]);
    const listTagsForTarget = vi.fn(async () => assigned);
    const attachTag = vi.fn(async () => { assigned = [taggedRecord]; return relationship; });
    const detachTag = vi.fn(async () => { assigned = []; return { ...relationship, removedAt: '2026-09-28T12:05:00Z' }; });
    const client = clientOf({ listTags, listTagsForTarget, attachTag, detachTag });
    const onOpenTag = vi.fn();
    render(<TagAssignments client={client} playerId={player.id} targetKind="quest" targetId="quest-1" onOpenTag={onOpenTag} />);

    expect(await screen.findByText('No Tags assigned.')).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: 'Assign Tag' }));
    expect(await screen.findByRole('button', { name: 'Study' })).toBeInTheDocument();
    expect(attachTag).toHaveBeenCalledWith(player.id, tag.id, 'quest', 'quest-1');
    fireEvent.click(screen.getByRole('button', { name: 'Study' }));
    expect(onOpenTag).toHaveBeenCalledWith(tag.id);

    fireEvent.click(screen.getByRole('button', { name: 'Remove Tag Study' }));
    await waitFor(() => expect(detachTag).toHaveBeenCalledWith(player.id, relationship.id));
    expect(await screen.findByText('No Tags assigned.')).toBeInTheDocument();
    expect(relationship.removedAt).toBeNull();
    expect(listTagsForTarget).toHaveBeenCalledWith(player.id, 'quest', 'quest-1');
  });
});
