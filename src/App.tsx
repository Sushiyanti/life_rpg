/**
 * Application root.
 *
 * Phase 1 renders exactly one screen: the status/health report. There is no
 * router and no workspace shell yet — adding them now would be building Phase 4
 * furniture for a Phase 1 foundation. What this file *does* establish is the
 * shape every later screen follows:
 *
 *   screen component  -> use-case hook -> CoreClient -> IPC -> Rust use case
 *
 * The screen owns no knowledge of SQLite and no knowledge of `invoke`.
 */

import { useState } from 'react';
import { AppShell } from './app/AppShell';
import { StatusScreen } from './features/status/StatusScreen';
import { RulePanel } from './features/world/RulePanel';
import { WorldPanel } from './features/world/WorldPanel';
import type { Player } from './domain/world';

export function App() {
  const [player,setPlayer]=useState<Player|null>(null);
  return (
    <AppShell>
      <StatusScreen />
      <WorldPanel onPlayerChanged={setPlayer} />
      <RulePanel playerId={player?.id} />
    </AppShell>
  );
}
