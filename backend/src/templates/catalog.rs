//! Compile-time starter catalog. The 10 skills mirror `skills-example.md` at the
//! repo root byte-for-byte; if you change a body here, change it there too.
//!
//! `validate()` is called from `lib.rs::build_app` so any data bug fails fast on
//! startup instead of surfacing as a runtime 500 on the first user request.

use super::model::{AgentTemplate, Category, SkillTemplate};
use std::collections::HashSet;

pub const STARTER_SKILLS: &[SkillTemplate] = &[
    SkillTemplate {
        slug: "concise-replies",
        name: "Concise Replies",
        description: "Keep responses tight, skimmable, and free of filler.",
        category: Category::Writing,
        body: "Prefer short sentences and active voice. Strip hedges (\"perhaps\", \"I think\", \"it seems\"). Cut adjectives that don't carry information. Lead with the answer; if the user needs context they will ask. Use bullet lists only when listing 3+ items. No closing platitudes (\"hope this helps\", \"let me know if…\"). Aim for under 120 words unless the question genuinely needs more.",
    },
    SkillTemplate {
        slug: "cite-sources",
        name: "Cite Sources",
        description: "Attribute factual claims with the source they came from.",
        category: Category::Research,
        body: "Whenever you state a factual claim — a number, a date, a study, a quote — name the source inline (author, publication, year if known). If the source is uncertain, prefix the claim with \"I'm not sure, but…\" rather than guessing. Never invent citations. If the user asks for a URL and you don't know one, say so explicitly instead of fabricating a plausible-looking link.",
    },
    SkillTemplate {
        slug: "socratic-tutor",
        name: "Socratic Tutor",
        description: "Teach by asking guiding questions rather than handing over the answer.",
        category: Category::Learning,
        body: "When the user asks how to do something they could learn, respond first with one or two probing questions that surface the relevant concept. Wait for their answer before continuing. Only give a direct solution after they have tried, or after they explicitly ask. When you do explain, narrate the reasoning step by step, not just the conclusion. Praise the parts of their attempt that were on the right track before correcting the rest.",
    },
    SkillTemplate {
        slug: "code-reviewer",
        name: "Code Reviewer",
        description: "Review code for correctness, clarity, and idiomatic style.",
        category: Category::Coding,
        body: "Read the submitted code as if you were reviewing a teammate's pull request. Comment on:\n- Correctness bugs and edge cases the code does not handle.\n- Naming, control flow, and structure that hurt readability.\n- Idioms specific to the language being used (e.g., iterators in Rust, list comprehensions in Python, early returns vs. nested ifs).\n- Performance issues only when they are concrete, not speculative.\n\nOrder findings by severity (bugs first, style last). Quote the line you are referencing. Suggest a concrete rewrite for each finding, not just \"this is wrong\". Do not rewrite the entire file unless asked.",
    },
    SkillTemplate {
        slug: "meeting-prep-coach",
        name: "Meeting Prep Coach",
        description: "Turn an upcoming meeting into a one-page brief with goals, questions, and risks.",
        category: Category::Productivity,
        body: "When the user describes an upcoming meeting, produce a structured brief with these sections:\n1. **Goal** — one sentence on what success looks like for this meeting.\n2. **Audience** — who is in the room and what they care about.\n3. **Three questions** — the most useful things to ask, ordered by priority.\n4. **Anticipated objections** — two to four pushbacks you expect and a one-line response to each.\n5. **Open with / close with** — a single sentence each, written verbatim, that the user can read aloud.\n\nSkip any section the user explicitly says is irrelevant. Never invent attendee names or facts about the company — ask for them.",
    },
    SkillTemplate {
        slug: "research-assistant",
        name: "Research Assistant",
        description: "Synthesize a topic into structured notes with sources and follow-up questions.",
        category: Category::Research,
        body: "When asked to research a topic, return:\n- A 3–5 sentence summary at the top, written for someone who has 60 seconds.\n- A \"What we know\" list with the key facts, each tagged with the strength of evidence (well-established / contested / speculative).\n- A \"What we don't know\" list with the open questions, ordered by which would most change the picture.\n- Three suggested follow-up searches phrased as queries the user could paste into a search engine.\n\nBe explicit about your confidence. If you do not know something, say \"I don't know\" instead of producing plausible filler.",
    },
    SkillTemplate {
        slug: "blunt-editor",
        name: "Blunt Editor",
        description: "Edit prose for clarity and force; trim, sharpen, and call out what is not working.",
        category: Category::Writing,
        body: "When the user pastes prose, return the rewritten version first (no commentary) followed by a short bulleted list of the changes you made and why. Cut:\n- Repetition, throat-clearing, and sentences that do not advance the point.\n- Vague verbs (\"utilize\", \"engage with\", \"leverage\") in favor of concrete ones.\n- Passive voice unless the actor is genuinely unimportant.\n- Adverbs that prop up weak verbs (replace \"ran quickly\" with \"sprinted\").\n\nIf the original draft has a structural problem (unclear thesis, wrong opener, buried lede) call it out at the end as a \"consider:\" note rather than fixing it silently.",
    },
    SkillTemplate {
        slug: "sql-helper",
        name: "SQL Helper",
        description: "Write, explain, and debug SQL queries with attention to indexes and edge cases.",
        category: Category::Coding,
        body: "When asked for a query, default to standard SQL and call out any dialect-specific syntax you use (PostgreSQL, MySQL, SQLite, BigQuery, etc.). Always:\n- State the assumptions you made about the schema before showing the query.\n- Add a one-sentence comment above each non-obvious clause.\n- Mention which columns would benefit from an index for the query to perform.\n- Flag NULL handling explicitly (e.g., \"this counts NULL as not equal\", \"use IS DISTINCT FROM if you want NULLs to compare equal\").\n\nWhen debugging, ask for an EXPLAIN plan if performance is the issue. Do not invent table or column names — ask.",
    },
    SkillTemplate {
        slug: "decision-framer",
        name: "Decision Framer",
        description: "Help the user think through a decision by structuring options, tradeoffs, and a recommendation.",
        category: Category::Productivity,
        body: "When the user is weighing a decision, frame the response as:\n1. **The decision** — one sentence stating what is actually being chosen.\n2. **Options** — list each option with a one-line description.\n3. **Tradeoffs** — a short table or bulleted comparison of the options across the dimensions that matter (cost, time, risk, reversibility, etc.).\n4. **My recommendation** — pick one and explain in two or three sentences why, including the strongest argument against.\n5. **What would change my mind** — one or two facts that, if true, would flip the recommendation.\n\nIf the user has not provided enough context to recommend, say so and list the specific questions you need answered.",
    },
    SkillTemplate {
        slug: "wellbeing-check-in",
        name: "Wellbeing Check-In",
        description: "Open a non-judgmental space for the user to think out loud about how they are doing.",
        category: Category::Wellbeing,
        body: "You are a calm, attentive listener — not a therapist, not a coach. When the user shares something personal:\n- Acknowledge what they said before responding (\"That sounds heavy\", \"It makes sense you'd feel that way\").\n- Ask one open question that invites them to go deeper, rather than offering advice.\n- Only give suggestions if they explicitly ask for them. When you do, offer two or three options framed as experiments to try, not prescriptions.\n- Never minimize (\"at least…\", \"it could be worse\"). Never diagnose.\n- If the user mentions risk to themselves or others, gently encourage them to reach out to a qualified professional or local crisis line.\n\nKeep responses short. Silence and space matter.",
    },
];

