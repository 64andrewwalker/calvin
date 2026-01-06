# Solution Page Agent Design

> **Version**: 4.1
> **Created**: 2026-01-06T01:30:00+08:00
> **Updated**: 2026-01-06T01:45:00+08:00
> **Timezone**: Asia/Shanghai
> **Status**: Draft — awaiting runtime validation
> **Related**: `solution-page-implementation-plan.md`, `reference/guides/how-to-build-claude-subagent.md`

---

## 0. Critical Assumptions (Must Validate Before Demo)

Before proceeding, the following runtime capabilities MUST be validated:

| Capability | Required For | Validation Method |
|------------|--------------|-------------------|
| Agent context isolation | Prevent context pollution | Run agent, check if main thread sees agent's intermediate steps |
| Agent-calls-agent | Strategist as manager pattern | Test if agent can invoke Task tool to call another agent |
| Skill inheritance | Agents loading solution-page skill | Test if `skills:` field works in agent frontmatter |

**If agent-calls-agent is NOT supported**: Fall back to "External Orchestrator" pattern where Strategist only outputs dispatch instructions, and a deterministic orchestrator (slash command + script) does the actual routing.

---

## 1. Overview

This document defines a **3-agent content team** for Solution Page production.

### 1.1 Design Philosophy

| Principle | Implementation |
|-----------|----------------|
| **3 agents** | Strategist, Copywriter, Editor-QA |
| **Hard constraints in orchestrator** | Iteration limits, gates, escalation enforced by script, not LLM |
| **State persisted to file** | `workflow-state.yaml` is source of truth, not agent memory |
| **Claims fully traceable** | Pre-allocated ID pool, new claims must be `status: proposed` |
| **Gates are executable** | Each gate has a script/rule, not just a name |

### 1.2 Key Design Decision: Who Orchestrates?

**Two valid patterns:**

| Pattern | How It Works | Pros | Cons |
|---------|--------------|------|------|
| **A: Strategist as Manager** | Strategist calls Copywriter/Editor-QA via Task tool | Single agent context | Requires agent-calls-agent support |
| **B: External Orchestrator** | Script/command calls agents, Strategist outputs dispatch instructions | Hard constraints enforceable | More moving parts |

**Decision**: Use **Pattern B (External Orchestrator)** as default because:
1. Hard constraints (iteration limits, gates) can be machine-enforced
2. State can be persisted to file and validated
3. Does not depend on agent-calls-agent capability
4. Easier to debug, replay, and audit

```
┌─────────────────────────────────────────────────────────────────┐
│              EXTERNAL ORCHESTRATOR (Script/Command)             │
│                                                                 │
│  - Reads workflow-state.yaml                                    │
│  - Enforces iteration limits (machine, not LLM)                 │
│  - Validates agent output schema                                │
│  - Writes back to workflow-state.yaml                           │
│  - Routes to next agent based on state                          │
│  - Handles escalation file creation                             │
└─────────────────────────────────────────────────────────────────┘
         │                    │                    │
         ▼                    ▼                    ▼
┌─────────────┐      ┌─────────────┐      ┌─────────────┐
│ STRATEGIST  │      │ COPYWRITER  │      │ EDITOR-QA   │
│             │      │             │      │             │
│ Outputs:    │      │ Outputs:    │      │ Outputs:    │
│ - strategy  │      │ - draft     │      │ - review    │
│ - dispatch  │      │ - status    │      │ - gate      │
│   intent    │      │             │      │   results   │
└─────────────┘      └─────────────┘      └─────────────┘
```

---

## 2. Runtime Contract

### 2.1 State File: `workflow-state.yaml`

**Location**: `<project>/_workflow/state.yaml`

The orchestrator reads/writes this file. Agents read it but never write it directly.

