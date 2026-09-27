import { afterEach, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';

const mocks = vi.hoisted(() => ({ searchWorld: vi.fn(async () => []) }));
vi.mock('../domain/ipc', () => ({
  coreClient: {
    searchWorld: mocks.searchWorld,
    ping: vi.fn(async () => ({ status: 'healthy' })),
    listStatDefinitions: vi.fn(async () => []),
  },
}));

import { App } from '../App';

afterEach(() => {
  cleanup();
  localStorage.clear();
  mocks.searchWorld.mockClear();
});

it('boots a fresh Player world without exceeding the native search limit', async () => {
  render(<App />);
  await waitFor(() => expect(mocks.searchWorld).toHaveBeenCalledOnce());
  expect(mocks.searchWorld).toHaveBeenCalledWith(expect.objectContaining({ kind: 'player', limit: 200 }));
  expect(await screen.findByRole('heading', { name: 'Start your first Player world' })).toBeInTheDocument();
  expect(screen.queryByText('World unavailable')).not.toBeInTheDocument();
  expect(screen.getAllByText('Life RPG').length).toBeGreaterThan(0);
  expect(screen.queryByText(/3\.6|world semantics/i)).not.toBeInTheDocument();
});


import { WorldExplorer } from '../features/world/WorldExplorer';
import type { Player, WorldOverview } from '../domain/world';

type ExplorerClient = Parameters<typeof WorldExplorer>[0]['client'];

it('offers only valid bounded search page sizes in Explorer', async () => {
  const player: Player = { id: 'player-1', name: 'Rin', description: null, level: 1, levelName: null, progressionLabel: null, currentXp: 0, isActive: true, metadataJson: '{}', createdAt: '', updatedAt: '' };
  const overview: WorldOverview = { player, quests: [], skillTrees: [], skills: [], effects: [], recentTransactions: [], narratives: [] };
  const searchWorld = vi.fn(async () => []);
  const client = { searchWorld, getEntityLifecycle: vi.fn(async () => ({ state: 'untracked' as const })) } as unknown as ExplorerClient;
  render(<WorldExplorer client={client} player={player} concepts={[]} overview={overview} onRefresh={async () => {}} />);
  const pageSize = await screen.findByLabelText('Page size') as HTMLSelectElement;
  expect(Array.from(pageSize.options).map(option => Number(option.value))).toEqual([25, 50, 100, 200]);
  fireEvent.change(pageSize, { target: { value: '200' } });
  fireEvent.click(screen.getByRole('button', { name: 'Search world' }));
  await waitFor(() => expect(searchWorld).toHaveBeenLastCalledWith(expect.objectContaining({ limit: 200 })));
});
