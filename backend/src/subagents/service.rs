use super::model::*;
use crate::error::{AppError, AppResult};
use sqlx::PgPool;
use std::collections::HashSet;
use uuid::Uuid;

#[derive(Clone)]
pub struct SubagentsService {
    pub pool: PgPool,
}

impl SubagentsService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// All sub-agents attached to `parent_id`, ordered by position.
    pub async fn list(&self, parent_id: Uuid) -> AppResult<Vec<AttachedSubagent>> {
        self.ensure_agent_exists(parent_id).await?;
        let rows: Vec<AttachedSubagent> = sqlx::query_as(
            r#"SELECT s.child_id, a.name, s.alias, s.description, s.position
               FROM agent_subagents s
               JOIN agents a ON a.id = s.child_id
               WHERE s.parent_id = $1
               ORDER BY s.position ASC"#,
        )
        .bind(parent_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    /// Attach `input.child_id` as a sub-agent of `parent_id`, validating the
    /// alias, the cap, self-attach, and cycles, then appending at the next position.
    pub async fn attach(
        &self,
        parent_id: Uuid,
        input: AttachInput,
    ) -> AppResult<AttachedSubagent> {
        self.ensure_agent_exists(parent_id).await?;

        // The child must exist; its name seeds the default alias.
        let child_name: Option<String> =
            sqlx::query_scalar("SELECT name FROM agents WHERE id = $1")
                .bind(input.child_id)
                .fetch_optional(&self.pool)
                .await?;
        let child_name = child_name.ok_or_else(|| {
            AppError::validation_field(
                "child_id",
                format!("agent {} does not exist", input.child_id),
            )
        })?;

        if input.child_id == parent_id {
            return Err(AppError::validation_field(
                "child_id",
                "an agent cannot be attached as its own sub-agent",
            ));
        }
        if let Some(d) = &input.description {
            validate_description(d)?;
        }

        let existing = self.list(parent_id).await?;
        if existing.len() >= MAX_SUBAGENTS {
            return Err(AppError::validation_field(
                "child_id",
                format!("an agent can have at most {MAX_SUBAGENTS} sub-agents"),
            ));
        }
        if existing.iter().any(|e| e.child_id == input.child_id) {
            return Err(AppError::validation_field(
                "child_id",
                "this agent is already attached as a sub-agent",
            ));
        }

        // Cycle check over the whole agent → sub-agent graph.
        let edges: Vec<(Uuid, Uuid)> =
            sqlx::query_as("SELECT parent_id, child_id FROM agent_subagents")
                .fetch_all(&self.pool)
                .await?;
        if would_create_cycle(&edges, parent_id, input.child_id) {
            return Err(AppError::validation_field(
                "child_id",
                "attaching this agent would create a cycle in the sub-agent graph",
            ));
        }

        let taken: HashSet<String> = existing.iter().map(|e| e.alias.clone()).collect();
        let alias = match input.alias {
            Some(a) => {
                let a = normalize_alias(&a);
                validate_alias(&a)?;
                if taken.contains(&a) {
                    return Err(AppError::validation_field(
                        "alias",
                        "alias already in use for this agent",
                    ));
                }
                a
            }
            None => unique_default_alias(&slug_from_name(&child_name), &taken),
        };

        let position = existing.len() as i16;
        sqlx::query(
            r#"INSERT INTO agent_subagents (parent_id, child_id, alias, description, position)
               VALUES ($1, $2, $3, $4, $5)"#,
        )
        .bind(parent_id)
        .bind(input.child_id)
        .bind(&alias)
        .bind(input.description.as_deref())
        .bind(position)
        .execute(&self.pool)
        .await?;

        self.get_attached(parent_id, input.child_id).await
    }

    /// Update the alias and/or description of an existing attachment. Omitted
    /// fields are left unchanged.
    pub async fn update(
        &self,
        parent_id: Uuid,
        child_id: Uuid,
        input: UpdateInput,
    ) -> AppResult<AttachedSubagent> {
        let existing = self.list(parent_id).await?;
        let current = existing
            .iter()
            .find(|e| e.child_id == child_id)
            .ok_or_else(|| {
                AppError::NotFound(format!(
                    "sub-agent {child_id} not attached to agent {parent_id}"
                ))
            })?;

        let final_alias = match input.alias {
            Some(a) => {
                let a = normalize_alias(&a);
                validate_alias(&a)?;
                if existing.iter().any(|e| e.child_id != child_id && e.alias == a) {
                    return Err(AppError::validation_field(
                        "alias",
                        "alias already in use for this agent",
                    ));
                }
                a
            }
            None => current.alias.clone(),
        };
        let final_description = match input.description {
            Some(d) => {
                validate_description(&d)?;
                Some(d)
            }
            None => current.description.clone(),
        };

        sqlx::query(
            r#"UPDATE agent_subagents SET alias = $3, description = $4
               WHERE parent_id = $1 AND child_id = $2"#,
        )
        .bind(parent_id)
        .bind(child_id)
        .bind(&final_alias)
        .bind(final_description.as_deref())
        .execute(&self.pool)
        .await?;

        self.get_attached(parent_id, child_id).await
    }

    /// Reorder a parent's sub-agents to match `ordered_child_ids` (a permutation
    /// of the currently attached child ids). Renumbers positions to `0..n`.
    pub async fn reorder(
        &self,
        parent_id: Uuid,
        input: ReorderInput,
    ) -> AppResult<Vec<AttachedSubagent>> {
        let existing = self.list(parent_id).await?;
        let current: HashSet<Uuid> = existing.iter().map(|e| e.child_id).collect();
        let proposed: HashSet<Uuid> = input.ordered_child_ids.iter().copied().collect();
        if input.ordered_child_ids.len() != existing.len() || current != proposed {
            return Err(AppError::validation_field(
                "ordered_child_ids",
                "must be a permutation of the currently attached sub-agent ids",
            ));
        }

        // The (parent_id, position) unique constraint is DEFERRABLE INITIALLY
        // DEFERRED, so we can assign final positions directly inside one
        // transaction without an intermediate-collision workaround.
        let mut tx = self.pool.begin().await?;
        for (idx, child_id) in input.ordered_child_ids.iter().enumerate() {
            sqlx::query(
                "UPDATE agent_subagents SET position = $3 WHERE parent_id = $1 AND child_id = $2",
            )
            .bind(parent_id)
            .bind(child_id)
            .bind(idx as i16)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;

        self.list(parent_id).await
    }

    /// Detach a sub-agent, closing the position gap left behind. Both agents are
    /// otherwise left intact.
    pub async fn detach(&self, parent_id: Uuid, child_id: Uuid) -> AppResult<()> {
        self.ensure_agent_exists(parent_id).await?;

        let mut tx = self.pool.begin().await?;
        let removed =
            sqlx::query("DELETE FROM agent_subagents WHERE parent_id = $1 AND child_id = $2")
                .bind(parent_id)
                .bind(child_id)
                .execute(&mut *tx)
                .await?
                .rows_affected();
        if removed == 0 {
            return Err(AppError::NotFound(format!(
                "sub-agent {child_id} not attached to agent {parent_id}"
            )));
        }

        // Renumber remaining rows ascending to close the gap; iterating in
        // ascending order keeps each new position <= the old one.
        let remaining: Vec<(Uuid, i16)> = sqlx::query_as(
            "SELECT child_id, position FROM agent_subagents WHERE parent_id = $1 ORDER BY position ASC",
        )
        .bind(parent_id)
        .fetch_all(&mut *tx)
        .await?;
        for (idx, (cid, old_pos)) in remaining.iter().enumerate() {
            let new_pos = idx as i16;
            if new_pos == *old_pos {
                continue;
            }
            sqlx::query(
                "UPDATE agent_subagents SET position = $3 WHERE parent_id = $1 AND child_id = $2",
            )
            .bind(parent_id)
            .bind(cid)
            .bind(new_pos)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(())
    }

    async fn get_attached(
        &self,
        parent_id: Uuid,
        child_id: Uuid,
    ) -> AppResult<AttachedSubagent> {
        let row: Option<AttachedSubagent> = sqlx::query_as(
            r#"SELECT s.child_id, a.name, s.alias, s.description, s.position
               FROM agent_subagents s
               JOIN agents a ON a.id = s.child_id
               WHERE s.parent_id = $1 AND s.child_id = $2"#,
        )
        .bind(parent_id)
        .bind(child_id)
        .fetch_optional(&self.pool)
        .await?;
        row.ok_or_else(|| {
            AppError::NotFound(format!(
                "sub-agent {child_id} not attached to agent {parent_id}"
            ))
        })
    }

    async fn ensure_agent_exists(&self, agent_id: Uuid) -> AppResult<()> {
        let exists: Option<Uuid> = sqlx::query_scalar("SELECT id FROM agents WHERE id = $1")
            .bind(agent_id)
            .fetch_optional(&self.pool)
            .await?;
        if exists.is_none() {
            return Err(AppError::NotFound(format!("agent {agent_id}")));
        }
        Ok(())
    }
}

/// Trim and lowercase a user-supplied alias before validation/comparison so that
/// `@Code-Reviewer` and `@code-reviewer` are treated as the same handle.
fn normalize_alias(alias: &str) -> String {
    alias.trim().to_lowercase()
}

/// Enforce the `^[a-z0-9-]{1,30}$` alias rule.
fn validate_alias(alias: &str) -> AppResult<()> {
    let len = alias.chars().count();
    let ok = (1..=ALIAS_MAX).contains(&len)
        && alias
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
    if !ok {
        return Err(AppError::validation_field(
            "alias",
            format!("must match ^[a-z0-9-]{{1,{ALIAS_MAX}}}$"),
        ));
    }
    Ok(())
}

fn validate_description(description: &str) -> AppResult<()> {
    if description.chars().count() > DESCRIPTION_MAX {
        return Err(AppError::validation_field(
            "description",
            format!("must be at most {DESCRIPTION_MAX} characters"),
        ));
    }
    Ok(())
}

/// Kebab-case slug of an agent name, constrained to the alias charset and length.
/// Falls back to `agent` when the name has no usable characters.
fn slug_from_name(name: &str) -> String {
    let mut slug = String::with_capacity(name.len().min(ALIAS_MAX));
    let mut prev_dash = false;
    for c in name.trim().to_lowercase().chars() {
        if c.is_ascii_lowercase() || c.is_ascii_digit() {
            slug.push(c);
            prev_dash = false;
        } else if !prev_dash && !slug.is_empty() {
            slug.push('-');
            prev_dash = true;
        }
    }
    while slug.ends_with('-') {
        slug.pop();
    }
    slug.truncate(ALIAS_MAX);
    while slug.ends_with('-') {
        slug.pop();
    }
    if slug.is_empty() {
        "agent".to_string()
    } else {
        slug
    }
}

/// Pick a default alias from `base`, appending `-N` until it is free.
fn unique_default_alias(base: &str, taken: &HashSet<String>) -> String {
    if !taken.contains(base) {
        return base.to_string();
    }
    for n in 2..1000 {
        let suffix = format!("-{n}");
        let keep = ALIAS_MAX.saturating_sub(suffix.len());
        let trimmed: String = base.chars().take(keep).collect();
        let candidate = format!("{trimmed}{suffix}");
        if !taken.contains(&candidate) {
            return candidate;
        }
    }
    // Practically unreachable given the 10-sub-agent cap.
    base.chars().take(ALIAS_MAX).collect()
}

/// Would adding the edge `parent -> child` close a cycle in the existing graph?
/// True when `parent == child` or a path `child -> … -> parent` already exists.
fn would_create_cycle(edges: &[(Uuid, Uuid)], parent: Uuid, child: Uuid) -> bool {
    if parent == child {
        return true;
    }
    let mut stack = vec![child];
    let mut visited = HashSet::new();
    while let Some(node) = stack.pop() {
        if node == parent {
            return true;
        }
        if !visited.insert(node) {
            continue;
        }
        for (p, c) in edges {
            if *p == node {
                stack.push(*c);
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(n: u8) -> Uuid {
        Uuid::from_bytes([n; 16])
    }

    #[test]
    fn accepts_good_alias() {
        assert!(validate_alias("code-reviewer").is_ok());
        assert!(validate_alias("a").is_ok());
        assert!(validate_alias(&"a".repeat(30)).is_ok());
    }

    #[test]
    fn rejects_bad_alias() {
        assert!(validate_alias("bad alias").is_err()); // space
        assert!(validate_alias("Bad").is_err()); // uppercase
        assert!(validate_alias("").is_err()); // empty
        assert!(validate_alias(&"a".repeat(31)).is_err()); // too long
    }

    #[test]
    fn normalize_lowercases_and_trims() {
        assert_eq!(normalize_alias("  Code-Reviewer "), "code-reviewer");
    }

    #[test]
    fn default_alias_from_name() {
        assert_eq!(slug_from_name("Code Reviewer"), "code-reviewer");
        assert_eq!(slug_from_name("  !!!  "), "agent");
        assert_eq!(slug_from_name("Déjà Vu 2!"), "d-j-vu-2");
    }

    #[test]
    fn unique_default_alias_appends_suffix() {
        let mut taken = HashSet::new();
        taken.insert("reviewer".to_string());
        assert_eq!(unique_default_alias("reviewer", &taken), "reviewer-2");
        let empty = HashSet::new();
        assert_eq!(unique_default_alias("reviewer", &empty), "reviewer");
    }

    #[test]
    fn rejects_self_attach_via_cycle_helper() {
        assert!(would_create_cycle(&[], id(1), id(1)));
    }

    #[test]
    fn rejects_cycle() {
        // A -> B exists; attaching B -> A would close a cycle.
        let edges = vec![(id(1), id(2))];
        assert!(would_create_cycle(&edges, id(2), id(1)));
    }

    #[test]
    fn accepts_dag_diamond() {
        // A->B, A->C exist; adding B->C (diamond) introduces no cycle.
        let edges = vec![(id(1), id(2)), (id(1), id(3))];
        assert!(!would_create_cycle(&edges, id(2), id(3)));
        // And the deeper A->B->D plus A->C, adding C->D is still acyclic.
        let edges = vec![(id(1), id(2)), (id(2), id(4)), (id(1), id(3))];
        assert!(!would_create_cycle(&edges, id(3), id(4)));
    }

    #[test]
    fn detects_transitive_cycle() {
        // A->B, B->C exist; attaching C->A closes a 3-cycle.
        let edges = vec![(id(1), id(2)), (id(2), id(3))];
        assert!(would_create_cycle(&edges, id(3), id(1)));
    }

    #[test]
    fn rejects_long_description() {
        assert!(validate_description(&"x".repeat(201)).is_err());
        assert!(validate_description(&"x".repeat(200)).is_ok());
    }
}
