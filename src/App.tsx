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

import { AppShell } from './app/AppShell';
import { StatusScreen } from './features/status/StatusScreen';

export function App() {
  return (
    <AppShell>
      <StatusScreen />
    </AppShell>
  );
}
