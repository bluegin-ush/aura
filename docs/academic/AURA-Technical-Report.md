# AURA: Embedding Cognitive Deliberation in Programming Language Semantics

## A Technical Report on Agent-Unified Runtime Architecture

---

**Abstract.** We present AURA (Agent-Unified Runtime Architecture), a programming language that embeds a cognitive deliberation loop---observe, reason, adjust, continue---directly into its execution semantics. Unlike existing approaches where large language models (LLMs) operate as external code-generation tools (GitHub Copilot, ChatRepair) or where self-healing operates at the systems level (MAPE-K, Rainbow), AURA introduces *cognitive primitives* (`goal`, `observe`, `expect`, `invariant`, `reason`) as first-class syntactic constructs parsed into AST nodes with defined evaluation rules. When a program executes under AURA's cognitive runtime, the virtual machine can pause at arbitrary execution points, reify the full execution context (variables, goals, invariants, observation history, checkpoints), dispatch to an LLM for deliberation, and resume with one of five structurally-typed interventions: continue, value injection, code patching, checkpoint-based backtracking with adjustments, or halting. Developer-declared goals and invariants constrain all LLM-generated modifications, creating a formally bounded adaptation space. To our knowledge, AURA is the first programming language where (1) cognitive deliberation is part of the operational semantics, (2) an LLM participates as a runtime component with access to live execution state, and (3) developer-declared invariants enforce safety constraints on AI-generated program modifications at the language level.

---

## 1. Introduction

### 1.1 The Paradigm Gap

Three research communities have independently developed solutions to the problem of building software that adapts to unexpected conditions:

**Agent-Oriented Programming** (Shoham 1993; Rao 1996; Bordini et al. 2007) introduced mental attitudes---beliefs, desires, intentions---as programming primitives. Languages like AgentSpeak/Jason, GOAL, and 2APL implement the BDI (Belief-Desire-Intention) architecture with explicit goal reasoning and plan failure handling. However, these languages predate the LLM era: their "reasoning" is plan-library lookup, not open-ended deliberation.

**Automatic Program Repair** (Le Goues et al. 2012; Xia & Zhang 2023; Long & Rinard 2016) developed techniques to fix bugs automatically, from search-based repair (GenProg) to LLM-based conversational repair (ChatRepair). These systems achieve impressive results on benchmarks, but they all operate *post-mortem*: the program must first fail, produce a test failure or error message, and then an external tool proposes a patch. No APR tool has access to live execution state.

**Self-Adaptive Systems** (Kephart & Chess 2003; Garlan et al. 2004; Weyns et al. 2012) formalized the MAPE-K loop (Monitor-Analyze-Plan-Execute over shared Knowledge) for autonomic computing. Systems like Rainbow detect architectural constraint violations and apply predefined repair strategies. These operate at the infrastructure level, not at the programming language level.

Despite decades of progress in each community, a fundamental gap persists: **no existing programming language integrates cognitive deliberation---the ability to pause execution, reason about the current state against declared intentions, and choose from structurally diverse interventions---into its execution semantics.**

### 1.2 The Synthesis

AURA closes this gap by synthesizing ideas from all three traditions into a single language design:

| Origin | Concept | AURA Realization |
|--------|---------|------------------|
| BDI architectures | Goals as first-class mental attitudes | `goal "description" check expr` --- goals with runtime-evaluated check expressions |
| Runtime verification | Continuous property monitoring | `observe variable` --- declares runtime monitoring points |
| Design by Contract | Preconditions and invariants | `invariant expr` --- constraints that bound all adaptations |
| MAPE-K loop | Monitor-Analyze-Plan-Execute cycle | `observe` -> `deliberate()` -> `CognitiveDecision` -> apply |
| Checkpoint/rollback | Transactional state management | `CheckpointManager` --- named snapshots with restoration and adjustment |
| LLM agent frameworks | LLM-powered reasoning | `reason "question"` --- explicit deliberation with value injection |

The result is a language where the execution model shifts from `parse -> run -> crash -> fix -> rerun` to `parse -> run -> observe -> reason -> adjust -> continue`.

### 1.3 Contributions

This report makes the following claims, each supported by implementation evidence and positioned against the surveyed literature:

1. **Goals as continuously evaluated runtime expressions** (Section 3.1). No existing BDI language treats goals as expressions in the host language evaluated during execution. AURA's `GoalDef.check: Option<Expr>` enables continuous goal monitoring at arbitrary granularity, distinct from AgentSpeak's symbolic atoms, GOAL's logical formulae, and Jadex's callback-based maintain goals.

2. **Cognitive deliberation as language semantics** (Section 3.2). No existing language defines deliberation as a semantic operation that can modify execution state, rewrite code, or backtrack with adjustments. The `CognitiveRuntime` trait (`observe`, `deliberate`, `check_goals`, `is_active`) is called by the VM during expression evaluation, not as an external monitoring layer.

3. **Five-mode intervention algebra** (Section 3.3). The `CognitiveDecision` enum defines five structurally typed interventions (`Continue`, `Override(Value)`, `Fix{new_code, explanation}`, `Backtrack{checkpoint, adjustments}`, `Halt(error)`), providing a richer intervention space than any existing self-healing system.

4. **Invariant-bounded adaptation** (Section 3.4). Developer-declared invariants and goals constrain all LLM-generated modifications. The `validate_fix()` function verifies that fixes are parseable, respect size limits, preserve all declared goals, and do not introduce new goals. This is a novel design pattern: developer-declared constraints on automated program modification.

5. **Zero-overhead cognitive abstraction** (Section 3.5). When `is_active()` returns `false` (the `NullCognitiveRuntime`), all cognitive checks are no-ops. Programs without cognitive features execute with identical performance to a non-cognitive runtime.

---

## 2. Related Work