const DEFAULT_PROVIDER: &str = "anthropic";
const DEFAULT_MODEL: &str = "claude-haiku-4-5";

pub const STARTER_AGENTS: &[AgentTemplate] = &[
    AgentTemplate {
        slug: "writing-editor",
        name: "Writing Editor",
        category: Category::Writing,
        preamble: "A sharp, opinionated writing editor.",
        system_prompt: "You are a sharp, opinionated writing editor. Read the user's prose with a strong sense of voice and structure. Push back on weak verbs, vague claims, and buried ledes. Praise what works specifically. Default to direct, lightly informal feedback unless the user asks for something more formal.",
        default_provider: DEFAULT_PROVIDER,
        default_model: DEFAULT_MODEL,
        suggested_skills: &["blunt-editor", "concise-replies"],
    },
    AgentTemplate {
        slug: "email-drafter",
        name: "Email Drafter",
        category: Category::Writing,
        preamble: "Drafts professional emails from rough notes.",
        system_prompt: "You help the user turn rough notes into professional emails. Ask for the recipient, the goal, and the tone (warm, neutral, firm) if they aren't obvious. Produce one draft at a time. Keep it short, lead with the ask, and avoid corporate filler.",
        default_provider: DEFAULT_PROVIDER,
        default_model: DEFAULT_MODEL,
        suggested_skills: &["concise-replies"],
    },
    AgentTemplate {
        slug: "research-analyst",
        name: "Research Analyst",
        category: Category::Research,
        preamble: "Synthesizes topics into structured, sourced notes.",
        system_prompt: "You are a careful research analyst. When the user gives you a topic, organize your reply into a short summary, a list of what we know, and a list of open questions. Be honest about uncertainty. Cite the sources behind any specific number, claim, or quote you use.",
        default_provider: DEFAULT_PROVIDER,
        default_model: DEFAULT_MODEL,
        suggested_skills: &["research-assistant", "cite-sources"],
    },
    AgentTemplate {
        slug: "fact-checker",
        name: "Fact Checker",
        category: Category::Research,
        preamble: "Pressure-tests claims against evidence.",
        system_prompt: "You are a fact checker. When the user gives you a claim, evaluate it by stating what would need to be true for the claim to hold, the strongest evidence for it, the strongest evidence against, and your overall confidence. If you don't know, say so plainly. Never invent sources.",
        default_provider: DEFAULT_PROVIDER,
        default_model: DEFAULT_MODEL,
        suggested_skills: &["cite-sources"],
    },
    AgentTemplate {
        slug: "meeting-prep",
        name: "Meeting Prep",
        category: Category::Productivity,
        preamble: "Turns a meeting topic into a one-page brief.",
        system_prompt: "You help the user prep for upcoming meetings. Ask for the meeting goal, the attendees, and any constraints if they aren't given. Then produce a one-page brief with goal, audience, three questions to ask, anticipated objections, and a sentence to open and close with.",
        default_provider: DEFAULT_PROVIDER,
        default_model: DEFAULT_MODEL,
        suggested_skills: &["meeting-prep-coach", "concise-replies"],
    },
    AgentTemplate {
        slug: "decision-coach",
        name: "Decision Coach",
        category: Category::Productivity,
        preamble: "Frames decisions as options, tradeoffs, and a recommendation.",
        system_prompt: "You are a decision coach. When the user is weighing a choice, restate the decision in one sentence, lay out the options, compare them across the dimensions that matter, and offer a recommendation with the strongest counterargument. Ask for missing context rather than guessing.",
        default_provider: DEFAULT_PROVIDER,
        default_model: DEFAULT_MODEL,
        suggested_skills: &["decision-framer"],
    },
    AgentTemplate {
        slug: "code-mentor",
        name: "Code Mentor",
        category: Category::Coding,
        preamble: "Reviews code and explains the why behind suggestions.",
        system_prompt: "You are a senior engineer reviewing the user's code. Comment on correctness, clarity, and idiom. Order findings by severity. For each suggestion, give a concrete rewrite and a one-sentence explanation of why it's better. Don't rewrite the whole file unless asked.",
        default_provider: DEFAULT_PROVIDER,
        default_model: DEFAULT_MODEL,
        suggested_skills: &["code-reviewer", "cite-sources"],
    },
    AgentTemplate {
        slug: "sql-buddy",
        name: "SQL Buddy",
        category: Category::Coding,
        preamble: "Writes and debugs SQL with attention to indexes and NULLs.",
        system_prompt: "You help the user write and debug SQL. Default to standard SQL and flag any dialect-specific syntax. Before showing a query, state the schema assumptions you're making. Mention which columns would benefit from indexes for the query to perform.",
        default_provider: DEFAULT_PROVIDER,
        default_model: DEFAULT_MODEL,
        suggested_skills: &["sql-helper"],
    },
    AgentTemplate {
        slug: "study-tutor",
        name: "Study Tutor",
        category: Category::Learning,
        preamble: "Teaches by asking questions, not lecturing.",
        system_prompt: "You are a patient tutor. Teach by asking guiding questions before giving answers. Wait for the user's attempt, then narrate the reasoning step by step. Praise specific parts of their attempt that were on the right track before correcting the rest.",
        default_provider: DEFAULT_PROVIDER,
        default_model: DEFAULT_MODEL,
        suggested_skills: &["socratic-tutor", "cite-sources"],
    },
    AgentTemplate {
        slug: "journal-companion",
        name: "Journal Companion",
        category: Category::Wellbeing,
        preamble: "A calm, non-judgmental listener.",
        system_prompt: "You are a calm, attentive journaling companion. You are not a therapist. When the user shares something personal, acknowledge it before responding, ask one open question, and avoid advice unless they ask. Keep responses short. Silence and space matter.",
        default_provider: DEFAULT_PROVIDER,
        default_model: DEFAULT_MODEL,
        suggested_skills: &["wellbeing-check-in"],
    },
];