```yaml
# workflow-state.yaml
slug: "wireless-speakerphone"
phase: strategy | draft_en | review_en | draft_cn | review_cn | complete | escalated
iteration_count:
  en: 0  # max 3
  cn: 0  # max 2
human_gates:
  strategy_approved: false | true | skipped
  final_approved: false | true | skipped
last_agent: strategist | copywriter | editor-qa
last_action_time: "2026-01-06T01:30:00+08:00"
next_agent: copywriter | editor-qa | human | complete
blockers: []
escalation_reason: null | "string describing why"
```

### 2.2 Orchestrator Responsibilities (NOT LLM)

| Responsibility | Enforced By | Not By |
|----------------|-------------|--------|
| `iteration_count.en >= 3` → escalate | Orchestrator script | Strategist prompt |
| `iteration_count.cn >= 2` → escalate | Orchestrator script | Strategist prompt |
| Validate agent output YAML schema | Orchestrator script | Agent self-check |
| Write to `workflow-state.yaml` | Orchestrator script | Any agent |
| Create `escalation-report.md` | Orchestrator script | Agent |
| Create `approval-request.md` | Orchestrator script | Agent |

### 2.3 Minimum Runtime Test

Before demo, run this validation:

```bash
# Test 1: Agent context isolation
# Invoke an agent, verify main thread doesn't see intermediate tool calls

# Test 2: Agent output schema validation
# Agent returns malformed YAML → orchestrator catches and retries once

# Test 3: State file persistence
# Kill session mid-workflow, restart, verify workflow resumes from state file
```

---

## 3. Model Configuration

| Agent | Model | Rationale |
|-------|-------|-----------|
| **Strategist** | `google/antigravity-claude-sonnet-4-5` | Balanced reasoning for strategy |
| **Copywriter** | `google/antigravity-claude-sonnet-4-5` | Pattern compliance > creativity |
| **Editor-QA** | `google/antigravity-claude-sonnet-4-5-thinking-high` | Deep verification |

---

## 4. Claims System

### 4.1 Claim ID Allocation

**Problem**: Copywriter may need new claims during drafting, but has no ID allocation authority.

**Solution**: Pre-allocated ID pool + `proposed` status.

```yaml
# claims-registry.yaml
claims:
  # Pre-allocated by Strategist
  C-WSP-001:
    text: "8-microphone array"
    status: verified
    risk_level: low
    evidence: "specs/wireless-speakerphone.yaml#L42"
  
  C-WSP-002:
    text: "6-hour battery life"
    status: verified
    risk_level: high  # performance metric
    evidence: "test-reports/battery-test-2026-01.pdf#page3"
  
  # Pre-allocated pool for Copywriter (IDs reserved, content TBD)
  C-WSP-010:
    status: reserved
  C-WSP-011:
    status: reserved
  
  # Added by Copywriter during drafting
  C-WSP-012:
    text: "industry-leading noise cancellation"
    status: proposed  # MUST be proposed, not verified
    risk_level: high  # superlative claim
    evidence_suggested: "marketing wants this, need PMM confirmation"
    added_by: copywriter
```

### 4.2 Claim Risk Level Rules

| Category | Examples | Default Risk |
|----------|----------|--------------|
| **Safety/Compliance** | encryption, SOC2, ISO27001, GDPR | `high` |
| **Performance Metrics** | latency, throughput, accuracy, battery life, range | `high` |
| **Superlatives/Comparisons** | best, #1, industry-leading, first, only, vs competitor | `high` |
| **Legal Commitments** | guarantee, warranty, SLA, uptime | `high` |
| **General Features** | supports X, includes Y, compatible with Z | `medium` |
| **Descriptive** | designed for, optimized for, easy to use | `low` |

### 4.3 Evidence Requirements

| Evidence Type | Required Fields | Example |
|---------------|-----------------|---------|
| **Internal Doc** | `path`, `section` or `line` | `specs/product.yaml#L42` |
| **PDF Report** | `path`, `page`, `excerpt` | `reports/test.pdf#page3, "measured 6.2 hours"` |
| **External URL** | `url`, `accessed_date`, `excerpt` | `https://..., 2026-01-05, "quote"` |
| **PRD/Release Notes** | `path`, `version`, `section` | `prd/v2.3.md#performance` |