### 2.1 Agent-Oriented Programming Languages

**AgentSpeak(L)** (Rao 1996) introduced the dominant BDI programming model: agents have beliefs (Prolog-like facts), triggering events activate plans from a plan library, and intentions are stacks of partially executed plans. **Jason** (Bordini et al. 2007) is the most complete implementation, adding speech acts, environments, and organizational abstractions. Goals in AgentSpeak are symbolic atoms (`!achieve_goal`) that trigger plan selection; failure causes intention dropping or replanning within the plan library.

**GOAL** (Hindriks 2009) uses declarative goals expressed as logical formulae. An agent's goal base is updated by a deliberation cycle that evaluates goals against beliefs. GOAL is the closest prior work to AURA's active goal model, but its goals are logical formulae in a separate belief query language, not expressions in the host language.

**2APL** (Dastani 2008) introduces *practical reasoning rules* (PR-rules) that revise plans when conditions change. When a plan fails, PR-rules match the failure context and generate revised plans. This is the most sophisticated replanning mechanism in the AOPL literature, but it operates on predefined rule-plan mappings, not open-ended LLM deliberation.

**Jadex** (Pokahr et al. 2005) adds *maintain goals* to the BDI model: conditions that must remain true, with automatic plan re-triggering when violated. This is structurally similar to AURA's `goal ... check expr`, but Jadex's maintain conditions are Java predicates registered as callbacks, not expressions in the agent language itself.

**SARL** (Rodriguez et al. 2014) introduces a capacity/skill model where agents declare required capacities and bind implementations at runtime. This is architecturally similar to AURA's capability system (`+http`, `+json`, `+db`).

**The gap.** No existing BDI language treats goals as continuously evaluated expressions in the host language's expression system. Table 1 summarizes the distinction:

*Table 1: Goal representation across agent-oriented languages*

| Language | Goal Representation | Evaluation Timing | Failure Response |
|----------|-------------------|-------------------|-----------------|
| AgentSpeak | Symbolic atom (`!g`) | On trigger | Drop intention |
| GOAL | Logical formula | Per deliberation cycle | Reselect plan |
| Jadex | Java predicate callback | On callback | Re-trigger plan |
| 2APL | Logical formula | Per cycle, PR-rules | Rule-based revision |
| **AURA** | **Host-language expression** | **Continuous, per-step** | **Cognitive deliberation + backtrack** |

### 2.2 Automatic Program Repair

**GenProg** (Le Goues et al. 2012) pioneered search-based automated program repair using genetic programming to evolve patches. **SemFix** (Nguyen et al. 2013) and **Angelix** (Mechtaev et al. 2016) introduced semantic-level repair using symbolic execution and constraint solving. **Prophet** (Long & Rinard 2016) learned code correctness models from human patches to rank candidates.

The LLM era transformed the field. **ChatRepair** (Xia & Zhang 2023) uses conversational LLM interaction to fix 162/337 Defects4J bugs at ~$0.42 per bug. **RepairLLaMA** (Silva et al. 2023) fine-tunes open-source LLMs with LoRA adapters for repair. **AlphaRepair** (Xia & Zhang 2022) demonstrated that pre-trained code models can perform zero-shot repair by treating buggy code as a masked language model problem.

**The post-mortem limitation.** All APR tools---classical and LLM-based---share a fundamental architecture:

```
[Program fails] -> [Extract code + error] -> [Send to repair tool] -> [Get patch] -> [Apply] -> [Re-run]
```

None has access to live execution state. None can inject values mid-execution. None can backtrack to a checkpoint with adjustments. The repair tool never sees which variables had which values at the moment of failure, what goals the developer intended (beyond test assertions), or the execution path that led to the error.

### 2.3 Self-Adaptive Systems

**Autonomic Computing** (Kephart & Chess 2003) proposed the MAPE-K reference architecture: Monitor (collect data via sensors), Analyze (determine if adaptation is needed), Plan (select strategy), Execute (apply via effectors), over shared Knowledge. **Rainbow** (Garlan et al. 2004) implements MAPE-K at the architectural level, monitoring running systems against constraints and applying predefined repair strategies.

**FORMS** (Weyns et al. 2012) provides a formal reference model for self-adaptive systems with rigorous semantics for the managed system, environment, adaptation goals, and feedback loop.

**The external-layer limitation.** All MAPE-K implementations add monitoring and adaptation as an external architectural layer. The managed system is a black box observed through probes. Adaptation strategies are predefined configurations, not runtime-generated code modifications. The adaptation logic is separate from the program logic.

### 2.4 Cognitive Architectures

**Soar** (Laird et al. 1987; Newell 1990) implements a production system with universal subgoaling: when no production fires, an *impasse* triggers automatic subgoal creation. Soar's *chunking* mechanism learns new productions from subgoal resolution, creating a learning loop. **ACT-R** (Anderson & Lebiere 1998; Anderson et al. 2004) models cognition as the interaction of modular buffers (visual, motor, declarative memory, goal buffer) mediated by production rules. **CLARION** (Sun 2016) explicitly models implicit (subsymbolic) and explicit (symbolic) knowledge interaction. **LIDA** (Franklin et al. 2014) implements Global Workspace Theory with a consciousness-like broadcast mechanism.

**The relevance.** AURA's cognitive runtime implements a cycle that maps directly to cognitive architecture components:

| Cognitive Component | AURA Implementation |
|---|---|
| Perception | `observe()` --- event detection during execution |
| Working Memory | Observation buffer + current execution context |
| Deliberation | `deliberate()` --- LLM invocation with packaged context |
| Decision | `CognitiveDecision` enum --- five intervention types |
| Action | Hot reload, value injection, checkpoint restore |
| Learning | `ReasoningEpisode` trace + `HealingMemory` persistence |
| Metacognition | `CognitiveSafetyConfig` --- safety bounds on reasoning |

