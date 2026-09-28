//! Use cases. One module per use case family; each is a small struct holding
//! its ports. No service reaches for a global — everything it needs is
//! injected, which is what keeps them trivially testable.

pub mod concepts;
pub mod health;
pub mod rule_engine;
pub mod semantics;
pub mod tags;
pub mod timeline;
pub mod world;
