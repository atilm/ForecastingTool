# Project YAML Format

General structure expectations:

- Put done tasks first.
- The first done task must have `dependencies: null`.
- Each task implicitly depends on the previous task if dependencies are not specified.
- If you need a special start date for the first TODO task, set it explicitly.

Date values should use `YYYY-MM-DD`.

Example skeleton:

```yaml
issues:
  - id: ISSUE-1
    summary: Setup
    status: Done
    done_date: 2026-01-10
    dependencies: null
    estimate:
      story_points: 3
  - id: ISSUE-2
    summary: Implementation
    status: Todo
    estimate:
      story_points: 5
```
