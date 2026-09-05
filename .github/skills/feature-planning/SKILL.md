---
name: feature-planning
description: "Create a detailed, implementation-ready feature plan from a Markdown feature description. Use when requirements need clarification, existing code analysis, and documentation in the source file before implementation begins."
argument-hint: "Provide the attached feature description to analyse"
user-invocable: true
disable-model-invocation: false
---

# Feature Planning From Attachment

## Purpose

Turn a Markdown feature description into a clarified requirements document and an implementation plan without changing production code, tests, configuration, or other implementation files.

## When to Use

Use this skill when the user asks to:

- read a Markdown feature description;
- ask clarification questions before implementation;
- inspect the existing code to make the plan concrete; and
- record the detailed requirements and implementation plan in the same attachment.

## Procedure

1. Identify the Markdown feature description and read it completely. Treat its existing content as the source document that must be updated.
2. Inspect the repository locally and read only the code, tests, documentation, and configuration needed to understand the feature's owning abstractions, data flow, affected APIs, and existing conventions. Prefer targeted searches and nearby implementations over broad repository mapping.
3. Before asking questions, separate what is explicit from what is ambiguous. Record a short working summary of the current behavior and the likely implementation surface in your reasoning.
4. Ask focused clarification questions about behavior, edge cases, compatibility, data formats, error handling, API changes, testing expectations, and acceptance criteria. Ask only questions whose answers could change the requirements or implementation plan. Group questions so the user can answer efficiently.
5. Wait for the user's answers before finalizing the documented requirements and plan. If the user cannot answer a question, document the assumption and its impact instead of silently deciding.
6. Update the same Markdown feature description with:
   - a `## Detailed Requirements` section;
   - explicit functional and non-functional requirements;
   - clarified behavior and acceptance criteria;
   - edge cases, constraints, assumptions, and unresolved decisions;
   - a `## Implementation Plan` section;
   - the relevant files, modules, symbols, data flow, and API changes identified from code analysis;
   - ordered implementation steps with dependencies;
   - a focused test strategy and validation commands; and
   - risks or migration considerations.
7. Preserve the original feature description and its intent. Improve existing planning sections in place where appropriate, but do not erase useful context. Keep the document readable and avoid duplicating unchanged prose.
8. Validate that the edited Markdown is coherent and that every planned change is grounded in something observed in the repository or explicitly marked as an assumption.

## Strict Boundaries

- Do not modify Rust source code, tests, build files, configuration, generated files, or documentation other than the Markdown feature description.
- Do not implement, refactor, install dependencies, run formatters that rewrite files, or make commits.
- Repository inspection must be read-only. Running focused read-only checks or existing test/build commands is allowed only when useful for understanding the current behavior; do not alter their outputs or generated artifacts.
- Do not claim that a requirement is clarified unless the user answered it or the document labels it as an assumption.
- Do not invent file names, symbols, APIs, or test results. Reference concrete repository evidence in the plan.

## Expected Document Shape

Keep the source document's existing sections, then add or revise content along these lines:

```markdown
## Detailed Requirements

### Functional Requirements
- ...

### Acceptance Criteria
- ...

### Edge Cases and Constraints
- ...

### Assumptions and Open Decisions
- ...

## Implementation Plan

### Affected Code
- `path/to/file`: `symbol` and why it is involved

### Steps
1. ...

### Tests and Validation
- ...

### Risks and Migration
- ...
```

The exact headings may follow the document's existing style, but both detailed requirements and the implementation plan must be clearly identifiable.
