import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import type { CoreClient } from '../domain/ipc';
import type { Workspace } from '../domain/world';
import { WorkspaceBuilder } from '../features/workspaces/WorkspaceBuilder';

const workspace: Workspace = {
  id: 'workspace-destination',
  playerId: 'player-destination',
  name: 'Focus',
  template: 'focus',
  sortOrder: 0,
  isDefault: true,
  createdAt: '',
  updatedAt: '',
};

const filePayload = {
  format: 'life-rpg-workspace',
  version: 2,
  workspace: { name: 'Python study', template: 'learning' },
  panels: [{
    panelType: 'quests',
    title: 'Python goals',
    variant: 'cards',
    density: 'cozy',
    filterStatus: 'active',
    filterActive: null,
    filterTypeCode: null,
    relatedConcept: { key: 'concept-ref-v1-python-key', name: 'Python', typeCode: 'subject' },
    filterRecentDays: null,
    sortBy: 'updated_desc',
    itemLimit: 8,
    sortOrder: 0,
    gridSpan: 1,
    isVisible: true,
    isPinned: false,
    isCollapsed: false,
  }],
};

describe('WorkspaceBuilder transfer feedback', () => {
  it('shows unresolved Concept filters before commit and confirms they remain unfiltered', async () => {
    const onImport = vi.fn(async () => undefined);
    render(
      <WorkspaceBuilder
        client={{} as CoreClient}
        playerId={workspace.playerId}
        workspace={workspace}
        workspaces={[workspace]}
        panels={[]}
        concepts={[]}
        onPanelsChange={() => undefined}
        onCreate={async () => undefined}
        onRename={async () => undefined}
        onDefault={async () => undefined}
        onDelete={async () => undefined}
        onDuplicate={async () => undefined}
        onImport={onImport}
      />,
    );

    const file = new File([JSON.stringify(filePayload)], 'workspace.json', { type: 'application/json' });
    Object.defineProperty(file, 'text', { value: async () => JSON.stringify(filePayload) });
    fireEvent.change(screen.getByLabelText('Preview import'), { target: { files: [file] } });

    expect(await screen.findByText('No matching Concept; the filter will be left unfiltered.')).toBeInTheDocument();
    expect(onImport).not.toHaveBeenCalled();
    fireEvent.click(screen.getByRole('button', { name: 'Import reviewed workspace' }));

    await waitFor(() => expect(onImport).toHaveBeenCalledWith(expect.objectContaining({
      resolvedConcepts: 0,
      unresolvedConcepts: 1,
      panels: [expect.objectContaining({ filterConceptId: null })],
    })));
    expect(await screen.findByText('Workspace imported. 0 Concept filter(s) resolved; 1 left unfiltered; 0 Timeline exact identity filter(s) need to be selected again in this world.')).toBeInTheDocument();
  });
});