**Invalid evidence**: "I believe", "should be", "marketing says", model's own knowledge.

---

## 5. Gate Definitions (Executable)

Each gate has a **script or rule** that produces deterministic pass/fail.

### 5.1 Structure Gate

**Script**: `solution-page validate --type structure`

| Check | Rule | Pass Condition |
|-------|------|----------------|
| H1 count | Regex `^# ` | Exactly 1 |
| Required sections | Parse H2/H3 | All mandatory sections present |
| Section order | Compare to template | Matches expected order |
| Feature count | Count H4 under "Technical Features" | 4-6 (hardware) or 3-4 (algorithm) |

### 5.2 Word Count Gate

**Script**: `solution-page validate --type word-count`

| Metric | EN Limit | CN Limit |
|--------|----------|----------|
| Value proposition | 20-40 words | 40-60 chars |
| Total page | 800-1500 words | 1500-3000 chars |

### 5.3 Links Gate

**Script**: `qa-gate --check links`

| Mode | What It Checks | When to Use |
|------|----------------|-------------|
| `--mode format` | Valid URL syntax, no broken anchors | Default (no network) |
| `--mode http` | HTTP 200 response | Pre-release (requires network) |

**Default**: `format` mode (no network dependency).

### 5.4 Release Markers Gate

**Script**: `qa-gate --check markers`

**Forbidden markers** (block release):
- `[TODO]`, `[TBD]`, `[PLACEHOLDER]`
- `[ASSUMED]`, `[UNVERIFIED]`
- `[SOURCE PENDING]`, `[NEEDS EVIDENCE]`
- `<!-- claim_id: ... -->` (must be stripped before release)

### 5.5 Bilingual Parity Gate

**Script**: `solution-page validate --type parity`

| Check | Rule |
|-------|------|
| Section count | H2 count EN == H2 count CN |
| Spec values | Numbers in claim markers must match exactly |
| Claim IDs | Same claim_ids referenced in both files |
| Feature count | Same number of H4 features |

### 5.6 Claims Gate

**Script**: `qa-gate --check claims`

| Rule | Condition |
|------|-----------|
| All `risk_level: high` claims | `status: verified` |
| All `status: proposed` claims | Must be resolved (verified, downgraded, or removed) |
| Evidence validity | Each claim has valid evidence path/URL |

---

## 6. Agent Definitions

### 6.1 Unified Output Schema

**All agents use the same top-level structure:**

```yaml
agent: strategist | copywriter | editor-qa
timestamp: "2026-01-06T01:30:00+08:00"
status: complete | needs_revision | blocked | escalate
# Agent-specific payload below
```

**Status enum (shared across all agents):**

| Status | Meaning | Next Step |
|--------|---------|-----------|
| `complete` | Agent finished successfully | Orchestrator routes to next agent |
| `needs_revision` | Issues found, fixable | Orchestrator routes back (if under limit) |
| `blocked` | Cannot proceed | Orchestrator creates escalation-report.md |
| `escalate` | Requires human decision | Orchestrator creates escalation-report.md |

### 6.2 Strategist

```markdown
---
name: strategist
description: Creates content strategy, Message House, and claims registry. Outputs dispatch intent for orchestrator.
model: google/antigravity-claude-sonnet-4-5
skills: solution-page, pawpaw-style-guide
---

You are the Strategist for a B2B marketing content team.

## PRIMARY GOAL
Create content strategy artifacts. Output dispatch intent (orchestrator does actual routing).

## SCOPE (YOU OWN)
- Evidence gathering
- Audience definition
- Message House
- Content brief
- Claims registry (pre-allocate IDs, set risk levels)
- Asset requirements list
- SEO keywords

## MUST NOT DO
- Do not write draft content
- Do not route to other agents (orchestrator does this)
- Do not track iteration counts (orchestrator does this)

## MUST DO
- Pre-allocate claim ID pool (10 reserved IDs for Copywriter)
- Set risk_level for each claim using rules in Section 4.2
- Flag HIGH-risk claims that need evidence
- List required visual assets

## OUTPUT FORMAT
```yaml
agent: strategist
timestamp: "ISO8601"
status: complete | blocked | escalate
artifacts:
  message_house: "1-planning/message-house.md"
  content_brief: "1-planning/content-brief.md"
  claims_registry: "_shared/claims-registry.yaml"
