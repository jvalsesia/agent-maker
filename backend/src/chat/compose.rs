use crate::error::{AppError, AppResult};
use crate::llm::{ChatMessage, ChatRole};
use crate::memory::model::{MemoryBlock, RetrievedTurn};

/// The composed prompt ready to dispatch: a system block and the ordered
/// user/assistant message array.
#[derive(Debug, Clone)]
pub struct Composed {
    pub system: String,
    pub messages: Vec<ChatMessage>,
}

/// Build the final prompt from the agent+skills system text and the F08 memory
/// block, reducing memory until it fits the model's character budget.
///
/// Reduction order (per spec §3): drop the lowest-similarity retrieved turns
/// first, then the oldest recent turns (always keeping the trailing user turn).
/// If it still doesn't fit, return `ContextTooLarge`.
pub fn compose(
    base_system: &str,
    block: &MemoryBlock,
    budget_chars: i64,
    reserve_chars: i64,
) -> AppResult<Composed> {
    let effective_budget = (budget_chars - reserve_chars).max(1);

    let mut keep_retrieved = block.retrieved.len();
    let mut recent_start = 0usize;

    loop {
        let system = build_system(base_system, &block.retrieved[..keep_retrieved]);
        let messages = build_messages(&block.recent[recent_start..]);
        let total = system.chars().count() as i64
            + messages
                .iter()
                .map(|m| m.content.chars().count() as i64)
                .sum::<i64>();

        if total <= effective_budget {
            return Ok(Composed { system, messages });
        }

        if keep_retrieved > 0 {
            keep_retrieved -= 1;
        } else if block.recent.len().saturating_sub(recent_start) > 1 {
            recent_start += 1;
        } else {
            return Err(AppError::ContextTooLarge(format!(
                "composed prompt ({total} chars) exceeds the model budget ({effective_budget} chars) even after reducing retrieved and recent turns"
            )));
        }
    }
}

/// Appends the "Earlier relevant context" section to the base system text when
/// any older turns were retrieved.
fn build_system(base_system: &str, retrieved: &[RetrievedTurn]) -> String {
    if retrieved.is_empty() {
        return base_system.to_string();
    }
    let mut s = String::with_capacity(base_system.len() + 256);
    s.push_str(base_system);
    s.push_str(
        "\n\n# Earlier relevant context\nThese earlier turns were retrieved as relevant to the current message (with original timestamps and similarity):\n",
    );
    for turn in retrieved {
        s.push_str(&format!(
            "\n[{ts}] ({role}, similarity {sim:.2}): {content}",
            ts = turn.created_at.to_rfc3339(),
            role = turn.role,
            sim = turn.similarity,
            content = turn.content,
        ));
    }
    s
}

/// Maps recent verbatim turns to provider chat messages.
fn build_messages(recent: &[crate::memory::model::MemoryTurn]) -> Vec<ChatMessage> {
    recent
        .iter()
        .map(|t| ChatMessage {
            role: match t.role.as_str() {
                "assistant" => ChatRole::Assistant,
                "system" => ChatRole::System,
                _ => ChatRole::User,
            },
            content: t.content.clone(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::model::{MemoryTurn, RetrievedTurn};
    use chrono::Utc;
    use uuid::Uuid;

    fn turn(role: &str, content: &str) -> MemoryTurn {
        MemoryTurn { id: Uuid::new_v4(), role: role.into(), content: content.into(), created_at: Utc::now() }
    }
    fn retr(content: &str, sim: f32) -> RetrievedTurn {
        RetrievedTurn {
            id: Uuid::new_v4(),
            role: "user".into(),
            content: content.into(),
            created_at: Utc::now(),
            similarity: sim,
        }
    }
    fn block(recent: Vec<MemoryTurn>, retrieved: Vec<RetrievedTurn>) -> MemoryBlock {
        MemoryBlock {
            conversation_id: Uuid::new_v4(),
            agent_id: Uuid::new_v4(),
            effective_n: 10,
            effective_k: 5,
            recent,
            retrieved,
            degraded: false,
            degraded_reason: None,
        }
    }

    #[test]
    fn composes_system_with_retrieved_and_messages_in_order() {
        let b = block(
            vec![turn("user", "hi"), turn("assistant", "hello"), turn("user", "what now?")],
            vec![retr("earlier important fact", 0.9)],
        );
        let out = compose("AGENT PROMPT + SKILLS", &b, 1_000_000, 0).unwrap();
        assert!(out.system.starts_with("AGENT PROMPT + SKILLS"));
        assert!(out.system.contains("Earlier relevant context"));
        assert!(out.system.contains("earlier important fact"));
        assert_eq!(out.messages.len(), 3);
        assert!(matches!(out.messages[0].role, ChatRole::User));
        assert!(matches!(out.messages[1].role, ChatRole::Assistant));
        assert_eq!(out.messages.last().unwrap().content, "what now?");
    }

    #[test]
    fn no_retrieved_means_no_context_section() {
        let b = block(vec![turn("user", "hi")], vec![]);
        let out = compose("BASE", &b, 1_000_000, 0).unwrap();
        assert_eq!(out.system, "BASE");
    }

    #[test]
    fn reduces_retrieved_then_recent_to_fit_budget() {
        // Tight budget forces dropping retrieved turns first, then old recent turns.
        let b = block(
            vec![turn("user", "aaaa"), turn("assistant", "bbbb"), turn("user", "cccc")],
            vec![retr("xxxxxxxx", 0.9), retr("yyyyyyyy", 0.5)],
        );
        let out = compose("SYS", &b, 40, 0).unwrap();
        // Must always keep the trailing user turn.
        assert_eq!(out.messages.last().unwrap().content, "cccc");
    }

    #[test]
    fn errors_when_even_minimal_prompt_exceeds_budget() {
        let b = block(vec![turn("user", "a very long trailing user message that cannot be trimmed")], vec![]);
        let err = compose("HUGE SYSTEM PROMPT", &b, 10, 0).unwrap_err();
        assert!(matches!(err, AppError::ContextTooLarge(_)));
    }
}