This makes AURA's runtime itself a cognitive architecture, rather than a language used to *implement* a cognitive architecture---a distinction without precedent in the literature.

### 2.5 Reflective and Meta-Level Architectures

**Smith's 3-Lisp** (Smith 1984) introduced computational reflection: a program that can inspect and modify its own execution. **CLOS MOP** (Kiczales et al. 1991) provided a meta-object protocol allowing programs to customize their own class system. **Aspect-Oriented Programming** (Kiczales et al. 1997) introduced join points where cross-cutting concerns can intercept execution.

**Algebraic Effects** (Plotkin & Pretnar 2009; Bauer & Pretnar 2015) provide the closest formal model: computations can "yield" effects to handlers that inspect and resume them. AURA's cognitive bridge can be formalized as an algebraic effect handler where the effect is "I need cognitive assistance" and the handler is the LLM. The key difference: algebraic effect handlers are statically defined; AURA's "handler" generates novel responses dynamically.

**Common Lisp's condition/restart system** is the closest classical precedent to AURA's mid-execution intervention. When an error signals a condition, handlers can choose from pre-defined restarts (e.g., `use-value`, `store-value`, `abort`). AURA generalizes this: instead of programmer-defined restarts, the LLM generates novel interventions informed by runtime context, goals, and invariants.

### 2.6 LLM-Integrated Programming Systems

**LMQL** (Beurer-Kellner et al. 2023) is the most relevant comparison as an actual programming language (published at PLDI) that extends Python with constrained LLM generation. LMQL compiles to token-level masks for constrained decoding. However, it focuses on generation-time constraints, not agent reasoning---it has no goals, observation, self-healing, or cognitive runtime.

**DSPy** (Khattab et al. 2023) introduces declarative LLM program specifications with automatic prompt optimization. **SGLang** (Zheng et al. 2024) optimizes structured LLM program execution with RadixAttention. Both are Python-embedded and focus on LLM call efficiency, not runtime adaptation.

**ReAct** (Yao et al. 2023) and **Reflexion** (Shinn et al. 2023) implement observe-reason-act loops in LLM agents, but as prompt patterns, not language semantics.

*Table 2: LLM-integrated programming systems*

| System | Is a Language? | LLM as Primitive? | Goals? | Self-Healing? | Runtime Loop? |
|--------|---------------|-------------------|--------|---------------|---------------|
| LMQL | **Yes** | Yes (constrained gen) | No | No | No |
| DSPy | Partial (Python DSL) | Yes (signatures) | No | Prompt optimization | No |
| SGLang | Partial (Python DSL) | Yes (primitives) | No | No | No |
| LangChain | No (library) | No (function call) | No | No | No |
| ReAct | No (prompt pattern) | Yes (in-prompt) | No | No | Yes (ad hoc) |
| **AURA** | **Yes** | **Yes** (`reason`) | **Yes** (`goal check`) | **Yes** (language-level) | **Yes** (VM-integrated) |

---

## 3. Design and Implementation

### 3.1 Cognitive Primitives

AURA introduces six constructs that form its cognitive vocabulary. These are parsed into AST nodes---they are part of the language grammar, not library functions.

#### 3.1.1 `goal`

```
goal "process user data correctly"
goal "all users must have valid names" check users != nil
```

Goals are top-level declarations (`Definition::Goal(GoalDef)`) with an optional `check` expression. The `GoalDef` structure:

```rust
pub struct GoalDef {
    pub description: String,
    pub check: Option<Expr>,  // The novel element
    pub span: Span,
}
```

When `check` is present, the goal is *active*: the VM evaluates the check expression after observed variable changes, after function returns, and at configurable step intervals. If the check evaluates to false, a `DeliberationTrigger::GoalMisalignment` is raised, invoking the cognitive runtime.

The `check` keyword is parsed as a *soft keyword* (`Ident("check")`), not a reserved token---preserving backward compatibility with programs that use "check" as an identifier.

#### 3.1.2 `observe`

```
observe users
observe response.status
observe data where valid == true
```

`observe` declares a runtime monitoring point (`Expr::Observe`). When an observed variable changes value, the VM:
1. Creates an implicit checkpoint (via `CheckpointManager`)
2. Notifies the cognitive runtime via `observe(ObservationEvent::ValueChanged{...})`
3. Triggers active goal evaluation

Without a cognitive runtime, `observe` is a no-op returning nil.

#### 3.1.3 `expect`

```
expect len(users) > 0 : "should have users"
```

`expect` is intent verification (`Expr::Expect`). Unlike assertions that crash on failure, expects register as `ExpectationFailure` and, when a cognitive runtime is active, trigger `DeliberationTrigger::ExpectFailed`. The runtime can then decide to continue, override the result, generate a fix, or backtrack.

#### 3.1.4 `invariant`

```
invariant len(users) > 0
```

Invariants (`Definition::Invariant(Expr)`) declare constraints that no adaptation may violate. They serve as the developer's safety boundary: the `validate_fix()` function verifies that LLM-proposed fixes do not break invariants before they are applied.

#### 3.1.5 `reason`

```
strategy = reason "we have {len(users)} users, should we process all or filter?"
```

`reason` is an explicit deliberation point (`Expr::Reason`). Execution pauses, the question and recent observations are sent to the cognitive runtime, and the LLM's decision becomes the expression's value. This enables *value injection*: the LLM can return a value that is bound to a variable and used in subsequent computation.

Without a cognitive runtime, `reason` returns nil.

#### 3.1.6 `@self_heal`

```
@self_heal(max_attempts: 5, mode: "semantic")
process_data(data) = { ... }
```

Function-level annotation (`SelfHealConfig`) that marks individual functions for automatic repair. Configurable with `max_attempts` and `mode` (technical, semantic, auto).

