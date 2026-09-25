//! Use cases. One module per use case family; each is a small struct holding
//! its ports. No service reaches for a global — everything it needs is
//! injected, which is what keeps them trivially testable.

pub mod health;
