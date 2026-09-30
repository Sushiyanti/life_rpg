# Life RPG UX Constitution

## Core model

- **Page** — a major application destination such as Overview, Quests, Skills, Concepts, Guidebook, History, or Rules.
- **Workspace** — the persistent context within a page where the player is currently working. Workspace position, scroll, filters, and panel state belong to the player’s current context.
- **Entity** — a world record such as a Player, Quest, Concept, Effect, Note, Session, or Skill.
- **Entity surface** — a contextual detail/edit surface for one entity. It is rendered above the current page and does not replace or navigate the page.
- **Nested entity surface** — a surface opened from another surface. Surfaces use a stack: closing the child returns to the parent, and closing the parent returns to the original workspace.

> Normal entity inspection and editing must happen in context.

The reusable `EntitySurfaceProvider` and stack are the single interaction foundation for entity details. Entity-specific renderers provide content; the surface owns overlay, focus, keyboard, responsive presentation, copy, and stack behavior.

## Route navigation

Route navigation is appropriate for:

- major application destinations;
- large collection management;
- dedicated analytical views; and
- settings and tools.

Route navigation is inappropriate for:

- viewing one Quest;
- viewing one Concept;
- viewing one Effect;
- editing one Note;
- inspecting one Session; or
- inspecting one Skill.

Those interactions should open a surface above the current page. The underlying page remains mounted so scroll position, filters, expanded panels, and workspace context survive ordinary interaction.

## Interaction requirements

- Escape closes the topmost surface.
- The backdrop closes the surface without changing the route.
- Focus moves to the opened surface and returns to the triggering control when possible.
- Wide layouts use a right-side inspector; constrained layouts use a centered dialog presentation.
- Editing is an explicit mode with Save and Cancel, and saving remains on the same surface.
- Copy actions provide non-destructive feedback.
- Domain rules and persistence remain behind the typed Tauri IPC client; React does not access SQLite.
