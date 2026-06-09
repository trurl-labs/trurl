use serde_json::Value;

use crate::store::ProjectState;
use crate::store::graph::InMemoryGraph;
use crate::store::schema::DecisionFile;

use super::context;
use crate::workflow::concerns;

// ── Types ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DesignMode {
    Full,
    Quick,
    Learn,
    Review,
}

impl DesignMode {
    pub fn parse(s: &str) -> Result<Self, String> {
        match s {
            "full" => Ok(Self::Full),
            "quick" => Ok(Self::Quick),
            "learn" => Ok(Self::Learn),
            "review" => Ok(Self::Review),
            _ => Err(format!(
                "invalid mode `{s}` — expected: full, quick, learn, review"
            )),
        }
    }

    fn token_budget(self) -> &'static str {
        match self {
            Self::Full => "thorough",
            Self::Quick => "compact",
            Self::Learn => "standard",
            Self::Review => "standard",
        }
    }
}

// ── Public API ──────────────────────────────────────────────────────────────

pub(crate) fn build_design_prompt(
    state: &ProjectState,
    component: &str,
    task: Option<&str>,
    mode: DesignMode,
) -> Result<Value, String> {
    let graph = &state.graph;

    if component != "project" && !state.components.contains_key(component) {
        return Err(format!("component `{component}` does not exist"));
    }

    let ctx = context::get_context(state, component, task, context::ContextDepth::Full)?;

    let instructions = match mode {
        DesignMode::Full => build_full_instructions(graph, component, task),
        DesignMode::Quick => build_quick_instructions(graph, component, task),
        DesignMode::Learn => build_learn_instructions(graph, component),
        DesignMode::Review => build_review_instructions(graph, component),
    };

    Ok(serde_json::json!({
        "system_instructions": instructions,
        "context": ctx,
        "token_budget": mode.token_budget(),
    }))
}

// ── Full mode ───────────────────────────────────────────────────────────────

