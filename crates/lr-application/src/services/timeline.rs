//! Application use case for querying persisted chronological records.

use crate::{AppError, TimelineItem, TimelineQuery, TimelineStore};

pub struct TimelineService<S: TimelineStore> {
    store: S,
}
impl<S: TimelineStore> TimelineService<S> {
    pub fn new(store: S) -> Self {
        Self { store }
    }

    /// Returns only stored source facts, projected by persistence without writes.
    pub fn query(&self, query: &TimelineQuery) -> Result<Vec<TimelineItem>, AppError> {
        query.validate()?;
        Ok(self.store.query_timeline(query)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{StorageError, TimelineCategory, TimelineSort};
    use lr_domain::{EntityId, Iso8601Timestamp};
    use std::sync::{Arc, Mutex};

    #[derive(Default)]
    struct FakeTimelineStore {
        seen: Mutex<Vec<TimelineQuery>>,
    }
    impl TimelineStore for FakeTimelineStore {
        fn query_timeline(&self, query: &TimelineQuery) -> Result<Vec<TimelineItem>, StorageError> {
            self.seen.lock().unwrap().push(query.clone());
            Ok(vec![TimelineItem {
                source_id: "transaction:1".into(),
                player_id: query.player_id.clone(),
                category: TimelineCategory::Transaction,
                entity_kind: crate::TimelineEntityKind::Transaction,
                entity_id: "1".into(),
                timestamp: Iso8601Timestamp::parse("2026-09-28T10:00:00Z").unwrap(),
                secondary_timestamp: None,
                timestamp_kind: crate::TimelineTimestampKind::Occurred,
                secondary_timestamp_kind: None,
                title: "XP transaction".into(),
                summary: "+5 xp".into(),
                concept_id: None,
                type_code: Some("xp".into()),
                state: None,
                relationship_context: None,
            }])
        }
    }

    fn query() -> TimelineQuery {
        TimelineQuery {
            player_id: EntityId::new("player-1").unwrap(),
            category: Some(TimelineCategory::Transaction),
            entity_kind: None,
            entity_id: None,
            concept_id: None,
            from: None,
            through: None,
            sort: TimelineSort::Newest,
            limit: 15,
            offset: 30,
        }
    }

    #[test]
    fn validates_and_forwards_bounded_typed_query_without_mutating_state() {
        let store = Arc::new(FakeTimelineStore::default());
        let service = TimelineService::new(store.clone());
        let result = service.query(&query()).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].source_id, "transaction:1");
        assert_eq!(store.seen.lock().unwrap().as_slice(), &[query()]);
    }

    #[test]
    fn invalid_query_is_rejected_before_reaching_the_store() {
        let store = Arc::new(FakeTimelineStore::default());
        let service = TimelineService::new(store.clone());
        let mut invalid = query();
        invalid.limit = 201;
        assert!(service.query(&invalid).is_err());
        assert!(store.seen.lock().unwrap().is_empty());
    }
}
