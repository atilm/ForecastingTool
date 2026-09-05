# Burndown chart with dynamically created issues

## Goal

Modify the burndown chart creation such that that chart also contains work packages that have been created dynamically during the
simulation with the critical path method.

## Context

A recent addition to the simulation is, that work_packages can be create dynamically during the project simulation with a specified
rate, so that we can simulate the fact that additional requirements are discovered while the project is executed.

## Overall burndwon logic

* The burndown chart should be created as if all work had been known at the beginning of the project
  * That means, the burn down chart never rises
  * It starts at the summed-up number of all story points including dynamically created story points

## Initial Implementation Direction

* At the moment `burndown_plot.rs` gets the information about story points from the `Project` data structre,
  but this data structure does not contain dynamically created issues.
* First, extend the `SimulationReport` data structure such that it is alone sufficient to create a burndown
  diagram. It must additionally include:
    * information about wether a work_package is Done, Planned, In Progress or Dynamically created
      * I suggest replacing the field `is_milestone` with an enum-like field `type` which can be one of:
        Planned, DynamicallyPlanned, InProgress, Done, Milestone
    * The story points of the work_package. Here we can reuse the existing `Estimate` enum
* Then, update `plot_burndown_from_yaml_files` so that it does no longer take the `project_path` and plots
  the burndown chart only from the report file and the calendar files.

## Detailed Requirements

### Functional Requirements

- `SimulationReport` must contain all work-package information required to build a burndown chart without loading the original project YAML.
- Each reported work package must include its identifier, serialized estimate, lifecycle type, and simulation percentile dates. The lifecycle type must use the exact YAML strings `ToDo`, `DynamicToDo`, `InProgress`, `Done`, and `Milestone`.
- The existing `Estimate` enum must be reused for report estimates. Add serialization and deserialization support to `Estimate` and any nested estimate types required by the YAML representation.
- A `Done` work package must include its actual `done_date` in the report. The date must remain a `NaiveDate` and serialize as `YYYY-MM-DD`.
- Dynamically created work packages must be represented in the report with type `DynamicToDo`, their generated story-point estimate, and their simulated percentile dates.
- The report generation path must distinguish original planned, in-progress, done, and milestone items from dynamically generated items. Dynamic items are identified from the nodes created by the critical path simulation rather than from the original project YAML.
- The burndown total must include the estimates of `Done`, `ToDo`, `InProgress`, and `DynamicToDo` items from the report before any work is burned down. Milestones do not contribute story points.
- The burndown must be monotonic non-increasing: dynamically discovered work is treated as known at chart start and must not make the plotted remaining-work line rise.
- `Done`, `InProgress`, `ToDo` and `DynamicToDo` work must be plotted in distinct colors. 
- `plot_burndown_from_yaml_files` must accept the report path, output path, and optional calendar path, and must not load or receive a project path.
- The `plot burndown` CLI command must remove the `--input`/`-i` project argument and invoke the report-only plotting API.

### Non-Functional Requirements

- Preserve `NaiveDate` throughout report parsing and chart calculations; date strings use `YYYY-MM-DD`.
- Use `thiserror` for any new report or burndown validation errors.
- Keep the public API and report model changes explicit and serializable through the existing YAML parsing path.
- Keep the implementation consistent with the existing plotting and simulation modules and avoid modifying generated documentation or unrelated legacy code.

### Acceptance Criteria

- A simulation with dynamically created story-point work packages produces a report containing those packages as `DynamicToDo` entries with estimates and percentile dates.
- A burndown created from only the report YAML and optional calendar renders successfully and includes dynamic work in its initial total.
- The chart distinguishes `Done`, `ToDo`, and `DynamicToDo` visually.
- Done work uses the stored actual `done_date`; it does not require reconstructing that date from a project YAML file.
- The burndown CLI succeeds with `--report`, `--output`, and optional `--calendar-dir`, and rejects/removes the obsolete `--input` option according to the normal Clap interface.
- Unit tests cover report classification/serialization, estimate handling, done-date handling, dynamic-work accounting, and monotonic chart points.
- Integration tests invoke the CLI with a report containing at least one `DynamicToDo` item and verify that a PNG is produced without a project input.

### Edge Cases and Constraints

- A report with no work-package data must return the existing report-data error rather than attempting to load a project.
- Unsupported estimate variants must produce a focused burndown error; milestones must be handled without story-point conversion.
- Missing `done_date` on a `Done` report item must be rejected with an item-specific error.
- Reports containing only done work or only forecast work must preserve the existing meaningful validation errors where those concepts still apply.
- Dynamic items may be present even though they have no corresponding original project issue; they must not be matched against the project model.
- The report format may change. Backward compatibility and migration fixtures for reports using `is_milestone` are not required.

