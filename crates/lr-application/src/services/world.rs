//! Phase 2 world use cases. This layer orchestrates domain values through ports;
//! it never imports SQLite, SQL, Tauri, or frontend types.

use crate::{AppError, Clock, WorldStore};
use lr_domain::{
    Comment, CommentTargetKind, DateValue, Effect, EntityId, Iso8601Timestamp, NarrativeEntry,
    Player, PlayerStateSnapshot, Quest, Skill, SkillTree, Transaction, TypeRef,
};
use std::sync::atomic::{AtomicU64, Ordering};

pub const DEFAULT_LEDGER_LIMIT: u32 = 25;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldOverview {
    pub player: Player,
    pub quests: Vec<Quest>,
    pub skill_trees: Vec<SkillTree>,
    pub skills: Vec<Skill>,
    pub effects: Vec<Effect>,
    pub recent_transactions: Vec<Transaction>,
    pub narratives: Vec<NarrativeEntry>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AwardXpOutcome {
    pub player: Player,
    pub transaction: Transaction,
}

pub struct WorldService<S, C>
where
    S: WorldStore,
    C: Clock,
{
    store: S,
    clock: C,
    sequence: AtomicU64,
}
impl<S, C> WorldService<S, C>
where
    S: WorldStore,
    C: Clock,
{
    pub fn new(store: S, clock: C) -> Self {
        Self {
            store,
            clock,
            sequence: AtomicU64::new(0),
        }
    }
    pub fn store(&self) -> &S {
        &self.store
    }
    fn now(&self) -> Result<Iso8601Timestamp, AppError> {
        Ok(Iso8601Timestamp::parse(self.clock.now_rfc3339())?)
    }
    fn new_id(&self, prefix: &str) -> Result<EntityId, AppError> {
        let n = self.clock.now_unix_nanos();
        let s = self.sequence.fetch_add(1, Ordering::SeqCst);
        Ok(EntityId::new(format!("{prefix}-{n:x}-{s:x}"))?)
    }
    fn required_player(&self, raw: &str) -> Result<Player, AppError> {
        let id = EntityId::new(raw)?;
        self.store
            .get_player(&id)?
            .ok_or_else(|| AppError::Internal("player not found".into()))
    }

    pub fn create_player(
        &self,
        name: &str,
        description: Option<String>,
    ) -> Result<Player, AppError> {
        let now = self.now()?;
        let mut value = Player::new(self.new_id("player")?, name, now.clone())?;
        value.description = description.filter(|v| !v.trim().is_empty());
        self.store.create_player(&value)?;
        Ok(value)
    }
    pub fn get_player(&self, id: &str) -> Result<Option<Player>, AppError> {
        Ok(self.store.get_player(&EntityId::new(id)?)?)
    }
    pub fn award_xp(
        &self,
        player_id: &str,
        amount: i64,
        reason: Option<String>,
        description: Option<String>,
    ) -> Result<AwardXpOutcome, AppError> {
        let now = self.now()?;
        let mut player = self.required_player(player_id)?;
        player.apply_xp(amount, now.clone())?;
        let mut tx = Transaction::new(
            player.id.clone(),
            TypeRef::transaction("xp")?,
            "xp",
            amount,
            now,
        )?;
        tx.reason = reason;
        tx.description = description;
        let transaction = self.store.award_xp(&player, &tx)?;
        Ok(AwardXpOutcome {
            player,
            transaction,
        })
    }
    pub fn create_quest(
        &self,
        player_id: &str,
        type_code: &str,
        title: &str,
        parent_quest_id: Option<String>,
        skill_id: Option<String>,
        difficulty: Option<i32>,
        xp_reward: Option<i64>,
    ) -> Result<Quest, AppError> {
        let player = self.required_player(player_id)?;
        let now = self.now()?;
        let mut q = Quest::new(
            self.new_id("quest")?,
            player.id,
            TypeRef::quest(type_code)?,
            title,
            now,
        )?;
        q.parent_quest_id = parent_quest_id.map(EntityId::new).transpose()?;
        q.skill_id = skill_id.map(EntityId::new).transpose()?;
        q.difficulty = difficulty;
        q.xp_reward = xp_reward.unwrap_or(0);
        if q.xp_reward < 0 {
            return Err(AppError::Domain(lr_domain::DomainError::invalid_value(
                "quest XP reward",
                "must not be negative",
            )));
        }
        self.store.insert_quest(&q)?;
        Ok(q)
    }
    pub fn start_quest(&self, quest_id: &str) -> Result<Quest, AppError> {
        let id = EntityId::new(quest_id)?;
        let mut q = self
            .store
            .get_quest(&id)?
            .ok_or_else(|| AppError::Internal("quest not found".into()))?;
        q.start(self.now()?)?;
        self.store.update_quest(&q)?;
        Ok(q)
    }
    pub fn complete_quest(&self, quest_id: &str) -> Result<Quest, AppError> {
        let id = EntityId::new(quest_id)?;
        let mut q = self
            .store
            .get_quest(&id)?
            .ok_or_else(|| AppError::Internal("quest not found".into()))?;
        let now = self.now()?;
        q.complete(now.clone())?;
        if q.xp_reward == 0 {
            self.store.complete_quest(&q, None, None)?;
        } else {
            let mut p = self.required_player(q.player_id.as_str())?;
            p.apply_xp(q.xp_reward, now.clone())?;
            let mut reward = Transaction::new(
                p.id.clone(),
                TypeRef::transaction("xp")?,
                "xp",
                q.xp_reward,
                now,
            )?;
            reward.reason = Some("quest_reward".into());
            reward.source_kind = Some("quest".into());
            reward.source_id = Some(q.id.to_string());
            self.store.complete_quest(&q, Some(&p), Some(&reward))?;
        };
        Ok(q)
    }
    pub fn create_skill_tree(
        &self,
        player_id: &str,
        type_code: &str,
        name: &str,
    ) -> Result<SkillTree, AppError> {
        let p = self.required_player(player_id)?;
        let value = SkillTree::new(
            self.new_id("skill-tree")?,
            p.id,
            TypeRef::skill_tree(type_code)?,
            name,
            self.now()?,
        )?;
        self.store.insert_skill_tree(&value)?;
        Ok(value)
    }
    pub fn add_skill(
        &self,
        tree_id: &str,
        type_code: &str,
        name: &str,
        parent_skill_id: Option<String>,
    ) -> Result<Skill, AppError> {
        let mut value = Skill::new(
            self.new_id("skill")?,
            EntityId::new(tree_id)?,
            TypeRef::skill(type_code)?,
            name,
            self.now()?,
        )?;
        value.parent_skill_id = parent_skill_id.map(EntityId::new).transpose()?;
        self.store.insert_skill(&value)?;
        Ok(value)
    }
    pub fn invest_skill_time(&self, skill_id: &str, minutes: i64) -> Result<Skill, AppError> {
        let id = EntityId::new(skill_id)?;
        let mut skill = self
            .store
            .get_skill(&id)?
            .ok_or_else(|| AppError::Internal("skill not found".into()))?;
        let now = self.now()?;
        skill.invest_time(minutes, now.clone())?;
        let mut tx = Transaction::new(
            self.player_for_tree(&skill.skill_tree_id)?,
            TypeRef::transaction("time")?,
            "minutes",
            minutes,
            now,
        )?;
        tx.reason = Some("skill_investment".into());
        tx.source_kind = Some("skill".into());
        tx.source_id = Some(skill.id.to_string());
        self.store.invest_skill_time(&skill, &tx)?;
        Ok(skill)
    }
    fn player_for_tree(&self, tree_id: &EntityId) -> Result<EntityId, AppError> {
        Ok(self
            .store
            .get_skill_tree(tree_id)?
            .ok_or_else(|| AppError::Internal("skill tree not found".into()))?
            .player_id)
    }
    pub fn apply_effect(
        &self,
        player_id: &str,
        type_code: &str,
        name: &str,
        expires_at: Option<String>,
    ) -> Result<Effect, AppError> {
        let p = self.required_player(player_id)?;
        let now = self.now()?;
        let mut e = Effect::new(
            self.new_id("effect")?,
            p.id,
            TypeRef::effect(type_code)?,
            name,
            now,
        )?;
        e.expires_at = expires_at.map(Iso8601Timestamp::parse).transpose()?;
        self.store.insert_effect(&e)?;
        Ok(e)
    }
    pub fn capture_player_snapshot(
        &self,
        player_id: &str,
    ) -> Result<PlayerStateSnapshot, AppError> {
        let p = self.required_player(player_id)?;
        let now = self.now()?;
        let value = PlayerStateSnapshot::new(
            p.id.clone(),
            DateValue::parse(now.as_str().get(..10).unwrap_or(""))?,
            p.level,
            p.current_xp,
            now,
        )?;
        self.store.insert_player_snapshot(&value)?;
        Ok(value)
    }
    pub fn add_comment(
        &self,
        author_player_id: Option<String>,
        target_kind: &str,
        target_id: &str,
        body: &str,
    ) -> Result<Comment, AppError> {
        let now = self.now()?;
        let value = Comment::new(
            author_player_id.map(EntityId::new).transpose()?,
            CommentTargetKind::parse(target_kind)?,
            EntityId::new(target_id)?,
            body,
            now,
        )?;
        Ok(self.store.add_comment(&value)?)
    }
    pub fn write_narrative(
        &self,
        player_id: &str,
        kind: &str,
        title: &str,
        content: &str,
    ) -> Result<NarrativeEntry, AppError> {
        let p = self.required_player(player_id)?;
        let value = NarrativeEntry::new(
            self.new_id("narrative")?,
            p.id,
            TypeRef::new("narrative_entry", kind)?,
            title,
            content,
            self.now()?,
        )?;
        self.store.insert_narrative_entry(&value)?;
        Ok(value)
    }
    pub fn world_overview(&self, player_id: &str) -> Result<WorldOverview, AppError> {
        let player = self.required_player(player_id)?;
        let trees = self.store.list_skill_trees(&player.id)?;
        let mut skills = Vec::new();
        for tree in &trees {
            skills.extend(self.store.list_skills(&tree.id)?);
        }
        let now = self.now()?;
        Ok(WorldOverview {
            quests: self.store.list_quests(&player.id)?,
            effects: self.store.list_effects(&player.id, Some(now.as_str()))?,
            recent_transactions: self
                .store
                .list_transactions(&player.id, DEFAULT_LEDGER_LIMIT)?,
            narratives: self.store.list_narrative_entries(&player.id)?,
            player,
            skill_trees: trees,
            skills,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{StorageError, WorldStore};
    use std::sync::Mutex;
    struct Clock;
    impl crate::Clock for Clock {
        fn now_rfc3339(&self) -> String {
            "2026-09-25T00:00:00+00:00".into()
        }
        fn now_unix_nanos(&self) -> u128 {
            7
        }
    }
    struct Memory {
        player: Mutex<Option<Player>>,
        tx: Mutex<Vec<Transaction>>,
    }
    impl Memory {
        fn new() -> Self {
            Self {
                player: Mutex::new(None),
                tx: Mutex::new(vec![]),
            }
        }
    }
    impl WorldStore for Memory {
        fn list_type_definitions(
            &self,
            _: Option<&str>,
        ) -> Result<Vec<lr_domain::TypeDefinition>, StorageError> {
            Ok(vec![])
        }
        fn create_type_definition(
            &self,
            d: &lr_domain::TypeDefinition,
        ) -> Result<lr_domain::TypeDefinition, StorageError> {
            Ok(d.clone())
        }
        fn create_player(&self, p: &Player) -> Result<(), StorageError> {
            *self.player.lock().unwrap() = Some(p.clone());
            Ok(())
        }
        fn get_player(&self, _: &EntityId) -> Result<Option<Player>, StorageError> {
            Ok(self.player.lock().unwrap().clone())
        }
        fn update_player(&self, p: &Player) -> Result<(), StorageError> {
            *self.player.lock().unwrap() = Some(p.clone());
            Ok(())
        }
        fn insert_quest(&self, _: &Quest) -> Result<(), StorageError> {
            Ok(())
        }
        fn get_quest(&self, _: &EntityId) -> Result<Option<Quest>, StorageError> {
            Ok(None)
        }
        fn update_quest(&self, _: &Quest) -> Result<(), StorageError> {
            Ok(())
        }
        fn list_quests(&self, _: &EntityId) -> Result<Vec<Quest>, StorageError> {
            Ok(vec![])
        }
        fn insert_skill_tree(&self, _: &SkillTree) -> Result<(), StorageError> {
            Ok(())
        }
        fn get_skill_tree(&self, _: &EntityId) -> Result<Option<SkillTree>, StorageError> {
            Ok(None)
        }
        fn list_skill_trees(&self, _: &EntityId) -> Result<Vec<SkillTree>, StorageError> {
            Ok(vec![])
        }
        fn insert_skill(&self, _: &Skill) -> Result<(), StorageError> {
            Ok(())
        }
        fn get_skill(&self, _: &EntityId) -> Result<Option<Skill>, StorageError> {
            Ok(None)
        }
        fn update_skill(&self, _: &Skill) -> Result<(), StorageError> {
            Ok(())
        }
        fn list_skills(&self, _: &EntityId) -> Result<Vec<Skill>, StorageError> {
            Ok(vec![])
        }
        fn insert_effect(&self, _: &Effect) -> Result<(), StorageError> {
            Ok(())
        }
        fn list_effects(&self, _: &EntityId, _: Option<&str>) -> Result<Vec<Effect>, StorageError> {
            Ok(vec![])
        }
        fn append_transaction(&self, t: &Transaction) -> Result<Transaction, StorageError> {
            Ok(t.clone())
        }
        fn list_transactions(
            &self,
            _: &EntityId,
            _: u32,
        ) -> Result<Vec<Transaction>, StorageError> {
            Ok(self.tx.lock().unwrap().clone())
        }
        fn transaction_total(&self, _: &EntityId, _: &str) -> Result<i64, StorageError> {
            Ok(0)
        }
        fn insert_player_snapshot(&self, _: &PlayerStateSnapshot) -> Result<(), StorageError> {
            Ok(())
        }
        fn list_player_snapshots(
            &self,
            _: &EntityId,
        ) -> Result<Vec<PlayerStateSnapshot>, StorageError> {
            Ok(vec![])
        }
        fn insert_skill_snapshot(
            &self,
            _: &lr_domain::SkillStateSnapshot,
        ) -> Result<(), StorageError> {
            Ok(())
        }
        fn add_comment(&self, c: &Comment) -> Result<Comment, StorageError> {
            Ok(c.clone())
        }
        fn list_comments(
            &self,
            _: CommentTargetKind,
            _: &EntityId,
        ) -> Result<Vec<Comment>, StorageError> {
            Ok(vec![])
        }
        fn insert_narrative_entry(&self, _: &NarrativeEntry) -> Result<(), StorageError> {
            Ok(())
        }
        fn list_narrative_entries(
            &self,
            _: &EntityId,
        ) -> Result<Vec<NarrativeEntry>, StorageError> {
            Ok(vec![])
        }
        fn award_xp(&self, p: &Player, t: &Transaction) -> Result<Transaction, StorageError> {
            *self.player.lock().unwrap() = Some(p.clone());
            self.tx.lock().unwrap().push(t.clone());
            Ok(t.clone())
        }
        fn complete_quest(
            &self,
            _: &Quest,
            _: Option<&Player>,
            _: Option<&Transaction>,
        ) -> Result<Option<Transaction>, StorageError> {
            Ok(None)
        }
        fn invest_skill_time(
            &self,
            _: &Skill,
            t: &Transaction,
        ) -> Result<Transaction, StorageError> {
            Ok(t.clone())
        }
    }
    #[test]
    fn creates_player_and_awards_xp() {
        let service = WorldService::new(Memory::new(), Clock);
        let p = service.create_player("Ada", None).unwrap();
        let outcome = service
            .award_xp(p.id.as_str(), 125, Some("test".into()), None)
            .unwrap();
        assert_eq!(outcome.player.current_xp, 125);
        assert_eq!(outcome.transaction.resource, "xp");
    }
}