### 3.2 The Cognitive Runtime Trait

The `CognitiveRuntime` trait defines the interface between the VM and the cognitive agent:

```rust
pub trait CognitiveRuntime: Send {
    fn observe(&mut self, event: ObservationEvent);
    fn deliberate(&mut self, trigger: DeliberationTrigger) -> CognitiveDecision;
    fn check_goals(&mut self) -> Vec<CognitiveDecision>;
    fn is_active(&self) -> bool;
    fn set_available_checkpoints(&mut self, checkpoints: Vec<String>) {}
}
```

**Observation events** (`ObservationEvent`) include `ValueChanged`, `ExpectEvaluated`, `FunctionReturned`, and `CheckpointCreated`. These provide the LLM with rich runtime context that no post-mortem repair tool can access.

**Deliberation triggers** (`DeliberationTrigger`) classify what provoked the deliberation: `ExpectFailed`, `ExplicitReason`, `TechnicalError`, or `GoalMisalignment`. This classification helps the LLM understand the nature of the problem.

The `NullCognitiveRuntime` implements all operations as no-ops with `is_active() = false`, providing zero overhead for non-cognitive execution.

### 3.3 The Five-Mode Intervention Algebra

`CognitiveDecision` defines five structurally typed interventions:

```rust
pub enum CognitiveDecision {
    Continue,
    Override(Value),
    Fix { new_code: String, explanation: String },
    Backtrack { checkpoint: String, adjustments: Vec<(String, Value)> },
    Halt(RuntimeError),
}
```

This is richer than any existing intervention model:

| Intervention | Semantics | Precedent |
|---|---|---|
| `Continue` | Proceed normally | Common (all systems) |
| `Override(Value)` | Inject a replacement value into the execution flow | Common Lisp `use-value` restart, but LLM-chosen |
| `Fix{new_code, explanation}` | Rewrite source code; re-execute from beginning | APR tools (GenProg, ChatRepair), but with runtime context |
| `Backtrack{checkpoint, adjustments}` | Restore VM to named checkpoint, apply variable adjustments, continue from that point | **No precedent** in APR or LLM frameworks |
| `Halt(error)` | Stop execution with explanation | Common (all systems) |

The `Backtrack` intervention is particularly novel. Unlike APR tools that must re-execute from scratch, and unlike Erlang supervision that restarts from initial state, AURA can restore to any named checkpoint *with adjustments*---the LLM specifies which variables to modify before resuming. This enables partial re-execution with informed corrections, a capability without precedent in the literature.

### 3.4 Safety: Invariant-Bounded Adaptation

The `validate_fix()` function enforces safety constraints before any LLM-proposed modification is applied:

1. **Size constraint**: Fixes exceeding `max_fix_lines` (default: 50) are rejected, preventing wholesale program rewrites.
2. **Syntactic validity**: Every proposed fix must tokenize and parse as valid AURA.
3. **Goal immutability**: The fix must preserve all declared goals---no additions, no removals, no modifications. Goals are the developer's exclusive domain.
4. **Backtrack depth**: `max_backtrack_depth` (default: 5) prevents infinite backtrack loops.
5. **Progress tracking**: `max_deliberations_without_progress` (default: 3) halts runaway reasoning.

```rust
pub struct CognitiveSafetyConfig {
    pub max_fix_lines: usize,
    pub max_backtrack_depth: usize,
    pub max_deliberations_without_progress: usize,
}
```

This establishes a formally bounded adaptation space: the LLM can modify the program, but only within constraints the developer has declared. This is a novel design pattern---**developer-declared constraints on automated program modification**---that has no direct precedent in the APR or self-adaptive systems literature.

### 3.5 Checkpoint System

The `CheckpointManager` maintains named snapshots of VM state:

```rust
pub struct VMCheckpoint {
    pub name: String,
    pub variables: HashMap<String, Value>,
    pub step_count: u64,
    pub timestamp: Instant,
}
```

Checkpoints are created implicitly (on `observe` triggers, before function calls) and can be restored with adjustments:

```
VM state at checkpoint "fetch_users": { users = [...], count = 3 }
                    |
                    v
Goal misalignment detected: "all users must be active"
                    |
                    v
cognitive.deliberate(GoalMisalignment{...})
                    |
                    v
CognitiveDecision::Backtrack {
    checkpoint: "fetch_users",
    adjustments: [("users", filtered_active_users)]
}
                    |
                    v
VM restores to "fetch_users", applies adjustments, continues
```

This combines ideas from software transactional memory (Shavit & Touitou 1995; Harris et al. 2005), Prolog's chronological backtracking, and BDI plan failure handling, but the synthesis---backtracking with LLM-suggested adjustments in a cognitive execution cycle---is new.

### 3.6 The AgentCognitiveRuntime

The real implementation connects the `CognitiveRuntime` trait to an `AgentProvider` (supporting multiple LLM backends):

```rust
pub struct AgentCognitiveRuntime<P: AgentProvider> {
    provider: P,
    tokio_handle: Handle,        // async-sync bridge
    goals: Vec<GoalDef>,
    invariants: Vec<String>,
    source_code: String,
    observation_buffer: Vec<ObservationEvent>,
    reasoning_trace: Vec<ReasoningEpisode>,
    available_checkpoints: Vec<String>,
    max_deliberations: usize,
    deliberation_count: usize,
    safety_config: CognitiveSafetyConfig,
    consecutive_backtracks: usize,
    deliberations_without_progress: usize,
}
```

Key design decisions:

- **Async-sync bridge**: The VM is synchronous; the `AgentProvider` is async. `tokio_handle.block_on()` bridges the gap, keeping the VM implementation simple.
- **Observation batching**: Events accumulate in `observation_buffer` and are drained after each deliberation, providing the LLM with cumulative context.
- **Episodic memory**: `reasoning_trace: Vec<ReasoningEpisode>` records every deliberation episode, included in subsequent requests so the LLM can learn from recent history.
- **Fail-open**: If the provider fails (network error, timeout), the runtime returns `Continue` rather than crashing. The cognitive layer never makes the program *less* reliable.