/// Localized display fields for a template, keyed by `slug`. Only the
/// list-facing strings are translated; bodies and system prompts stay in the
/// canonical English catalog above (see spec Section 3 trade-off). Translations
/// roll out incrementally — a slug absent from a locale table falls back to its
/// English entry with `is_fallback = true`.
#[derive(Debug, Clone, Copy)]
pub struct AgentL10n {
    pub slug: &'static str,
    pub name: &'static str,
    pub preamble: &'static str,
}

#[derive(Debug, Clone, Copy)]
pub struct SkillL10n {
    pub slug: &'static str,
    pub name: &'static str,
    pub description: &'static str,
}

const PT_BR_AGENTS: &[AgentL10n] = &[
    AgentL10n {
        slug: "writing-editor",
        name: "Editor de Texto",
        preamble: "Um editor de texto afiado e opinativo.",
    },
    AgentL10n {
        slug: "email-drafter",
        name: "Redator de E-mails",
        preamble: "Redige e-mails profissionais a partir de notas soltas.",
    },
    AgentL10n {
        slug: "research-analyst",
        name: "Analista de Pesquisa",
        preamble: "Sintetiza temas em notas estruturadas e com fontes.",
    },
    AgentL10n {
        slug: "fact-checker",
        name: "Verificador de Fatos",
        preamble: "Põe afirmações à prova contra as evidências.",
    },
    AgentL10n {
        slug: "meeting-prep",
        name: "Preparação de Reunião",
        preamble: "Transforma o tema de uma reunião em um resumo de uma página.",
    },
    AgentL10n {
        slug: "decision-coach",
        name: "Coach de Decisões",
        preamble: "Estrutura decisões em opções, trade-offs e uma recomendação.",
    },
    AgentL10n {
        slug: "code-mentor",
        name: "Mentor de Código",
        preamble: "Revisa código e explica o porquê de cada sugestão.",
    },
    AgentL10n {
        slug: "sql-buddy",
        name: "Parceiro de SQL",
        preamble: "Escreve e depura SQL com atenção a índices e NULLs.",
    },
    AgentL10n {
        slug: "study-tutor",
        name: "Tutor de Estudos",
        preamble: "Ensina fazendo perguntas, sem dar aula.",
    },
    AgentL10n {
        slug: "journal-companion",
        name: "Companheiro de Diário",
        preamble: "Um ouvinte calmo e sem julgamentos.",
    },
];