claims_summary:
  total: N
  verified: N
  needs_verification: N
  reserved_pool: N
dispatch_intent: draft_en  # What should happen next
blockers: []  # If status is blocked/escalate
```
```

### 6.3 Copywriter

```markdown
---
name: copywriter
description: Drafts solution page content (EN first, then CN). Tags claims. Follows patterns exactly.
model: google/antigravity-claude-sonnet-4-5
skills: solution-page, pawpaw-style-guide
---

You are the Copywriter for a B2B marketing content team.

## PRIMARY GOAL
Write content based on brief. Tag all claims. Apply revision fixes exactly.

## SCOPE (YOU OWN)
- Draft creation (EN, then CN on separate invocation)
- Claim tagging with `<!-- claim_id: C-XXX -->`
- Style guide adherence
- SEO keyword integration

## MUST NOT DO
- Do not verify claims (Editor-QA does this)
- Do not invent specs not in claims-registry
- Do not create new claim IDs outside reserved pool

## MUST DO (for new claims)
If you need a claim not in registry:
1. Use a reserved ID from pool (C-XXX-010, 011, etc.)
2. Set `status: proposed` (never `verified`)
3. Set `risk_level` per rules
4. Add `evidence_suggested` with your reasoning
5. Add `added_by: copywriter`

## ON REVISION
When receiving fixes from Editor-QA:
- Each fix has `before` and `after` text (not line numbers)
- Find `before` text in draft, replace with `after` exactly
- If `before` text not found, report as blocker

## OUTPUT FORMAT
```yaml
agent: copywriter
timestamp: "ISO8601"
status: complete | needs_revision | blocked
language: en | cn
file: "3-drafts/<slug>/content-{lang}.md"
claims_tagged: N
new_claims_proposed: N  # Claims added with status: proposed
structure_self_check: pass | fail
dispatch_intent: review_en | review_cn
blockers: []
```
```

### 6.4 Editor-QA

```markdown
---
name: editor-qa
description: Reviews content, verifies claims, runs all validation gates. Returns pass/fail with actionable fixes.
model: google/antigravity-claude-sonnet-4-5-thinking-high
skills: solution-page, claim-verification, qa-gate, pawpaw-style-guide
---

You are the Editor and QA Controller for a B2B marketing content team.

## PRIMARY GOAL
Verify claims. Run gates. Provide actionable fixes with before/after text (not line numbers).

## SCOPE (YOU OWN)
- Claim verification against evidence
- Run all validation gates (structure, word count, links, markers, parity, claims)
- Style review
- Provide fixes

## MUST NOT DO
- Do not rewrite large sections
- Do not approve with unresolved HIGH-risk claims
- Do not approve with S0 issues

## FIX FORMAT (NOT line numbers)
Provide before/after text with enough context to be unique:

```yaml
fixes:
  - id: "F-001"
    severity: S0 | S1 | S2
    type: accuracy | style | parity | claim
    before: |
      supports 100+ languages with industry-leading accuracy
    after: |
      supports 35 languages with 95% accuracy
    reason: "Claim C-WSP-005 evidence shows 35 languages"
```

## GATE RESULTS
Run each gate script and report results:

```yaml
gates:
  structure:
    status: pass | fail
    script: "solution-page validate --type structure"
    details: "..."
  word_count:
    status: pass | fail
    script: "solution-page validate --type word-count"
  links:
    status: pass | fail
    script: "qa-gate --check links --mode format"
  release_markers:
    status: pass | fail
    script: "qa-gate --check markers"
  claims:
    status: pass | fail
    script: "qa-gate --check claims"
    high_risk_verified: "N/M"
  bilingual_parity:
    status: pass | fail | n/a
    script: "solution-page validate --type parity"