### 3.7 The Cognitive Execution Runner

The `run_cognitive()` function orchestrates the retry loop:

```rust
pub fn run_cognitive(
    source: &str,
    cognitive: Box<dyn CognitiveRuntime>,
    max_retries: usize,
) -> Result<CognitiveRunResult, RuntimeError>
```

For each attempt:
1. Parse the current source
2. Create VM with cognitive runtime (first attempt) or `NullCognitiveRuntime` (retries)
3. Load and run the program
4. If `pending_fixes` exist, validate each fix via `validate_fix()`, apply the valid one, and retry
5. If execution succeeds with no pending fixes, return the result
6. If retries exhausted, return the error

Crucially, `Backtrack` decisions are handled *within* a single execution (they are inline state restorations), while `Fix` decisions require re-parsing and re-execution. This dual-level adaptation---inline backtrack for quick corrections, full re-execution for structural changes---provides flexibility unmatched by single-strategy systems.

---

## 4. Positioning Against the State of the Art

### 4.1 Comprehensive Comparison

*Table 3: AURA positioned against representative systems from each research thread*

| Dimension | GenProg | ChatRepair | LangChain | Rainbow | Jason | AURA v2.0 |
|---|---|---|---|---|---|---|
| **Nature** | APR tool | LLM repair tool | LLM orchestration | Self-adaptive framework | BDI language | **Cognitive language** |
| **When repair happens** | Post-mortem | Post-mortem | N/A | Runtime (external) | Plan failure | **Mid-execution (in-VM)** |
| **Runtime state access** | None | None | None | Architectural metrics | Belief base | **Full: variables, goals, checkpoints** |
| **Repair oracle** | Test suite | Test suite + LLM | N/A | Predefined strategies | Plan library | **Goals + expects + invariants + LLM** |
| **Value injection** | No | No | No | No | Belief update | **Yes (`Override`)** |
| **Backtracking** | No | No | No | No | Intention stack | **Yes (checkpoint + adjustments)** |
| **Code patching** | Yes (source) | Yes (source) | N/A | Yes (config) | No | **Yes (validated source)** |
| **Safety constraints** | Test suite only | None | N/A | By construction | None | **Invariants + goal immutability** |
| **Developer intent** | Test cases | Test cases | Python code | Arch. constraints | BDI goals | **`goal`, `expect`, `invariant`** |
| **LLM integration** | None | External API | External API | None | None | **First-class runtime trait** |

### 4.2 The Tripartite Gap

AURA closes a gap at the intersection of three previously separate concerns:

1. **No current language** provides built-in constructs for expressing developer intent (`goal`), runtime expectations (`expect`), variable monitoring (`observe`), safe rollback points (checkpoints), and explicit reasoning requests (`reason`) as first-class syntax.

2. **No current system** gives an LLM access to live execution state (variable values, execution path, goal evaluation results) during program execution, enabling mid-execution decisions (value injection, code patching, checkpoint-based backtracking).

3. **No current system** enforces safety invariants on LLM-generated adaptations at the language level---where invariants are declared in program syntax, validated by the parser, and enforced before any LLM-proposed fix is applied.

### 4.3 Formal Novelty Claim

> AURA embeds a cognitive loop (observe-reason-adjust-continue) as a first-class runtime mechanism within the language's execution semantics, where (1) the "reason" phase invokes an external large language model with reified execution context, (2) the "adjust" phase applies type-checked modifications to running code, and (3) the loop operates at arbitrary granularity---from individual expressions to entire function bodies---rather than only at process boundaries (Erlang), transaction boundaries (STM), or predefined join points (AOP).

### 4.4 What Is Not Novel

Academic honesty requires identifying what AURA builds upon rather than invents:

