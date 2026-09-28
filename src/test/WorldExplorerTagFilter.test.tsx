import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import type { CoreClient } from '../domain/ipc';
import type { Player, Tag, WorldOverview } from '../domain/world';
import { WorldExplorer } from '../features/world/WorldExplorer';

const player: Player = {
  id: 'player-1', name: 'Ada', description: null, level: 1, levelName: null,
  progressionLabel: null, currentXp: 0, isActive: true, metadataJson: '{}',
  createdAt: '2026-09-28T12:00:00Z', updatedAt: '2026-09-28T12:00:00Z',
};
const overview: WorldOverview = {
  player, quests: [], skillTrees: [], skills: [], effects: [], recentTransactions: [], narratives: [],
};
const tags: Tag[] = ['tag-1', 'tag-2'].map((id, index) => ({
  id, playerId: player.id, transferKey: `tag-ref-${index + 1}`, name: `Label ${index + 1}`,
  normalizedName: `label ${index + 1}`, description: null, lifecycle: 'active', usageCount: 0,
  createdAt: '', updatedAt: '',
}));

describe('World Explorer Tag filters', () => {
  it('sends selected any/all Tags to Search without loading world rows into React for filtering', async () => {
    const searchWorld = vi.fn(async () => []);
    const client = {
      searchWorld,
      listTags: vi.fn(async () => tags),
    } as unknown as CoreClient;
    render(<WorldExplorer client={client} player={player} concepts={[]} overview={overview} onRefresh={async () => {}} />);
    await waitFor(() => expect(searchWorld).toHaveBeenCalled());

    const select = await screen.findByLabelText(/^Tags/) as HTMLSelectElement;
    for (const option of Array.from(select.options)) option.selected = ['tag-1', 'tag-2'].includes(option.value);
    fireEvent.change(select);
    fireEvent.change(await screen.findByLabelText('Tag match'), { target: { value: 'all' } });
    fireEvent.click(screen.getByRole('button', { name: 'Search world' }));

    await waitFor(() => expect(searchWorld).toHaveBeenLastCalledWith(expect.objectContaining({
      playerId: player.id,
      tagIds: ['tag-1', 'tag-2'],
      tagMatch: 'all',
      context: 'explorer',
      limit: 100,
    })));
  });
});