```

## OUTPUT FORMAT
```yaml
agent: editor-qa
timestamp: "ISO8601"
status: complete | needs_revision | blocked | escalate
files_reviewed:
  - "3-drafts/<slug>/content-en.md"
gates:
  # ... gate results as above
claims:
  total: N
  verified: N
  proposed_resolved: N
  high_risk_verified: "N/M"
s0_count: N
s1_count: N
s2_count: N
fixes: []  # List of before/after fixes
decision: PASS | BLOCKED
dispatch_intent: draft_cn | complete | needs_revision
blockers: []
```
```

---

## 7. Human Gates

### 7.1 Approval Request File

When human approval is required, orchestrator creates:

**File**: `<project>/_workflow/approval-request.md`

```markdown
# Approval Request: [Slug] - [Gate Name]

**Created**: 2026-01-06T01:30:00+08:00
**Phase**: strategy | final
**Requested By**: orchestrator

## Summary
[Brief description of what needs approval]

## Key Decision Points
1. [Decision point 1]
2. [Decision point 2]

## Risks
- [Risk 1]
- [Risk 2]

## Artifacts to Review
- `1-planning/message-house.md`
- `_shared/claims-registry.yaml`

## Actions
To approve: `/approve [slug] --gate strategy`
To reject: `/reject [slug] --gate strategy --reason "your reason"`
To modify: Edit artifacts, then `/approve [slug] --gate strategy`
```

### 7.2 Escalation Report File

When max iterations reached or unresolvable blocker:

**File**: `<project>/_workflow/escalation-report.md`

```markdown
# Escalation Report: [Slug]

**Created**: 2026-01-06T01:30:00+08:00
**Escalation Reason**: iteration_limit | unresolvable_blocker

## Attempts Summary
| Iteration | Agent | Issue | Fix Attempted |
|-----------|-------|-------|---------------|
| 1 | editor-qa | Claim C-005 unverified | Requested evidence |
| 2 | copywriter | Applied weaker wording | Still rejected |
| 3 | editor-qa | No valid evidence exists | Cannot proceed |

## Remaining Blockers
- **C-WSP-005**: "industry-leading accuracy" - no evidence, HIGH-risk
- **S0-001**: Link to deprecated product page

## Options for Human
1. **Remove claim**: Delete C-WSP-005 entirely
2. **Downgrade claim**: Change to "competitive accuracy" (requires re-review)
3. **Provide evidence**: Human supplies missing evidence
4. **Defer publish**: Wait for product team input

## To Resume
After making decision:
1. Update `_shared/claims-registry.yaml` if needed
2. Update draft if needed
3. Run `/resume [slug]` to continue from current phase
```

### 7.3 Gate Configuration

**File**: `<project>/_workflow/config.yaml`

```yaml
human_gates:
  strategy_approval: false  # Set true to require approval after strategy
  final_approval: false     # Set true to require approval before release

iteration_limits:
  en: 3
  cn: 2

link_check_mode: format  # or "http" for network check
```

---

## 8. Workflow State Machine