fn build_full_instructions(graph: &InMemoryGraph, component: &str, task: Option<&str>) -> String {
    let mut out = String::with_capacity(2048);

    out.push_str(&format!(
        "You are running a Trurl design session for component [{component}].\n\n"
    ));

    // ── Source code mandate ─────────────────────────────────────────
    out.push_str(
        "MANDATORY FIRST STEP — READ THE SOURCE CODE:\n\
         Before asking any questions, use your file-reading tools to read the \
         actual source files for this component. Do NOT rely on README or \
         documentation — read the implementation. The source code is truth.\n\n",
    );

    // Existing constraints.
    let project_rules = graph.project_decisions();
    let existing = graph.decisions_for(component);
    if !project_rules.is_empty() || !existing.is_empty() {
        out.push_str("EXISTING CONSTRAINTS (do not re-discuss):\n");
        for (_, d) in &project_rules {
            out.push_str(&format!("- {} (project rule)\n", d.decision.choice));
        }
        for (_, d) in &existing {
            out.push_str(&format!(
                "- {} ({})\n",
                d.decision.choice, d.decision.reason
            ));
        }
        out.push('\n');
    }

    // Connected components for context.
    let connects_to = graph.connects_to(component);
    let connects_from = graph.connects_from(component);
    if !connects_to.is_empty() || !connects_from.is_empty() {
        out.push_str("COMPONENT GRAPH:\n");
        for c in &connects_to {
            out.push_str(&format!("- {component} → {c}\n"));
        }
        for c in &connects_from {
            out.push_str(&format!("- {c} → {component}\n"));
        }
        out.push('\n');
    }

    if let Some(t) = task {
        out.push_str(&format!("TASK CONTEXT: {t}\n\n"));
    }

    // Scope boundary.
    if component == "project" {
        out.push_str(
            "SCOPE — PROJECT LEVEL:\n\
             Project decisions are cross-cutting principles: error strategy, \
             coding standards, security posture, dependency policy, build \
             configuration. Do NOT record component-specific implementation \
             details here.\n\n",
        );
    } else {
        out.push_str(&format!(
            "SCOPE — COMPONENT [{component}]:\n\
             Record only decisions specific to this component. Cross-cutting \
             principles belong at project scope.\n\n"
        ));
    }

    // Dynamic concern tracking.
    let all_decs: Vec<&DecisionFile> = project_rules
        .iter()
        .chain(existing.iter())
        .map(|(_, d)| *d)
        .collect();
    out.push_str(&concerns::concern_status(&all_decs));

    // Decision count context — no hardcoded numbers.
    let n_existing = existing.len();
    out.push_str(&format!(
        "CURRENT STATE: {n_existing} decisions recorded.\n\
         The number of decisions is NOT predetermined — it depends on the \
         component's complexity. Stop when every concern area is covered and \
         the user can articulate the design, not when you reach a count.\n\n",
    ));

    out.push_str(
        "PHASE 1 — SCOPE:\n\
         Ask ONE question at a time. Wait for the answer before asking the next.\n\
         1. \"What is this component responsible for?\"\n\
         2. \"What is explicitly NOT its responsibility?\"\n\
         After each answer, call record_decision with tags: [\"scope\"].\n\n\
         PHASE 2 — TECHNICAL CHOICES:\n\
         Work through each UNCOVERED concern above. For each:\n\
         - Read the relevant source code for that concern area\n\
         - Present 2-3 viable options with trade-offs\n\
         - Ask the user to choose\n\
         - Call record_decision with tags: [concern_area_lowercase, specific_topic]\n\
           Example tags: [\"concurrency\", \"file-locking\"] or [\"security\", \"path-validation\"]\n\
           Tags are REQUIRED — they enable server-side pattern detection.\n\
         - If the component has domain-specific concerns not in the list, ask about those too.\n\n\
         COMPREHENSION GATE (after each decision):\n\
         Ask the USER to state one concrete implication:\n\
           \"What does this decision mean in practice? Give me one specific scenario.\"\n\
         The USER must articulate the implication — not you.\n\
         If their answer is CORRECT BUT SHALLOW (e.g. a one-sentence restatement):\n\
           You are a senior engineer. Expand their understanding:\n\
           - Explain the deeper implications they didn't mention\n\
           - Give a concrete failure scenario this decision prevents\n\
           - Connect it to other decisions already recorded\n\
           Then: \"Does that deepen your understanding? Can you add to \
           what you said?\"\n\
         If WRONG: correct them, explain clearly, and ask again.\n\
         Do not record the decision until the user demonstrates understanding.\n\n\
         \"I DON'T KNOW\" PROTOCOL:\n\
         When the user says \"I don't know\" or gives a ≤3 word answer:\n\
         You are a senior engineer teaching. Do explain:\n\
         1. Describe what the code does and why the decision matters (3-4 sentences)\n\
         2. Give one concrete failure scenario if the decision were different\n\
         3. Then ask: \"Can you restate that in your own words?\"\n\
         Record only after they demonstrate understanding.\n\
         This is a teaching moment, not a test.\n\n\
         PATTERN DETECTION:\n\
         If record_decision returns a pattern_opportunity field (non-null),\n\
         present it to the user. If they confirm, call record_pattern\n\
         immediately with the listed decision names. Do not defer.\n\n\
         PHASE 3 — COVERAGE CHECK:\n\
         Before proceeding to the summary, verify you have addressed the\n\
         UNCOVERED concerns listed above. If more than half remain uncovered,\n\
         go back and probe each one.\n\n\
         PHASE 4 — SUMMARY CHECKPOINT:\n\
         Ask: \"Without looking at the list, describe in 3-5 sentences the \
         constraints any code touching this component must respect.\"\n\
         Do NOT help. Do NOT break it into sub-questions. Do NOT give hints.\n\
         If the user cannot produce a coherent summary, the comprehension\n\
         gates were insufficient — revisit the decisions they could not explain.\n\
         The session ends only when the user demonstrates ownership.\n",
    );

    out
}

// ── Quick mode ──────────────────────────────────────────────────────────────

