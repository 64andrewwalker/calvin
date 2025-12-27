---
description: Initialize Project
---

# [1-PLAN] Project Initialization

> **Stage**: Planning | **Category**: Scaffolding | **Frequency**: New projects

Design and scaffold a new project based on user requirements. This is PLANNING ONLY - no implementation code.

## Process

1. **Requirements Clarification**: Summarize user's goals, constraints, and preferences
2. **Architecture Design**: Select appropriate patterns (monolith/microservices/serverless, etc.)
3. **Tech Stack Selection**: Justify each technology choice with trade-offs
4. **Directory Structure**: Create logical folder hierarchy with placeholder READMEs
5. **Scaffold Setup**:
   - Initialize Git with `.gitignore`
   - Create `README.md` with project overview
   - Generate `docs/architecture.md` with design decisions
   - Create `docs/implementation-plan.md` with phased roadmap

## Output Files

```text
├── README.md
├── .gitignore
├── docs/
│   ├── architecture.md
│   ├── implementation-plan.md
│   └── tech-decisions.md
└── TODO.md (implementation checklist)
```

## Constraints

- Do NOT write implementation code
- Ensure plan is detailed enough for a different model to execute
- Include estimated complexity per component