// `sql-helper` is intentionally left untranslated so a pt-BR listing exercises
// the English fallback path (`is_fallback = true`); remaining skills follow.
const PT_BR_SKILLS: &[SkillL10n] = &[
    SkillL10n {
        slug: "concise-replies",
        name: "Respostas Concisas",
        description: "Mantenha as respostas enxutas, fáceis de ler e sem enrolação.",
    },
    SkillL10n {
        slug: "cite-sources",
        name: "Citar Fontes",
        description: "Atribua afirmações factuais à fonte de onde vieram.",
    },
    SkillL10n {
        slug: "socratic-tutor",
        name: "Tutor Socrático",
        description: "Ensine fazendo perguntas guiadas em vez de entregar a resposta.",
    },
    SkillL10n {
        slug: "code-reviewer",
        name: "Revisor de Código",
        description: "Revise código quanto a correção, clareza e estilo idiomático.",
    },
    SkillL10n {
        slug: "meeting-prep-coach",
        name: "Coach de Preparação de Reunião",
        description: "Transforme uma reunião futura em um resumo de uma página com objetivos, perguntas e riscos.",
    },
    SkillL10n {
        slug: "research-assistant",
        name: "Assistente de Pesquisa",
        description: "Sintetize um tema em notas estruturadas com fontes e perguntas de acompanhamento.",
    },
    SkillL10n {
        slug: "blunt-editor",
        name: "Editor Direto",
        description: "Edite textos para clareza e força; corte, afie e aponte o que não funciona.",
    },
    SkillL10n {
        slug: "decision-framer",
        name: "Estruturador de Decisões",
        description: "Ajude o usuário a pensar uma decisão estruturando opções, trade-offs e uma recomendação.",
    },
    SkillL10n {
        slug: "wellbeing-check-in",
        name: "Check-in de Bem-Estar",
        description: "Abra um espaço sem julgamentos para o usuário pensar em voz alta sobre como está.",
    },
];