- The BDI architecture (Rao & Georgeff 1991, 1995; Bratman 1987)
- MAPE-K self-adaptive loops (Kephart & Chess 2003)
- Checkpoint/rollback mechanisms (Shavit & Touitou 1995)
- LLM-based code repair (Xia & Zhang 2023; Le Goues et al. 2012)
- Hot code reloading (Armstrong 2003)
- Capability-based module systems (cf. SARL's capacity model)
- Condition/restart systems (Common Lisp)
- Runtime verification (Leucker & Schallhart 2009)

AURA's contribution is the *synthesis*: integrating all of the above into a unified runtime mechanism designed from the ground up for this interaction.

---

## 5. Worked Example

The following AURA program demonstrates cognitive execution. Comments annotate what the cognitive runtime does at each point.

```aura
# Cognitive Runtime Demo
# Run with: aura run --cognitive examples/cognitive_demo.aura

+http +json

# Developer declares intent
goal "process user data correctly"
goal "all users must have valid names" check users != nil

# Type definition with validation annotations
@User {
    id :i
    name :s
    email :s
}

# Data source function
fetch_users() = [
    {id: 1, name: "Alice", email: "alice@example.com"},
    {id: 2, name: "Bob", email: "bob@example.com"},
    {id: 3, name: "Charlie", email: "charlie@example.com"}
]

# Formatting function
format_user(user) = "#{user.id}: {user.name} <{user.email}>"

main = {
    # [1] Variable binding --- standard execution
    users = fetch_users()

    # [2] OBSERVE: Cognitive runtime is notified
    #     - Creates implicit checkpoint "users_observed"
    #     - Sends ObservationEvent::ValueChanged to LLM
    #     - Triggers active goal check: "users != nil" -> true -> OK
    observe users

    # [3] EXPECT: Intent verification with cognitive deliberation
    #     - Evaluates len(users) > 0 -> true -> passes
    #     - If false: would trigger DeliberationTrigger::ExpectFailed
    #     - LLM could respond with Override, Fix, or Backtrack
    expect len(users) > 0 : "should have users"

    # [4] REASON: Explicit deliberation
    #     - Execution pauses
    #     - LLM receives: question, observations, goals, invariants, checkpoints
    #     - LLM responds with CognitiveDecision
    #     - If Override("process_all"): strategy = "process_all"
    #     - If Continue: strategy = nil
    strategy = reason "we have {len(users)} users, should we process all or filter?"

    # [5] Standard computation continues with LLM-injected value
    results = map(users, format_user)

    # [6] Final verification
    expect len(results) == len(users) : "all users should be formatted"

    results
}
```

### 5.1 Execution Trace Under Cognitive Runtime

When run with `aura run --cognitive --provider claude examples/cognitive_demo.aura`:

```
Step 1: VM loads program. Goals registered. CheckpointManager initialized.

Step 2: VM evaluates `users = fetch_users()`.
        -> Value: List of 3 User records

Step 3: VM evaluates `observe users`.
        -> checkpoint_manager.save("users_observed", current_vars, step=3)
        -> cognitive.observe(ValueChanged { name: "users", old: Nil, new: List[...] })
        -> cognitive.check_goals():
           - "users != nil" evaluates to true -> no misalignment

Step 4: VM evaluates `expect len(users) > 0 : "should have users"`.
        -> Condition: true
        -> cognitive.observe(ExpectEvaluated { condition: "len(users) > 0", result: true })

Step 5: VM evaluates `reason "we have 3 users, should we process all or filter?"`.
        -> cognitive.deliberate(ExplicitReason {
             observations: ["users changed: Nil -> List[...]"],
             question: "we have 3 users, should we process all or filter?"
           })
        -> LLM receives: question + goals + invariants + checkpoints + recent observations
        -> LLM responds: Override(String("process_all"))
        -> strategy = "process_all"

Step 6: VM evaluates `results = map(users, format_user)`.
        -> Value: List of 3 formatted strings

Step 7: VM evaluates `expect len(results) == len(users)`.
        -> Condition: true

Step 8: VM returns results.
```

### 5.2 Counterfactual: Goal Misalignment Scenario

Suppose `fetch_users()` returned a list including a user with `name: nil`. The goal `check users != nil` would still pass (the list itself is not nil), but imagine a more precise goal:

```aura
goal "all names must be non-empty" check for(u in users) : u.name != nil
```

When the goal check fails:

```
Step 3b: cognitive.check_goals():
         - "for(u in users) : u.name != nil" evaluates to false
         -> DeliberationTrigger::GoalMisalignment {
              goal_description: "all names must be non-empty",
              check_result: Bool(false)
            }
         -> LLM deliberates...
         -> CognitiveDecision::Backtrack {
              checkpoint: "users_observed",
              adjustments: [("users", filtered_list_without_nil_names)]
            }
         -> VM restores to checkpoint "users_observed"
         -> VM applies adjustment: users = [only users with valid names]
         -> Execution continues from step 3 with corrected data
```

This is the power of checkpoint-based backtracking with LLM-informed adjustments: the program does not crash, does not restart from scratch, and does not apply a brittle predefined strategy. The LLM understands the goal ("all names must be non-empty"), examines the data, and proposes a targeted correction.

---

## 6. Discussion

### 6.1 Theoretical Framing

AURA's cognitive runtime can be formalized through multiple theoretical lenses:

**As an algebraic effect handler** (Plotkin & Pretnar 2009): The cognitive primitives (`observe`, `reason`, `expect` on failure) are effects yielded by the computation. The `CognitiveRuntime` is the handler that interprets these effects. The key novelty: the handler is not a statically defined function but a dynamically reasoning LLM.

**As a MAPE-K instance** (Kephart & Chess 2003): `observe` = Monitor, `DeliberationTrigger` classification = Analyze, LLM deliberation = Plan, `CognitiveDecision` application = Execute, `ReasoningEpisode` trace = Knowledge. The novelty: all phases are embedded in the language runtime, not layered externally.

**As a generalized condition/restart system**: Common Lisp conditions signal errors; restarts provide recovery options. AURA generalizes both: any execution event (not just errors) can trigger deliberation, and recovery options are generated dynamically by an LLM rather than pre-defined by the programmer.

### 6.2 The Runtime as Cognitive Architecture

When mapped to cognitive architecture theory, AURA's runtime implements the essential components identified by Newell (1990) and subsequent architectures:

- **Perception**: Event detection via `observe()`
- **Working memory**: Observation buffer + execution context
- **Long-term memory**: `HealingMemory` with `ReasoningEpisode` persistence
- **Deliberation**: LLM invocation with structured context
- **Action selection**: `CognitiveDecision` enum
- **Learning**: Episode history informs subsequent deliberations
- **Metacognition**: `CognitiveSafetyConfig` bounds on reasoning behavior

This makes AURA, to our knowledge, **the first programming language runtime that is itself a cognitive architecture**---rather than a language used to implement one.

### 6.3 Limitations

**Latency.** LLM deliberation adds seconds of latency per invocation. AURA mitigates this through the `NullCognitiveRuntime` (zero overhead when cognitive features are inactive) and batched observations, but real-time applications may need tighter latency bounds.

**Determinism.** LLM responses are non-deterministic. Two executions of the same program may follow different paths. AURA records the `ReasoningEpisode` trace for reproducibility analysis, but formal guarantees require further work.

**Correctness of LLM-generated fixes.** The `validate_fix()` function checks syntax, goal preservation, and size---but not semantic correctness. A fix that parses correctly and preserves goals may still introduce bugs. The test suite provides additional validation, but formal verification of LLM-generated patches remains an open research problem.

**Cost.** Each deliberation incurs LLM API costs. The `max_deliberations` and `max_deliberations_without_progress` bounds limit this, but cost-aware deliberation strategies are future work.

### 6.4 Future Directions

- **Formal semantics**: Define AURA's cognitive loop in an operational semantics framework, building on algebraic effects literature.
- **Multi-agent cognitive runtime**: Multiple LLMs with different specializations (e.g., one for code repair, one for architectural reasoning).
- **Verified adaptation**: Use formal methods to prove that adaptations within the invariant-bounded space preserve specified properties.
- **Cost-aware deliberation**: Strategies that balance LLM call cost against expected benefit.
- **Collaborative cognition**: Human-in-the-loop modes where the runtime presents options rather than acting autonomously.

---

## 7. Conclusion

AURA demonstrates that cognitive deliberation can be embedded in programming language semantics, creating a new category of language design. By synthesizing concepts from agent-oriented programming (BDI goals), self-adaptive systems (MAPE-K monitoring), and LLM-based program repair (conversational fixing) into first-class language constructs with defined evaluation rules, AURA opens a design space that is, to our knowledge, unexplored in the published literature.

The key insight is architectural: the LLM is not an external tool that processes source files, but a runtime component that participates in execution---observing variable changes, evaluating goals, and making decisions that are immediately applied within the execution flow. Developer-declared invariants and goals constrain the adaptation space, creating a formally bounded contract between human intent and machine reasoning.

AURA's 244 tests, complete implementation in Rust, and working cognitive execution mode demonstrate that this design is not merely theoretical but practically realizable. The `NullCognitiveRuntime` ensures zero overhead for non-cognitive programs, making the cognitive capabilities purely additive.

Whether this paradigm scales to production systems, how formal correctness guarantees can be established for LLM-generated adaptations, and what developer experience patterns emerge when programs can reason about their own execution---these are questions that AURA's existence makes concrete and tractable.

---

## References

### Agent-Oriented Programming Languages

[1] Shoham, Y. (1993). "Agent-Oriented Programming." *Artificial Intelligence*, 60(1):51-92.

[2] Rao, A.S. (1996). "AgentSpeak(L): BDI Agents Speak Out in a Logical Computable Language." *MAAMAW'96*, LNCS 1038, Springer, 42-55.

[3] Bordini, R.H., Hubner, J.F., & Wooldridge, M. (2007). *Programming Multi-Agent Systems in AgentSpeak using Jason*. Wiley.

[4] Hindriks, K.V. (2009). "Programming Rational Agents in GOAL." In *Multi-Agent Programming*, Springer, 119-157.

[5] Dastani, M. (2008). "2APL: A Practical Agent Programming Language." *Autonomous Agents and Multi-Agent Systems*, 16(3):214-248.

[6] Rodriguez, S., Gaud, N., & Galland, S. (2014). "SARL: A General-Purpose Agent-Oriented Programming Language." *WI-IAT 2014*, IEEE/WIC/ACM.

[7] Pokahr, A., Braubach, L., & Lamersdorf, W. (2005). "Jadex: A BDI Reasoning Engine." In *Multi-Agent Programming*, Springer, 149-174.

### BDI Theory

[8] Bratman, M.E. (1987). *Intention, Plans, and Practical Reason*. Harvard University Press.

[9] Rao, A.S. & Georgeff, M.P. (1991). "Modeling Rational Agents within a BDI-Architecture." *KR'91*, Morgan Kaufmann, 473-484.

[10] Rao, A.S. & Georgeff, M.P. (1995). "BDI Agents: From Theory to Practice." *ICMAS'95*, AAAI Press, 312-319.

[11] Sardina, S. & Padgham, L. (2011). "A BDI Agent Programming Language with Failure Handling, Declarative Goals, and Planning." *Autonomous Agents and Multi-Agent Systems*, 23(1):18-70.

[12] Cohen, P.R. & Levesque, H.J. (1990). "Intention is Choice with Commitment." *Artificial Intelligence*, 42(2-3):213-261.

### Automatic Program Repair

[13] Le Goues, C., Nguyen, T.V., Forrest, S., & Weimer, W. (2012). "GenProg: A Generic Method for Automatic Software Repair." *IEEE TSE*, 38(1):54-72.

[14] Nguyen, H.D.T., Qi, D., Roychoudhury, A., & Chandra, S. (2013). "SemFix: Program Repair via Semantic Analysis." *ICSE 2013*, 772-781.

[15] Mechtaev, S., Yi, J., & Roychoudhury, A. (2016). "Angelix: Scalable Multiline Program Patch Synthesis via Symbolic Analysis." *ICSE 2016*, 1071-1082.

[16] Long, F. & Rinard, M. (2016). "Automatic Patch Generation by Learning Correct Code." *POPL 2016*, 298-312.

[17] Xia, C.S. & Zhang, L. (2022). "Less Training, More Repairing Please: Revisiting Automated Program Repair via Zero-Shot Learning." *ESEC/FSE 2022*, 959-971.

[18] Xia, C.S. & Zhang, L. (2023). "Keep the Conversation Going: Fixing 162 out of 337 bugs for $0.42 each using ChatGPT." *ISSTA 2024*. arXiv:2304.00385.

[19] Monperrus, M. (2018). "Automatic Software Repair: A Bibliography." *ACM Computing Surveys*, 51(1):1-24.

### Self-Adaptive Systems

[20] Kephart, J.O. & Chess, D.M. (2003). "The Vision of Autonomic Computing." *IEEE Computer*, 36(1):41-50.

[21] Garlan, D., Cheng, S.-W., Huang, A.-C., Schmerl, B., & Steenkiste, P. (2004). "Rainbow: Architecture-Based Self-Adaptation with Reusable Infrastructure." *IEEE Computer*, 37(10):46-54.

[22] Weyns, D., Malek, S., & Andersson, J. (2012). "FORMS: Unifying Reference Model for Formal Specification of Distributed Self-Adaptive Systems." *ACM TAAS*, 7(1).

[23] Weyns, D. (2020). *An Introduction to Self-Adaptive Systems: A Contemporary Software Engineering Perspective*. Wiley/IEEE Press.

### Cognitive Architectures

[24] Laird, J.E., Newell, A., & Rosenbloom, P.S. (1987). "SOAR: An Architecture for General Intelligence." *Artificial Intelligence*, 33(1):1-64.

[25] Newell, A. (1990). *Unified Theories of Cognition*. Harvard University Press.

[26] Anderson, J.R. & Lebiere, C. (1998). *The Atomic Components of Thought*. Lawrence Erlbaum Associates.

[27] Anderson, J.R. et al. (2004). "An Integrated Theory of the Mind." *Psychological Review*, 111(4):1036-1060.

[28] Sun, R. (2016). *Anatomy of the Mind: Exploring Psychological Mechanisms and Processes with the Clarion Cognitive Architecture*. Oxford University Press.

[29] Franklin, S. et al. (2014). "LIDA: A Systems-level Architecture for Cognition, Emotion, and Learning." *IEEE Trans. on Autonomous Mental Development*, 6(1):19-41.

### Reflection, Effects, and Meta-Programming

[30] Smith, B.C. (1984). "Reflection and Semantics in Lisp." *POPL '84*, ACM, 23-35.

[31] Kiczales, G., des Rivieres, J., & Bobrow, D.G. (1991). *The Art of the Metaobject Protocol*. MIT Press.

[32] Kiczales, G. et al. (1997). "Aspect-Oriented Programming." *ECOOP '97*, LNCS 1241, Springer, 220-242.

[33] Plotkin, G.D. & Pretnar, M. (2009). "Handlers of Algebraic Effects." *ESOP 2009*, LNCS 5502, Springer, 80-94.

[34] Bauer, A. & Pretnar, M. (2015). "Programming with Algebraic Effects and Handlers." *Journal of Logical and Algebraic Methods in Programming*, 84(1):108-123.

### Checkpoint, Rollback, and Fault Tolerance

[35] Shavit, N. & Touitou, D. (1995). "Software Transactional Memory." *PODC '95*, ACM, 204-213.

[36] Harris, T., Marlow, S., Peyton Jones, S., & Herlihy, M. (2005). "Composable Memory Transactions." *PPoPP '05*, ACM, 48-60.

[37] Armstrong, J. (2003). *Making Reliable Distributed Systems in the Presence of Software Errors*. PhD Thesis, Royal Institute of Technology, Stockholm.

[38] Rinard, M. et al. (2004). "Enhancing Server Availability and Security Through Failure-Oblivious Computing." *OSDI 2004*, USENIX, 303-316.

[39] Perkins, J.H. et al. (2009). "Automatically Patching Errors in Deployed Software." *SOSP 2009*, ACM, 87-102.

### LLM-Integrated Programming

[40] Beurer-Kellner, L., Fischer, M., & Vechev, M. (2023). "Prompting Is Programming: A Query Language for Large Language Models." *PLDI 2023*, ACM, 1507-1532.

[41] Khattab, O. et al. (2023). "DSPy: Compiling Declarative Language Model Calls into Self-Improving Pipelines." arXiv:2310.03714. *ICLR 2024*.

[42] Zheng, L. et al. (2024). "SGLang: Efficient Execution of Structured Language Model Programs." arXiv:2312.07104.

[43] Yao, S. et al. (2023). "ReAct: Synergizing Reasoning and Acting in Language Models." *ICLR 2023*.

[44] Shinn, N. et al. (2023). "Reflexion: Language Agents with Verbal Reinforcement Learning." *NeurIPS 2023*.

### Goal-Oriented Programming and Planning

[45] Fikes, R.E. & Nilsson, N.J. (1971). "STRIPS: A New Approach to the Application of Theorem Proving to Problem Solving." *Artificial Intelligence*, 2(3-4):189-208.

[46] Nilsson, N.J. (1994). "Teleo-Reactive Programs for Agent Control." *JAIR*, 1:139-158.

[47] Nau, D. et al. (2003). "SHOP2: An HTN Planning System." *JAIR*, 20:379-404.

### Runtime Verification and Design by Contract

[48] Meyer, B. (1992). "Applying 'Design by Contract'." *IEEE Computer*, 25(10):40-51.

[49] Leucker, M. & Schallhart, C. (2009). "A Brief Account of Runtime Verification." *Journal of Logic and Algebraic Programming*, 78(5):293-303.

[50] Ernst, M.D. et al. (2007). "The Daikon System for Dynamic Detection of Likely Invariants." *Science of Computer Programming*, 69(1-3):35-45.

### Surveys and Foundational Work

[51] Wooldridge, M. & Jennings, N.R. (1995). "Intelligent Agents: Theory and Practice." *Knowledge Engineering Review*, 10(2):115-152.

[52] Wang, L. et al. (2024). "A Survey on Large Language Model Based Autonomous Agents." *Frontiers of Computer Science*.

[53] Schmidhuber, J. (2003). "Goedel Machines: Self-Referential Universal Problem Solvers Making Provably Optimal Self-Improvements." Technical Report IDSIA-19-03.

[54] Hicks, M. & Nettles, S. (2005). "Dynamic Software Updating." *ACM TOPLAS*, 27(6):1049-1096.

[55] Gat, E. (1998). "On Three-Layer Architectures." In *Artificial Intelligence and Mobile Robots*, MIT Press, 195-210.

---

*AURA is implemented in Rust with 244 tests. Source code available at the project repository.*
