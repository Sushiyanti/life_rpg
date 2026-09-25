/**
 * `AppShell` — the outermost chrome: title bar band plus a content slot.
 *
 * It knows nothing about status or storage. Phase 5 will swap the static title
 * for a command/keyboard-shortcut bar and put workspace tabs here; the slot
 * contract (`children`) will not change.
 */

import type { ReactNode } from 'react';
import './AppShell.css';

interface AppShellProps {
  children: ReactNode;
}

export function AppShell({ children }: AppShellProps) {
  return (
    <div className="app-shell">
      <header className="app-shell__bar">
        <div className="app-shell__brand">
          <span className="app-shell__sigil" aria-hidden="true">
            ◆
          </span>
          <span className="app-shell__title">Life RPG</span>
          <span className="app-shell__tag">local-first world engine</span>
        </div>
        <div className="app-shell__meta">
          <span className="app-shell__phase">Phase 1 · foundation</span>
        </div>
      </header>

      <main className="app-shell__content">{children}</main>
    </div>
  );
}