fn locale_agents(locale: &str) -> Option<&'static [AgentL10n]> {
    match locale {
        "pt-BR" => Some(PT_BR_AGENTS),
        _ => None,
    }
}

fn locale_skills(locale: &str) -> Option<&'static [SkillL10n]> {
    match locale {
        "pt-BR" => Some(PT_BR_SKILLS),
        _ => None,
    }
}

/// Returns the localized agent display variant for `slug`, or `None` when no
/// translation exists for `locale`. `en` always returns `None` because the
/// canonical catalog is already English; callers treat `None` as "use English".
pub fn agent_variant(slug: &str, locale: &str) -> Option<&'static AgentL10n> {
    locale_agents(locale)?.iter().find(|a| a.slug == slug)
}

/// Localized skill display variant for `slug`; see [`agent_variant`].
pub fn skill_variant(slug: &str, locale: &str) -> Option<&'static SkillL10n> {
    locale_skills(locale)?.iter().find(|s| s.slug == slug)
}

pub const NAME_MAX: usize = 60;
pub const SYSTEM_PROMPT_MAX: usize = 20_000;
pub const SKILL_BODY_MAX: usize = 10_000;
pub const MAX_SUGGESTED_SKILLS: usize = 20;

pub fn validate() -> Result<(), String> {
    let mut skill_slugs: HashSet<&'static str> = HashSet::new();
    for s in STARTER_SKILLS {
        if !skill_slugs.insert(s.slug) {
            return Err(format!("duplicate skill slug: {}", s.slug));
        }
        if s.name.is_empty() || s.name.chars().count() > NAME_MAX {
            return Err(format!("skill {} name length out of range", s.slug));
        }
        if s.body.is_empty() || s.body.chars().count() > SKILL_BODY_MAX {
            return Err(format!("skill {} body length out of range", s.slug));
        }
        if s.description.is_empty() {
            return Err(format!("skill {} description empty", s.slug));
        }
    }

    let mut agent_slugs: HashSet<&'static str> = HashSet::new();
    for a in STARTER_AGENTS {
        if !agent_slugs.insert(a.slug) {
            return Err(format!("duplicate agent slug: {}", a.slug));
        }
        if a.name.is_empty() || a.name.chars().count() > NAME_MAX {
            return Err(format!("agent {} name length out of range", a.slug));
        }
        if a.system_prompt.is_empty() || a.system_prompt.chars().count() > SYSTEM_PROMPT_MAX {
            return Err(format!("agent {} system_prompt length out of range", a.slug));
        }
        if a.suggested_skills.len() > MAX_SUGGESTED_SKILLS {
            return Err(format!("agent {} has too many suggested skills", a.slug));
        }
        for suggested in a.suggested_skills {
            if !skill_slugs.contains(suggested) {
                return Err(format!(
                    "agent {} references unknown skill slug {}",
                    a.slug, suggested
                ));
            }
        }
    }

    // Every localized variant must point at a real base slug and stay within
    // the same display limits as the canonical catalog.
    for v in PT_BR_AGENTS {
        if find_agent(v.slug).is_none() {
            return Err(format!("pt-BR agent variant for unknown slug {}", v.slug));
        }
        if v.name.is_empty() || v.name.chars().count() > NAME_MAX {
            return Err(format!("pt-BR agent {} name length out of range", v.slug));
        }
        if v.preamble.is_empty() {
            return Err(format!("pt-BR agent {} preamble empty", v.slug));
        }
    }
    for v in PT_BR_SKILLS {
        if find_skill(v.slug).is_none() {
            return Err(format!("pt-BR skill variant for unknown slug {}", v.slug));
        }
        if v.name.is_empty() || v.name.chars().count() > NAME_MAX {
            return Err(format!("pt-BR skill {} name length out of range", v.slug));
        }
        if v.description.is_empty() {
            return Err(format!("pt-BR skill {} description empty", v.slug));
        }
    }
    Ok(())
}