fn build_quick_instructions(graph: &InMemoryGraph, component: &str, task: Option<&str>) -> String {
    let mut out = String::with_capacity(512);

    out.push_str(&format!(
        "You are running a quick Trurl design check for [{component}].\n\n"
    ));

    let existing = graph.decisions_for(component);
    let project_rules = graph.project_decisions();

    if !existing.is_empty() || !project_rules.is_empty() {
        out.push_str("ACTIVE CONSTRAINTS (the user must confirm each applies):\n");
        for (name, d) in &project_rules {
            out.push_str(&format!(
                "- [project] {}: {} ({})\n",
                name, d.decision.choice, d.decision.reason
            ));
        }
        for (name, d) in &existing {
            out.push_str(&format!(
                "- {}: {} ({})\n",
                name, d.decision.choice, d.decision.reason
            ));
        }
        out.push('\n');
    }

    if let Some(t) = task {
        out.push_str(&format!("TASK: {t}\n\n"));
    }

    out.push_str(
        "FIRST: Read the source code relevant to this task before asking \
         questions. The code is truth.\n\n\
         STEP 1 — CONFIRM EXISTING CONSTRAINTS:\n\
         Present the constraints above and ask:\n\
         \"This task touches this component. Here are the existing constraints.\n\
         Do all of these still apply, or does something need to change?\"\n\
         Wait for the user to confirm or flag changes.\n\
         If changes → call update_decision (amend or supersede) for each.\n\n\
         STEP 2 — CHECK FOR NEW DECISIONS:\n\
         Ask: \"Does this task introduce any new architectural choice not\n\
         covered by the constraints above?\"\n\
         If NO → the session is complete. Call advance to check readiness.\n\
         If YES → ask 1-3 targeted questions, call record_decision for each.\n\n\
         COMPREHENSION GATE (only if any decision was changed or added):\n\
         Ask the user: \"What does this change mean in practice?\"\n\
         The user must articulate the implication — not you.\n\
         If their answer is correct but shallow, expand their understanding \
         as a senior engineer would — deeper implications, failure scenarios, \
         connections to other decisions — then confirm they follow.\n",
    );

    out
}

// ── Learn mode ──────────────────────────────────────────────────────────────