### Assumptions and Open Decisions

- `WorkPackageSimulation` remains the per-item report structure and gains the type, estimate, and optional done-date fields; the exact Rust enum name is an implementation detail, while its serialized values are fixed above.
- The existing percentile structure remains the source of forecast completion dates for `ToDo`, `InProgress`, and `DynamicToDo` items.
- `Done` items can retain percentile data for report-shape consistency, but the burndown uses `done_date` for their actual completion event.
- The renderer will implement distinct colors through separate classified point/event series and will add or update a legend if needed to make the distinction observable in the PNG.
- The current CLI convention remains `--report`/`-r` and `--output`/`-o`; only the project input argument is removed.

## Implementation Plan

### Affected Code

- `src/services/project_simulation/simulation_types.rs`: extend `WorkPackageSimulation` and define the serializable lifecycle type and report estimate/date fields.
- `src/domain/estimate.rs`: derive or implement YAML serialization/deserialization for `Estimate`, `StoryPointEstimate`, and the nested estimate records required by the report format.
- `src/services/project_simulation/project_simulation.rs`: retain source issue status, estimate, and done date while aggregating simulation samples; classify generated nodes as `DynamicToDo` and serialize their generated estimates.
- `src/services/project_simulation/critical_path_method.rs`: expose enough generated-node metadata, if required by the report aggregation, to distinguish dynamically created nodes from original nodes and preserve their estimates.
- `src/services/plotting/burndown_plot.rs`: remove project loading and project-based matching; build burndown data directly from report work packages, including dynamic items and actual done dates.
- `src/services/plotting/burndown_plot_rendering.rs`: render distinct colors for done, planned/in-progress, and dynamic-to-do work and update chart legend/series data as needed.
- `src/commands/base_commands.rs`: remove the project `input` field from `PlotBurndownArgs`.
- `src/commands/plot_burndown_cmd.rs`: pass only report, output, and calendar arguments to the plotting service.
- `src/services/plotting/burndown_plot_tests.rs`: update unit fixtures to the new report schema and add report-only burndown cases.
- `tests/plot_burndown_tests.rs`: update CLI fixtures and add a dynamic-work report integration case without `--input`.
- `src/services/plotting/milestone_plot.rs`, `src/services/plotting/simulation_gantt.rs`, and other consumers of `WorkPackageSimulation`: update field access from `is_milestone` to the new lifecycle type where compilation and behavior require it.

### Steps

1. Define the serializable lifecycle enum with the exact YAML values and extend estimate serialization while preserving `NaiveDate` formatting.
2. Extend the report aggregation data flow so each original issue carries its status/type, estimate, and done date into `WorkPackageSimulation`.
3. Carry generated story-point estimates and generated-node identity through the critical-path simulation so dynamically created items are emitted as `DynamicToDo` entries.
4. Update all report consumers that currently read `is_milestone` to use the lifecycle type, preserving milestone-specific behavior.
5. Refactor `build_burndown_data` to consume only report work packages and calendar data, calculate the initial total from all story-point-bearing work, and create classified completion events for the forecast curves.
6. Update the renderer to use distinct colors for `Done`, `ToDo`/`InProgress`, and `DynamicToDo` work and expose the distinction in the chart legend or equivalent labels.
7. Remove the burndown project argument from the command definition and command/service call chain.
8. Update focused unit and integration fixtures and add the dynamic-work and report-only acceptance cases.

### Tests and Validation

- Run focused Rust unit tests for the simulation report, estimate serialization, simulation aggregation, and burndown plotting modules.
- Add unit assertions for exact YAML lifecycle values, serialized estimates, actual done dates, dynamic-item inclusion, milestone exclusion, unsupported estimates, and non-increasing remaining points.
- Run `cargo test --test plot_burndown_tests` for CLI coverage, including the absence of `--input` and successful PNG creation from a report containing `DynamicToDo`.
- Run the complete `cargo test` suite after focused tests pass.
- Run `cargo fmt --check` and `cargo clippy --all-targets --all-features -- -D warnings` as final repository validation.

### Risks and Migration

- Changing `WorkPackageSimulation` and removing `is_milestone` affects simulation Gantt, milestone plotting, report fixtures, and any checked-in report examples; all source consumers must be updated together.
- Existing serialized reports using `is_milestone` are intentionally not supported after this format change, so checked-in fixtures and examples must be regenerated or rewritten.
- The estimate enum contains variants that are meaningful to project simulation but may not be valid burndown inputs; burndown validation must reject unsupported variants clearly.
- Rendering separate classifications may require restructuring the current aggregate forecast-point representation. The implementation should preserve the existing percentile band semantics while adding only the minimum series data needed for color distinction.