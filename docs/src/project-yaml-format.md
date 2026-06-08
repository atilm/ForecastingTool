# Project YAML Format

General structure expectations:

- Put done tasks first.
- The first done task must have `dependencies: null`.
- Each task implicitly depends on the previous task if dependencies are not specified.
- If you need a special start date for the first TODO task, set it explicitly.

Date values should use `YYYY-MM-DD`.

Projects are described with a top-level `name` and a `work_packages` list.

Each work package supports these common fields:

- `id`: required unique identifier.
- `summary`: optional short label.
- `description`: optional free text.
- `status`: optional, one of `ToDo`, `InProgress`, or `Done`.
- `created_date`, `start_date`, `done_date`: optional dates in `YYYY-MM-DD` format.
- `dependencies`: optional list of work package ids. Use `null` for the first done item. Use `[]` to make an item depend on the previous one implicitly.
- `estimate`: optional estimate definition.

## Estimate Types

### Story points

```yaml
estimate:
  type: story_points
  value: 5
```

### Three-point estimate

```yaml
estimate:
  type: three_point
  optimistic: 2
  most_likely: 4
  pessimistic: 7
```

### Reference to a simulation report

This reuses the `p0`, `p50`, and `p100` durations from an existing project simulation report.

```yaml
estimate:
  type: reference
  report_file_path: reports/subproject.yaml
```

For in-progress items, elapsed days since `start_date` are added to the referenced remaining duration.

### Reference to a histogram distribution

This samples directly from a histogram YAML file. The file format is the same one produced by throughput simulation histogram export.

```yaml
estimate:
  type: histogram_reference
  histogram_file_path: reports/throughput_histogram.yaml
```

For in-progress items, elapsed days since `start_date` are added to the sampled histogram duration.

### Milestone

```yaml
estimate:
  type: milestone
```

## Example

```yaml
name: Demo Project
work_packages:
  - id: DONE-1
    summary: Setup
    status: Done
    start_date: 2026-01-06
    done_date: 2026-01-10
    dependencies: null
    estimate:
      type: story_points
      value: 3
  - id: ISSUE-2
    summary: Implementation
    status: ToDo
    estimate:
      type: three_point
      optimistic: 3
      most_likely: 5
      pessimistic: 8
  - id: ISSUE-3
    summary: Reuse a previous simulation
    estimate:
      type: reference
      report_file_path: reports/backend-project.yaml
  - id: ISSUE-4
    summary: Sample from throughput histogram
    estimate:
      type: histogram_reference
      histogram_file_path: reports/histogram.yaml
  - id: RELEASE
    estimate:
      type: milestone
    dependencies: [ISSUE-4]
```
