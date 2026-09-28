import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import type { CoreClient } from '../domain/ipc';
import type { TimelineItem, TimelineQuery } from '../domain/world';
import { TimelineScreen } from '../features/world/TimelineScreen';

const event = (overrides: Partial<TimelineItem> = {}): TimelineItem => ({
  sourceId: 'session:session-1', playerId: 'player-1', category: 'session', entityKind: 'quest_session', entityId: 'session-1',
  timestamp: '2026-09-28T10:30:00Z', secondaryTimestamp: null, timestampKind: 'occurred', secondaryTimestampKind: null,
  title: 'Session · Garden', summary: 'Seeds prepared and soil watered.', conceptId: null, typeCode: null, state: 'completed', relationshipContext: null, ...overrides,
});
const queryClient = (queryTimeline: (query: TimelineQuery) => Promise<TimelineItem[]>) => ({ queryTimeline: vi.fn(queryTimeline) } as unknown as CoreClient);
const renderScreen = (client: CoreClient, onOpenEntity = vi.fn()) => render(
  <TimelineScreen client={client} player={{ id: 'player-1', name: 'Ada' }} concepts={[]} onOpenEntity={onOpenEntity} />,
);

describe('TimelineScreen', () => {
  it('groups recorded items by date, preserves timestamp meanings, and navigates to the exact source record', async () => {
    const rows = [
      event({ sourceId: 'session:2', entityId: 'session-2', title: 'Session · Garden', timestamp: '2026-09-28T12:30:00Z', secondaryTimestamp: '2026-09-28T13:00:00Z' }),
      event({ sourceId: 'comment:1', category: 'comment', entityKind: 'quest', entityId: 'quest-1', title: 'Comment · Prepare beds', timestamp: '2026-09-28T11:00:00Z', secondaryTimestampKind: null, timestampKind: 'created', state: null }),
      event({ sourceId: 'transaction:1', category: 'transaction', entityKind: 'transaction', entityId: '42', title: 'Transaction · XP', timestamp: '2026-09-27T09:00:00Z', timestampKind: 'occurred', state: null }),
    ];
    const queryTimeline = vi.fn(async () => rows);
    const client = queryClient(queryTimeline);
    const onOpenEntity = vi.fn();
    renderScreen(client, onOpenEntity);

    const target = await screen.findByRole('button', { name: /Comment · Prepare beds/ });
    expect(screen.getByText('Ada')).toBeInTheDocument();
    expect(screen.getAllByRole('heading', { level: 3 })).toHaveLength(2);
    expect(screen.getByText('Session · Garden')).toBeInTheDocument();
    expect(screen.getByText('Started')).toBeInTheDocument();
    expect(screen.getByText(/Ended ·/)).toBeInTheDocument();
    expect(screen.getByText('created')).toBeInTheDocument();
    expect(queryTimeline).toHaveBeenCalledWith(expect.objectContaining({
      playerId: 'player-1', category: null, entityKind: null, entityId: null, conceptId: null,
      from: null, through: null, sort: 'newest', limit: 50, offset: 0,
    }));
    fireEvent.click(target);
    expect(onOpenEntity).toHaveBeenCalledWith('quest', 'quest-1');
  });

  it('inspects an exact Content relationship and opens both the Content and target records', async () => {
    const onOpenEntity = vi.fn();
    const relationship = event({
      sourceId: 'relationship:rel-9:removed', category: 'relationship_history', entityKind: 'quest', entityId: 'quest-7',
      title: 'Content relationship removed · guidance', summary: 'Content · Garden guide → quest quest-7', timestampKind: 'removed', state: 'removed',
      relationshipContext: { relationshipId: 'rel-9', contentId: 'content-3', contentTitle: 'Garden guide', roleCode: 'guidance', createdAt: '2026-09-20T10:00:00Z', removedAt: '2026-09-28T10:00:00Z' },
    });
    renderScreen(queryClient(async () => [relationship]), onOpenEntity);
    await screen.findByRole('button', { name: /Content relationship removed/ });
    fireEvent.click(screen.getByText('Inspect relationship context'));
    expect(screen.getByText('Garden guide')).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: 'Open Content' }));
    fireEvent.click(screen.getByRole('button', { name: 'Open target' }));
    expect(onOpenEntity).toHaveBeenNthCalledWith(1, 'narrative_entry', 'content-3');
    expect(onOpenEntity).toHaveBeenNthCalledWith(2, 'quest', 'quest-7');
  });

  it('sends category, entity, concept, and inclusive local date filters as a bounded query', async () => {
    const queryTimeline = vi.fn(async () => []);
    renderScreen(queryClient(queryTimeline));
    expect(await screen.findByText('No recorded events in this range.')).toBeInTheDocument();

    fireEvent.change(screen.getByLabelText('Timeline category'), { target: { value: 'content' } });
    fireEvent.change(screen.getByLabelText('Entity kind'), { target: { value: 'narrative_entry' } });
    fireEvent.change(screen.getByLabelText('Timeline from date'), { target: { value: '2026-09-10' } });
    fireEvent.change(screen.getByLabelText('Exact entity ID'), { target: { value: 'session-42' } });
    fireEvent.change(screen.getByLabelText('Timeline through date'), { target: { value: '2026-09-12' } });
    fireEvent.change(screen.getByLabelText('Timeline order'), { target: { value: 'oldest' } });

    await waitFor(() => expect(queryTimeline).toHaveBeenLastCalledWith(expect.objectContaining({
      playerId: 'player-1', category: 'content', entityKind: 'narrative_entry', entityId: 'session-42',
      from: new Date('2026-09-10T00:00:00.000').toISOString(),
      through: new Date('2026-09-12T23:59:59.999').toISOString(),
      sort: 'oldest', limit: 50, offset: 0,
    })));
    fireEvent.click(screen.getByRole('button', { name: 'Clear filters' }));
    await waitFor(() => expect(queryTimeline).toHaveBeenLastCalledWith(expect.objectContaining({ category: null, entityKind: null, entityId: null, from: null, through: null, sort: 'newest' })));
  });

  it('offers bounded load-more paging without fetching an unbounded history', async () => {
    const firstPage = Array.from({ length: 50 }, (_, index) => event({ sourceId: `session:${index}`, entityId: `session-${index}`, title: `Session ${index}` }));
    const secondPage = [event({ sourceId: 'session:50', entityId: 'session-50', title: 'Session 50', timestamp: '2026-09-27T10:30:00Z' })];
    const queryTimeline = vi.fn().mockResolvedValueOnce(firstPage).mockResolvedValueOnce(secondPage);
    renderScreen(queryClient(queryTimeline));
    await screen.findByRole('button', { name: /Session 0/ });
    fireEvent.click(screen.getByRole('button', { name: 'Load more recorded items' }));
    await screen.findByRole('button', { name: /Session 50/ });
    expect(queryTimeline).toHaveBeenNthCalledWith(2, expect.objectContaining({ limit: 50, offset: 50, playerId: 'player-1' }));
    expect(screen.getAllByRole('button', { name: /Session/ }).length).toBeGreaterThanOrEqual(51);
  });

  it('keeps empty and error states honest and does not claim that nothing happened', async () => {
    const empty = queryClient(async () => []);
    renderScreen(empty);
    expect(await screen.findByText('No recorded events in this range.')).toBeInTheDocument();
    expect(screen.queryByText('Nothing happened.')).not.toBeInTheDocument();
  });
});


