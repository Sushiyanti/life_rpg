# Life RPG UX Constitution

## Core model

- **Page** — a major destination such as Overview, Quests, Skills, Concepts, Guidebook, History, or Rules.
- **Workspace** — persistent context within a page. Workspace position, scroll, filters, panels, and selected workspace belong to the player’s current context.
- **Entity** — a persisted world record such as Player, Quest, Concept, Effect, Session, Skill, Note, or Stage.
- **Entity surface** — a contextual detail/edit surface rendered above the current page. It does not replace or navigate the page.
- **Nested entity surface** — a child surface opened from another surface. Back removes only the child; Close leaves the entire surface stack and returns focus to the original workspace trigger where possible.

> Normal entity inspection and editing must happen in context.

## Implemented now

The registry-backed surface foundation currently supports these entity kinds:

| Entity | Loading | View | Edit |
|---|---|---|---|
| Player | Player-scoped single-record read | Identity, progression, description | Authored progression through the existing command |
| Quest | Single Quest read plus filtered Concept associations | Status, progress, type, description, attached Concepts | Read-only in this phase |
| Concept | Single Concept read plus relationship targets | Identity, status, description, related Concepts | Honest Active/Inactive state mutation |
| Effect | Player-scoped effect query | Type, state, intensity, description | Read-only in this phase |

Quest→Concept and Concept→Concept navigation use actual persisted relationships and authoritative IDs. The TypeScript client exposes the backend association filters instead of silently sending null filters.

## Foundation supports later

The generic registry and surface lifecycle are designed to accept later descriptors for Session, Skill, Note/Content, Stage, Branch, Level, and other entities. Those entities are **not** presented as implemented surface renderers by this checkpoint. Adding one should mean registering its loader, renderer/editor, title formatter, and copy formatter rather than changing stack, overlay, focus, or lifecycle code.

## Architecture rules

The generic surface infrastructure owns the stack, Back/Close semantics, backdrop, responsive drawer/dialog presentation, Escape, keyboard focus containment, body locking, common toolbar, copy action, loading lifecycle, and missing-record state. Entity-specific modules own their loader, metadata presentation, domain editor, relationship presentation, and human-readable copy formatter.

The generic infrastructure must not require entity-specific branches for stack, overlay, focus, or lifecycle behavior. A new entity surface belongs in its own descriptor and renderer module. Entity-specific rendering is valid; entity-specific modal mechanics are not.

The generic load result contains only the resolved entity and optional descriptor-owned context. Quest attachment data and Concept relationship data remain in their own loaders and renderers. If a kind has no descriptor, the surface explicitly says that the type has no contextual viewer; it never loads the record through an unrelated entity descriptor.

The boundary remains:

> React + TypeScript → typed Tauri IPC → Rust application/domain → SQLite persistence

React does not access SQLite or invent relationship semantics. The resolver may orchestrate typed CoreClient calls, while Rust remains the owner of domain and data rules.

## Navigation and context

Route navigation is appropriate for major destinations, large collection management, analytical views, settings, and tools. It is inappropriate for inspecting one Quest, Concept, Effect, Player, Session, or Skill. Those interactions open a surface above the current page so the workspace remains mounted.

The page/workspace owns its own scrolling. The open surface owns only its constrained body scrolling, with overscroll contained at the surface boundary. Opening or closing a surface must not recreate the workspace or change the route.

## Interaction and accessibility requirements

- Escape removes the topmost child surface; with one surface it closes the surface.
- The explicit Close control leaves the entire surface stack.
- The Back control removes only the current child; with one surface it closes it deliberately.
- The backdrop closes the entire stack without changing the route.
- Focus enters the opened dialog, Tab remains contained, and focus does not escape into the inert background.
- Closing a child restores focus to its parent trigger where possible; closing the stack restores focus to the original workspace trigger.
- Wide layouts use a right-side inspector; constrained layouts use a centered dialog.
- Editing is explicit and supports Save/Cancel only where a real domain mutation exists.
- Missing, archived, trashed, or externally changed records receive an explanatory state with Close and Back when a parent exists.
- Copy actions provide human-readable, entity-specific, non-destructive feedback.

## Scope boundary

This checkpoint does not redesign the Level Manager, Notes system, Guidebook, Quest experience, Skill system, navigation customization, display settings, Explorer, History, Rules, or global theme. It establishes the foundation those later phases can extend safely.