fn build_learn_instructions(graph: &InMemoryGraph, component: &str) -> String {
    let mut out = String::with_capacity(2048);

    out.push_str(&format!(
        "You are running a Trurl learn session for [{component}].\n\
         The goal is understanding AND capturing — every implicit decision \
         in the code becomes an explicit, recorded decision.\n\n"
    ));

    // ── Source code mandate ─────────────────────────────────────────
    out.push_str(
        "MANDATORY FIRST STEP — READ THE SOURCE CODE:\n\
         Use your file-reading tools to read the actual source files for \
         this component. Do NOT rely on README, documentation, or comments \
         about the code — read the implementation. The source code is truth.\n\n\
         For project scope: read Cargo.toml, the crate root (lib.rs or \
         main.rs), and any configuration files.\n\
         For components: read every source file in the component's module.\n\n",
    );

    let decisions = graph.decisions_for(component);
    let project_rules = graph.project_decisions();
    let patterns = graph.patterns_for(component);

    // ── Existing decisions context ──────────────────────────────────
    if !project_rules.is_empty() || !decisions.is_empty() {
        out.push_str("ALREADY RECORDED (do not re-record these):\n");
        for (name, d) in &project_rules {
            out.push_str(&format!("  [project] {}: {}\n", name, d.decision.choice));
        }
        for (name, d) in &decisions {
            out.push_str(&format!("  {}: {}\n", name, d.decision.choice));
        }
        out.push('\n');
    }

    // ── Analysis phase ──────────────────────────────────────────────
    out.push_str(
        "STEP 1 — ANALYZE THE CODE:\n\
         After reading the source, build a numbered list of every \
         architectural decision you can identify. Look for:\n\
         - Data structures and why they were chosen\n\
         - Error handling strategy (Result types, error enums, panics)\n\
         - Concurrency primitives (locks, channels, atomics, async)\n\
         - Validation and integrity checks\n\
         - Performance-sensitive paths (caching, batching, allocation)\n\
         - External boundaries (protocols, serialization, I/O)\n\
         - Security measures (input validation, secret handling, trust boundaries)\n\
         - Storage strategy (file layout, formats, atomicity)\n\
         - Dependency choices (why this crate, not that one)\n\
         - What the code explicitly does NOT do (scope boundaries)\n\n\
         Present this list to the user: \"I found N architectural decisions \
         in the source code. Let me go through each one.\"\n\
         The list drives the session — do not stop until every identified \
         decision has been discussed.\n\n",
    );

    // ── Scope boundary ──────────────────────────────────────────────
    if component == "project" {
        out.push_str(
            "SCOPE — PROJECT LEVEL:\n\
             Project decisions are cross-cutting principles that apply \
             everywhere: error strategy, coding standards, security posture, \
             dependency policy, build configuration. If a decision is specific \
             to one component's implementation, it belongs on that component, \
             not here.\n\n",
        );
    } else {
        out.push_str(&format!(
            "SCOPE — COMPONENT [{component}]:\n\
             Record only decisions specific to this component's implementation. \
             Cross-cutting principles (error strategy, coding standards) belong \
             at project scope. If a decision applies to multiple components \
             equally, it is a project rule.\n\n"
        ));
    }

    // ── Walkthrough protocol ────────────────────────────────────────
    out.push_str(
        "STEP 2 — SYSTEMATIC WALKTHROUGH:\n\
         Present ONE decision per message. After asking any question, STOP \
         and wait for the user's response. Do not continue to the next \
         decision in the same message.\n\n\
         For each decision from your analysis:\n\
         1. State what you found in the code (cite the specific file/function)\n\
         2. Explain WHY this is an architectural decision, not just an \
            implementation detail — what would break if someone changed it?\n\
         3. Ask the user to confirm or correct your understanding\n\
         4. STOP. Wait for their response.\n\n\
         WHEN THE USER'S ANSWER IS CORRECT BUT SHALLOW:\n\
         Do not just move on. You are a senior engineer mentoring. Expand:\n\
         - Explain the deeper implications they may not have considered\n\
         - Give a concrete scenario where this decision prevents a real problem\n\
         - Connect it to other decisions in the graph\n\
         Then ask: \"Does that match your reasoning, or was there something \
         else driving this choice?\"\n\
         STOP. Wait for their response before moving on.\n\n\
         WHEN THE USER SAYS \"I DON'T KNOW\":\n\
         This is a teaching moment, not a failure. Explain it:\n\
         1. Describe what the code does and why it matters (2-4 sentences)\n\
         2. Give one concrete failure scenario if the decision were different\n\
         3. Ask: \"Can you restate that in your own words?\"\n\
         STOP. Wait. Record the decision after they demonstrate understanding.\n\n\
         After each confirmed decision, call record_decision immediately. \
         Include specific tags matching the concern area (e.g. [\"concurrency\", \
         \"file-locking\"]). Tags are required.\n\n",
    );

    // ── Existing decision review ────────────────────────────────────
    if !decisions.is_empty() {
        out.push_str(
            "STEP 2.5 — REVIEW ALREADY-RECORDED DECISIONS:\n\
             Before probing for new ones, briefly confirm each existing \
             decision still matches the code:\n",
        );
        for (name, d) in &decisions {
            out.push_str(&format!(
                "  {}: {} — ask: \"Is this still accurate?\"\n",
                name, d.decision.choice,
            ));
        }
        out.push_str(
            "If any are outdated → call update_decision(mode=\"supersede\").\n\
             Then continue identifying NEW decisions from the code analysis.\n\n",
        );
    }

    // ── Patterns ────────────────────────────────────────────────────
    if !patterns.is_empty() {
        out.push_str("EXISTING PATTERNS:\n");
        for (name, p) in &patterns {
            out.push_str(&format!("- {}: {}\n", name, p.pattern.description));
        }
        out.push('\n');
    }

    // ── Completion ──────────────────────────────────────────────────
    out.push_str(
        "STEP 3 — COMPLETENESS CHECK:\n\
         After all identified decisions are recorded, ask:\n\
         \"I've captured N decisions from the source code. Are there \
         architectural choices I missed — things that aren't obvious from \
         reading the code but that you decided deliberately?\"\n\n\
         If record_decision returns a pattern_opportunity field (non-null), \
         present it to the user. If they confirm, call record_pattern \
         immediately.\n\n\
         STEP 4 — PATTERN IDENTIFICATION:\n\
         Review all recorded decisions for this component (and project rules). \
         Look for groups of 2+ decisions that share a common theme, reinforce \
         the same invariant, or form an interlocking guarantee. Examples:\n\
         - Multiple security decisions forming a defense-in-depth chain\n\
         - Multiple integrity decisions forming a consistency guarantee\n\
         - Multiple performance decisions forming a latency budget\n\
         For each candidate pattern, ask the user: \"These N decisions seem \
         to form a pattern — [describe it]. Should I record this as a pattern?\"\n\
         If confirmed, call record_pattern with the decision names.\n\n\
         The session ends when the user confirms all decisions and patterns \
         are captured. Do not proceed to implementation.\n",
    );

    out
}

