import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import type { CoreClient } from '../domain/ipc';
import type { Tag, Workspace, WorkspacePanel } from '../domain/world';
import { PANEL_REGISTRY } from '../features/workspaces/panelRegistry';
import { WorkspaceBuilder } from '../features/workspaces/WorkspaceBuilder';

const workspace: Workspace = {
  id: 'workspace-1', playerId: 'player-1', name: 'Learning', template: 'learning', sortOrder: 0,
  isDefault: true, createdAt: '2026-09-28T12:00:00Z', updatedAt: '2026-09-28T12:00:00Z',
};
const panel: WorkspacePanel = {
  id: 'panel-1', workspaceId: workspace.id, panelType: 'quests', title: 'Quest board',
  variant: 'cards', density: 'cozy', filterStatus: null, filterActive: null,
  filterTypeCode: null, filterConceptId: null, filterTagIds: [], filterTagMatch: 'any',
  filterRecentDays: null, filterTimelineCategory: null, filterTimelineEntityKind: null,
  filterTimelineEntityId: null, filterTimelineFrom: null, filterTimelineThrough: null,
  sortBy: 'updated_desc', itemLimit: 8, sortOrder: 0, gridSpan: 1,
  isVisible: true, isPinned: false, isCollapsed: false,
  createdAt: '2026-09-28T12:00:00Z', updatedAt: '2026-09-28T12:00:00Z',
};
const tags: Tag[] = ['tag-1', 'tag-2'].map((id, index) => ({
  id, playerId: workspace.playerId, transferKey: `tag-ref-${index + 1}`,
  name: `Label ${index + 1}`, normalizedName: `label ${index + 1}`, description: null,
  lifecycle: 'active', usageCount: 0, createdAt: '', updatedAt: '',
}));

describe('Workspace Tag filters', () => {
  it('authors and persists a bounded multi-Tag all-match filter through CoreClient', async () => {
    const saveWorkspacePanel = vi.fn(async (payload: Record<string, unknown>) => ({
      ...panel, ...payload, id: panel.id, workspaceId: workspace.id,
    }));
    const client = {
      listTags: vi.fn(async () => tags),
      saveWorkspacePanel,
    } as unknown as CoreClient;
    const onPanelsChange = vi.fn();
    render(<WorkspaceBuilder
      client={client}
      playerId={workspace.playerId}
      workspace={workspace}
      workspaces={[workspace]}
      panels={[panel]}
      concepts={[]}
      onPanelsChange={onPanelsChange}
      onCreate={async () => {}}
      onRename={async () => {}}
      onDefault={async () => {}}
      onDelete={async () => {}}
      onDuplicate={async () => {}}
      onImport={async () => {}}
    />);

    const select = await screen.findByLabelText(/^Tags/) as HTMLSelectElement;
    for (const option of Array.from(select.options)) option.selected = ['tag-1', 'tag-2'].includes(option.value);
    fireEvent.change(select);
    fireEvent.change(await screen.findByLabelText('Tag match'), { target: { value: 'all' } });
    fireEvent.click(screen.getByRole('button', { name: 'Save panel' }));

    await waitFor(() => expect(saveWorkspacePanel).toHaveBeenCalledWith(expect.objectContaining({
      playerId: workspace.playerId,
      workspaceId: workspace.id,
      panelId: panel.id,
      filterTagIds: ['tag-1', 'tag-2'],
      filterTagMatch: 'all',
    })));
    expect(PANEL_REGISTRY.quests.supportsTags).toBe(true);
    expect(PANEL_REGISTRY.timeline.supportsTags).toBe(false);
  });
});