describe('Timeline accessibility states', () => {
  it('announces loading and query errors without inventing an empty result', async () => {
    let rejectQuery!: (reason: Error) => void;
    const queryTimeline = vi.fn(() => new Promise<TimelineItem[]>((_resolve, reject) => { rejectQuery = reject; }));
    renderScreen(queryClient(queryTimeline));
    expect(screen.getByRole('status')).toHaveTextContent('Loading recorded history');
    rejectQuery(new Error('Local history is unavailable.'));
    expect(await screen.findByRole('alert')).toHaveTextContent('Local history is unavailable.');
    expect(screen.queryByText('No recorded events in this range.')).not.toBeInTheDocument();
  });

  it('keeps a long title on the exact-record button and exposes the full long summary on demand', async () => {
    const title = `A carefully preserved record ${'with a long title '.repeat(15)}`;
    const summary = `Full recorded notes: ${'seed trays, soil preparation, and careful watering. '.repeat(8)}`;
    const row = event({ title, summary });
    renderScreen(queryClient(async () => [row]));
    const open = await screen.findByRole('button', { name: new RegExp(title.slice(0, 30)) });
    expect(open).toHaveAttribute('title', `Open quest session: ${title}`);
    fireEvent.click(screen.getByText('Read full recorded summary'));
    const full = screen.getByText('Read full recorded summary').closest('details');
    expect(full?.querySelector('p')).toHaveTextContent(summary.trim());
    expect(open).toBeInTheDocument();
  });
});