// ── Review mode ─────────────────────────────────────────────────────────────

fn build_review_instructions(graph: &InMemoryGraph, component: &str) -> String {
    let mut out = String::with_capacity(1024);

    out.push_str(&format!(
        "You are running a Trurl design review for [{component}].\n\
         Challenge each decision against the current source code.\n\n"
    ));

    out.push_str(
        "MANDATORY FIRST STEP — READ THE SOURCE CODE:\n\
         Read the current source files for this component. Compare what \
         the code actually does against each recorded decision. Decisions \
         may have drifted from the implementation.\n\n",
    );

    let mut decisions: Vec<_> = graph.decisions_for(component);
    // Sort oldest-first by created timestamp.
    decisions.sort_by_key(|(_, d)| d.decision.created);

    if decisions.is_empty() {
        out.push_str("No decisions to review for this component.\n");
        return out;
    }

    out.push_str("Review each decision oldest-first:\n\n");

    for (name, d) in &decisions {
        out.push_str(&format!(
            "DECISION: {} (created {})\n  {}: {}\n",
            name,
            d.decision.created.format("%Y-%m-%d"),
            d.decision.choice,
            d.decision.reason,
        ));
        out.push_str(
            "  → First: verify this matches the current source code\n\
             → Ask: \"In your own words, why does this still hold? \
             Or has something changed?\"\n",
        );
        out.push_str(
            "  The user must articulate why — \"yes\" is not sufficient.\n\
             If their answer is correct but shallow, explain the deeper \
             implications as a senior engineer would.\n\
             If they say it no longer holds → call update_decision \
             with mode=\"supersede\".\n\n",
        );
    }

    out.push_str(
        "After all decisions reviewed, check for any architectural choices \
         in the current source code that are NOT captured as decisions. \
         If found, ask the user about each and call record_decision.\n\n\
         Finally, call validate_consistency to check graph health.\n",
    );

    out
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::schema::*;
    use chrono::{TimeZone, Utc};
    use std::collections::BTreeMap;

    fn test_state() -> ProjectState {
        let mut components = BTreeMap::new();
        components.insert(
            "auth".into(),
            ComponentFile {
                component: Component {
                    name: "auth".into(),
                    description: "Authentication".into(),
                },
            },
        );
        components.insert(
            "database".into(),
            ComponentFile {
                component: Component {
                    name: "database".into(),
                    description: "Database layer".into(),
                },
            },
        );

        let ts = Utc.with_ymd_and_hms(2025, 1, 15, 10, 0, 0).unwrap();
        let mut decisions = BTreeMap::new();
        decisions.insert(
            "use-jwt".into(),
            DecisionFile {
                decision: Decision {
                    component: "auth".into(),
                    choice: "JWT with DPoP binding".into(),
                    reason: "Stateless, no session store".into(),
                    alternatives: vec!["Session cookies — rejected: server state".into()],
                    tags: vec![],
                    created: ts,
                },
            },
        );
        decisions.insert(
            "error-strategy".into(),
            DecisionFile {
                decision: Decision {
                    component: "project".into(),
                    choice: "Result<T, AppError>".into(),
                    reason: "Consistent errors".into(),
                    alternatives: vec![],
                    tags: vec![],
                    created: Utc.with_ymd_and_hms(2025, 1, 10, 8, 0, 0).unwrap(),
                },
            },
        );

        let graph_index = GraphIndex {
            version: 1,
            rebuilt: ts,
            nodes: vec![
                NodeEntry {
                    name: "project".into(),
                    kind: NodeKind::Component,
                    tags: vec![],
                    hash: String::new(),
                },
                NodeEntry {
                    name: "auth".into(),
                    kind: NodeKind::Component,
                    tags: vec![],
                    hash: String::new(),
                },
                NodeEntry {
                    name: "database".into(),
                    kind: NodeKind::Component,
                    tags: vec![],
                    hash: String::new(),
                },
                NodeEntry {
                    name: "use-jwt".into(),
                    kind: NodeKind::Decision,
                    tags: vec![],
                    hash: String::new(),
                },
                NodeEntry {
                    name: "error-strategy".into(),
                    kind: NodeKind::Decision,
                    tags: vec![],
                    hash: String::new(),
                },
            ],
            edges: vec![
                EdgeEntry {
                    from: "auth".into(),
                    to: "database".into(),
                    kind: EdgeKind::ConnectsTo,
                },
                EdgeEntry {
                    from: "use-jwt".into(),
                    to: "auth".into(),
                    kind: EdgeKind::BelongsTo,
                },
                EdgeEntry {
                    from: "error-strategy".into(),
                    to: "project".into(),
                    kind: EdgeKind::BelongsTo,
                },
            ],
        };

        ProjectState::new(
            ProjectFile {
                trurl_version: FORMAT_VERSION.into(),
                project: Project {
                    name: "test-project".into(),
                    description: String::new(),
                },
            },
            components,
            decisions,
            BTreeMap::new(),
            graph_index,
        )
    }

    #[test]
    fn design_mode_parse() {
        assert_eq!(DesignMode::parse("full").unwrap(), DesignMode::Full);
        assert_eq!(DesignMode::parse("quick").unwrap(), DesignMode::Quick);
        assert_eq!(DesignMode::parse("learn").unwrap(), DesignMode::Learn);
        assert_eq!(DesignMode::parse("review").unwrap(), DesignMode::Review);
        assert!(DesignMode::parse("bogus").is_err());
    }

    #[test]
    fn full_mode_has_all_phases() {
        let state = test_state();
        let result =
            build_design_prompt(&state, "auth", Some("add rate limiting"), DesignMode::Full)
                .unwrap();
        let instructions = result["system_instructions"].as_str().unwrap();

        assert!(instructions.contains("PHASE 1"));
        assert!(instructions.contains("PHASE 2"));
        assert!(instructions.contains("PHASE 3"));
        assert!(instructions.contains("PHASE 4"));
        assert!(instructions.contains("EXISTING CONSTRAINTS"));
        assert!(instructions.contains("Result<T, AppError>"));
        assert!(instructions.contains("JWT with DPoP"));
        assert!(instructions.contains("auth → database"));
        assert!(instructions.contains("rate limiting"));
        assert_eq!(result["token_budget"], "thorough");
    }

    #[test]
    fn full_mode_has_concern_checklist() {
        let state = test_state();
        let result = build_design_prompt(&state, "auth", None, DesignMode::Full).unwrap();
        let instructions = result["system_instructions"].as_str().unwrap();

        // Auth has one decision (JWT with DPoP) — some concerns should be
        // COVERED and others UNCOVERED.
        assert!(
            instructions.contains("COVERED") || instructions.contains("UNCOVERED"),
            "should have dynamic concern status"
        );
        assert!(instructions.contains("COMPREHENSION GATE"));
        assert!(instructions.contains("I DON'T KNOW"));
        assert!(instructions.contains("PATTERN DETECTION"));
        assert!(instructions.contains("COVERAGE CHECK"));
    }

    #[test]
    fn full_mode_concern_status_reflects_existing_decisions() {
        let state = test_state();
        let result = build_design_prompt(&state, "auth", None, DesignMode::Full).unwrap();
        let instructions = result["system_instructions"].as_str().unwrap();

        // "JWT with DPoP binding" should trigger Security boundaries concern.
        // "Stateless, no session store" has no direct keyword match for other concerns.
        // Project rule "Result<T, AppError>" should trigger Error handling concern.
        assert!(
            instructions.contains("COVERED"),
            "should show covered concerns: {instructions}"
        );
        assert!(
            instructions.contains("UNCOVERED"),
            "should show uncovered concerns for remaining areas: {instructions}"
        );
    }

    #[test]
    fn full_mode_empty_component_all_uncovered() {
        let state = test_state();
        // database has no decisions in this fixture.
        let result = build_design_prompt(&state, "database", None, DesignMode::Full).unwrap();
        let instructions = result["system_instructions"].as_str().unwrap();

        // Project-wide "Result<T, AppError>" covers Error handling.
        // Everything else should be UNCOVERED.
        assert!(
            instructions.contains("UNCOVERED"),
            "mostly-empty component should have many uncovered concerns"
        );
    }

    #[test]
    fn full_mode_has_decision_count_and_user_gate() {
        let state = test_state();
        let result = build_design_prompt(&state, "auth", None, DesignMode::Full).unwrap();
        let instructions = result["system_instructions"].as_str().unwrap();

        // Decision count guidance.
        assert!(
            instructions.contains("CURRENT STATE:"),
            "should include decision count context"
        );
        assert!(
            instructions.contains("decisions recorded"),
            "should show how many decisions exist"
        );
        // Gate flipped: user articulates, not agent.
        assert!(
            instructions.contains("The USER must articulate"),
            "comprehension gate must require user to explain, not agent"
        );
        // Mandatory tags.
        assert!(
            instructions.contains("Tags are REQUIRED"),
            "tags instruction must be mandatory"
        );
    }

    #[test]
    fn full_mode_mandates_source_reading() {
        let state = test_state();
        let result = build_design_prompt(&state, "auth", None, DesignMode::Full).unwrap();
        let instructions = result["system_instructions"].as_str().unwrap();
        assert!(
            instructions.contains("READ THE SOURCE CODE"),
            "full mode must mandate reading source code before asking questions"
        );
    }

    #[test]
    fn full_mode_explains_shallow_answers() {
        let state = test_state();
        let result = build_design_prompt(&state, "auth", None, DesignMode::Full).unwrap();
        let instructions = result["system_instructions"].as_str().unwrap();
        assert!(
            instructions.contains("CORRECT BUT SHALLOW"),
            "comprehension gate must handle correct-but-shallow answers"
        );
        assert!(
            instructions.contains("senior engineer"),
            "agent must explain deeply like a senior engineer"
        );
    }

    #[test]
    fn full_mode_teaches_on_i_dont_know() {
        let state = test_state();
        let result = build_design_prompt(&state, "auth", None, DesignMode::Full).unwrap();
        let instructions = result["system_instructions"].as_str().unwrap();
        assert!(
            instructions.contains("I DON'T KNOW"),
            "must have I DON'T KNOW protocol"
        );
        assert!(
            instructions.contains("teaching moment"),
            "I DON'T KNOW should be a teaching moment, not a block"
        );
    }

    #[test]
    fn project_scope_has_boundary_guidance() {
        let state = test_state();
        let result = build_design_prompt(&state, "project", None, DesignMode::Full).unwrap();
        let instructions = result["system_instructions"].as_str().unwrap();
        assert!(
            instructions.contains("cross-cutting"),
            "project scope must explain what belongs at project level"
        );
    }

    #[test]
    fn component_scope_has_boundary_guidance() {
        let state = test_state();
        let result = build_design_prompt(&state, "auth", None, DesignMode::Full).unwrap();
        let instructions = result["system_instructions"].as_str().unwrap();
        assert!(
            instructions.contains("SCOPE"),
            "component scope must have boundary guidance"
        );
    }

    #[test]
    fn full_mode_no_hardcoded_decision_count() {
        let state = test_state();
        let result = build_design_prompt(&state, "auth", None, DesignMode::Full).unwrap();
        let instructions = result["system_instructions"].as_str().unwrap();
        assert!(
            !instructions.contains("10-20") && !instructions.contains("3-8"),
            "must not have hardcoded decision count guidance"
        );
        assert!(
            instructions.contains("NOT predetermined"),
            "should say decision count depends on complexity"
        );
    }

    #[test]
    fn quick_mode_compact() {
        let state = test_state();
        let result = build_design_prompt(&state, "auth", None, DesignMode::Quick).unwrap();
        let instructions = result["system_instructions"].as_str().unwrap();

        assert!(instructions.contains("quick"));
        assert!(instructions.contains("ACTIVE CONSTRAINTS"));
        assert!(instructions.contains("JWT with DPoP"));
        assert!(instructions.contains("CONFIRM EXISTING"));
        assert!(instructions.contains("CHECK FOR NEW"));
        assert!(!instructions.contains("PHASE 1"));
        assert_eq!(result["token_budget"], "compact");
    }

    #[test]
    fn quick_mode_includes_project_rules() {
        let state = test_state();
        let result = build_design_prompt(&state, "auth", None, DesignMode::Quick).unwrap();
        let instructions = result["system_instructions"].as_str().unwrap();

        assert!(
            instructions.contains("[project]"),
            "quick mode should show project rules for confirmation"
        );
        assert!(instructions.contains("Result<T, AppError>"));
    }

    #[test]
    fn learn_mode_mandates_source_reading() {
        let state = test_state();
        let result = build_design_prompt(&state, "auth", None, DesignMode::Learn).unwrap();
        let instructions = result["system_instructions"].as_str().unwrap();

        assert!(instructions.contains("learn session"));
        assert!(
            instructions.contains("READ THE SOURCE CODE"),
            "learn mode must mandate reading source code, not README"
        );
        assert!(
            instructions.contains("ANALYZE THE CODE"),
            "learn mode must drive code analysis before asking questions"
        );
        // Existing decisions shown for context.
        assert!(instructions.contains("JWT with DPoP"));
        assert_eq!(result["token_budget"], "standard");
    }

    #[test]
    fn review_mode_oldest_first() {
        let state = test_state();
        let result = build_design_prompt(&state, "auth", None, DesignMode::Review).unwrap();
        let instructions = result["system_instructions"].as_str().unwrap();

        assert!(instructions.contains("review"));
        assert!(instructions.contains("still hold"));
        assert!(instructions.contains("validate_consistency"));
        // Review must require articulation, not just yes/no.
        assert!(
            instructions.contains("must articulate") || instructions.contains("not sufficient"),
            "review mode must require the user to articulate, not just confirm"
        );
        assert_eq!(result["token_budget"], "standard");
    }

    #[test]
    fn context_included_in_output() {
        let state = test_state();
        let result = build_design_prompt(&state, "auth", None, DesignMode::Full).unwrap();
        // Context has the same shape as get_context output.
        assert_eq!(result["context"]["component"]["name"], "auth");
        assert!(result["context"]["brief"].is_string());
    }

    #[test]
    fn rejects_nonexistent_component() {
        let state = test_state();
        let err = build_design_prompt(&state, "ghost", None, DesignMode::Full).unwrap_err();
        assert!(err.contains("ghost"));
    }

    #[test]
    fn project_works() {
        let state = test_state();
        let result = build_design_prompt(&state, "project", None, DesignMode::Full).unwrap();
        assert_eq!(result["context"]["component"]["name"], "project");
    }

    #[test]
    fn no_workflow_hints_in_response() {
        let state = test_state();
        for mode in [
            DesignMode::Full,
            DesignMode::Quick,
            DesignMode::Learn,
            DesignMode::Review,
        ] {
            let result = build_design_prompt(&state, "auth", None, mode).unwrap();
            assert!(
                result.get("workflow").is_none(),
                "advance owns workflow — design prompt must not carry hints"
            );
        }
    }

    #[test]
    fn learn_mode_empty_component_still_analyzes_code() {
        let state = test_state();
        let result = build_design_prompt(&state, "database", None, DesignMode::Learn).unwrap();
        let instructions = result["system_instructions"].as_str().unwrap();
        assert!(
            instructions.contains("READ THE SOURCE CODE"),
            "empty learn mode must still read source code"
        );
        assert!(
            instructions.contains("ANALYZE THE CODE"),
            "empty learn mode should drive code analysis to find implicit decisions"
        );
    }

    #[test]
    fn learn_mode_has_completeness_check() {
        let state = test_state();
        let result = build_design_prompt(&state, "auth", None, DesignMode::Learn).unwrap();
        let instructions = result["system_instructions"].as_str().unwrap();
        assert!(
            instructions.contains("COMPLETENESS CHECK"),
            "learn mode should verify all decisions are captured before ending"
        );
    }

    #[test]
    fn learn_mode_explains_when_user_doesnt_know() {
        let state = test_state();
        let result = build_design_prompt(&state, "auth", None, DesignMode::Learn).unwrap();
        let instructions = result["system_instructions"].as_str().unwrap();
        assert!(
            instructions.contains("senior engineer"),
            "learn mode should instruct agent to teach like a senior engineer"
        );
    }
}