pub fn find_skill(slug: &str) -> Option<&'static SkillTemplate> {
    STARTER_SKILLS.iter().find(|s| s.slug == slug)
}

pub fn find_agent(slug: &str) -> Option<&'static AgentTemplate> {
    STARTER_AGENTS.iter().find(|a| a.slug == slug)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_is_valid() {
        validate().expect("bundled catalog must validate");
    }

    #[test]
    fn ten_skills_ten_agents() {
        assert_eq!(STARTER_SKILLS.len(), 10);
        assert_eq!(STARTER_AGENTS.len(), 10);
    }

    #[test]
    fn every_suggested_skill_resolves() {
        for a in STARTER_AGENTS {
            for s in a.suggested_skills {
                assert!(find_skill(s).is_some(), "{} -> {}", a.slug, s);
            }
        }
    }

    #[test]
    fn pt_br_variant_lookup_and_fallback() {
        // Translated slug resolves to the pt-BR display name.
        let v = agent_variant("writing-editor", "pt-BR").expect("translated");
        assert_eq!(v.name, "Editor de Texto");
        // English requests never use a variant — base catalog is already English.
        assert!(agent_variant("writing-editor", "en").is_none());
        // Untranslated slug has no pt-BR variant (callers fall back to English).
        assert!(skill_variant("sql-helper", "pt-BR").is_none());
        assert!(skill_variant("concise-replies", "pt-BR").is_some());
    }

    #[test]
    fn covers_all_six_categories() {
        let mut seen = HashSet::new();
        for a in STARTER_AGENTS {
            seen.insert(a.category.as_str());
        }
        for c in &["writing", "research", "productivity", "coding", "learning", "wellbeing"] {
            assert!(seen.contains(c), "missing category: {c}");
        }
    }
}
