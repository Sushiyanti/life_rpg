/**
 * Vitest setup: jsdom + jest-dom matchers.
 *
 * No `@tauri-apps/api` stubbing happens here on purpose — the domain API layer
 * takes an injectable transport, so tests never need to reach for the global
 * Tauri internals object. If a test *does* want to exercise the real default
 * transport, `src/test/ipc.test.ts` shows how to stub it explicitly.
 */

import '@testing-library/jest-dom/vitest';

// React 18 + jsdom: silence the "not wrapped in act" noise that fires for the
// async status fetch in tests that intentionally do not await it.
import { configure } from '@testing-library/react';

configure({ asyncUtilTimeout: 2000 });