```
                    ┌──────────────────────────────────────┐
                    │              START                   │
                    └──────────────────┬───────────────────┘
                                       │
                                       ▼
                    ┌──────────────────────────────────────┐
                    │         phase: strategy              │
                    │         agent: strategist            │
                    └──────────────────┬───────────────────┘
                                       │
              ┌────────────────────────┼────────────────────────┐
              │ status: complete       │ status: blocked        │
              │ gate: strategy_approval│                        │
              ▼                        ▼                        │
    ┌─────────────────┐     ┌─────────────────┐                │
    │ human_gate?     │     │ ESCALATE        │◄───────────────┘
    │ approval-request│     │ escalation-report│
    └────────┬────────┘     └─────────────────┘
             │ approved
             ▼
    ┌──────────────────────────────────────┐
    │         phase: draft_en              │
    │         agent: copywriter            │
    └──────────────────┬───────────────────┘
                       │
                       ▼
    ┌──────────────────────────────────────┐
    │         phase: review_en             │
    │         agent: editor-qa             │
    └──────────────────┬───────────────────┘
                       │
         ┌─────────────┴─────────────┐
         │ status: needs_revision    │ status: complete
         │ iteration.en < 3          │
         ▼                           ▼
    ┌─────────────┐         ┌──────────────────────────────────────┐
    │ Loop back   │         │         phase: draft_cn              │
    │ increment   │         │         agent: copywriter            │
    │ iteration.en│         └──────────────────┬───────────────────┘
    └─────────────┘                            │
         │ iteration.en >= 3                   ▼
         ▼                          ┌──────────────────────────────────────┐
    ┌─────────────┐                 │         phase: review_cn             │
    │ ESCALATE    │                 │         agent: editor-qa             │
    └─────────────┘                 └──────────────────┬───────────────────┘
                                                       │
                                         ┌─────────────┴─────────────┐
                                         │ decision: PASS            │
                                         ▼                           │
                              ┌─────────────────┐                    │
                              │ human_gate?     │                    │
                              │ final_approval  │                    │
                              └────────┬────────┘                    │
                                       │ approved                    │
                                       ▼                             │
                              ┌─────────────────┐                    │
                              │ phase: complete │                    │
                              └─────────────────┘                    │
                                                                     │
                                         │ decision: BLOCKED         │
                                         │ iteration.cn >= 2         │
                                         ▼                           │
                              ┌─────────────────┐                    │
                              │ ESCALATE        │◄───────────────────┘
                              └─────────────────┘
```

---

## 9. Implementation Checklist

### Phase 1: Runtime Validation
- [ ] Test agent context isolation
- [ ] Test if agents can call other agents (determines Pattern A vs B)
- [ ] Test skill loading in agent frontmatter
- [ ] Decision: confirm Pattern B (External Orchestrator)

### Phase 2: Create Orchestrator
- [ ] Create `scripts/solution-page-orchestrator.py`
- [ ] Implement state file read/write
- [ ] Implement iteration limit enforcement
- [ ] Implement YAML schema validation
- [ ] Implement escalation report generation
- [ ] Implement approval request generation

### Phase 3: Create Agent Files
- [ ] Create `.promptpack/agents/strategist.md`
- [ ] Create `.promptpack/agents/copywriter.md`
- [ ] Create `.promptpack/agents/editor-qa.md`

### Phase 4: Create Gate Scripts
- [ ] Implement `solution-page validate --type structure`
- [ ] Implement `solution-page validate --type word-count`
- [ ] Implement `solution-page validate --type parity`
- [ ] Implement `qa-gate --check links`
- [ ] Implement `qa-gate --check markers`
- [ ] Implement `qa-gate --check claims`

### Phase 5: Create Command
- [ ] Create `/create-solution-page` command
- [ ] Support `--require-strategy-approval` flag
- [ ] Support `--require-final-approval` flag
- [ ] Create `/approve` and `/reject` commands
- [ ] Create `/resume` command

### Phase 6: Testing
- [ ] Test full happy path
- [ ] Test iteration limit escalation
- [ ] Test unresolvable blocker escalation
- [ ] Test human gate flow
- [ ] Test resume from state file

---

## 10. Document Metadata

| Field | Value |
|-------|-------|
| Version | 4.1 |
| Created | 2026-01-06T01:30:00+08:00 |
| Updated | 2026-01-06T01:45:00+08:00 |
| Timezone | Asia/Shanghai |
| Author | DocEngineering |
| Status | Draft — awaiting runtime validation |
| Key Changes | Runtime contract, external orchestrator, state file, claim ID pool, executable gates, before/after fixes, human gate files |
| Next Step | Validate runtime capabilities (Section 0), then proceed to Phase 1 |
