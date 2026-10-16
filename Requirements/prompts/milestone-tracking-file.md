Add automatic tracking of milestones in a yaml file to the command `forecasts simulate project`.

- Add a parameter to the command to specify the tracking file path
  - Use `--milestones-file` for the optional path. If omitted, use `milestones.yaml` next to the `project.yaml` input file.
- Use the file format defined by the yaml code example below
- When a simulation has been run, for each work item of type `milestone` in the simulated data, a `forecast` entry should be added to the data set for the milestone identified by its id
- When the milestones file already exists, it should be parsed, the parsed data should be appended with the data of the current simulation with the current date. Then the extended data set should be written back to the same file.
- If the milestones file does not exist already, it should be created.
- The boolean entry `plot` specifies if the milestone should be included in a plot. It is intended to be edited manually only. The default should be true.
- There is an existing feature which already plots milestones directly into `report.yaml.milestones.png`. Please remove this feature.

```yaml
start_date: 2026-01-01
end_date: 2028-09-15
milestones:
  - id: MS_1
    name: First milestone title
    plot: true
    forecasts:
      - date: 2026-02-25
        p0: 2026-03-24
        p15: 2026-08-10
        p50: 2026-07-09
        p85: 2026-09-14
        p100: 2027-02-05
      - date: 2026-03-12
        p0: 2026-06-23
        p15: 2026-07-14
        p50: 2026-08-03
        p85: 2026-08-26
        p100: 2026-10-05
  - id: MS_2
    name: Second milestone title
    plot: false
    forecasts:
      - date: 2026-07-09
        p0: 2026-11-24
        p15: 2026-12-25
        p50: 2027-01-04
        p85: 2027-01-15
        p100: 2027-02-15
      - date: 2026-08-10
        p0: 2026-11-26
        p15: 2026-12-18
        p50: 2026-12-27
        p85: 2027-01-06
        p100: 2027-02-02
```

## Detailed Requirements

### Functional Requirements

- Add the optional `--milestones-file <PATH>` argument to `forecasts simulate project`.
- Resolve the default tracking path to `milestones.yaml` in the directory containing the input project YAML. Resolve an explicitly supplied relative path from the process working directory.
- Create the tracking file when it does not exist. For an existing file, parse its YAML, update its milestone records, and write the complete dataset back to that same path.
- For every simulated work package of type `Milestone`, identify the tracking record by ID and record one forecast containing the run date and that work package's p0, p15, p50, p85, and p100 end dates.
- Use the current local calendar date for the forecast's `date` field. If an entry for the same milestone and date already exists, replace it with the current run's forecast rather than adding a duplicate.
- Use the milestone summary from the project as its tracking `name`. Refresh the name on every run. A missing summary is an error and fails the tracking update.
- Initialize new milestones with `plot: true`. Preserve an existing milestone's `plot` value; the simulation command must not change this manually maintained value. When reading a record without `plot`, treat it as `true`.
- Retain existing milestone records that are not present in the current simulation. Add new records for milestones first encountered in the current simulation.
- Set the tracking file's `start_date` to the run's `--start-date` when creating the file; preserve it on subsequent updates. On runs with milestones, set `end_date` to the latest p100 end date among milestones in that run. If a run has no milestones, use the overall simulation p100 to initialize `end_date` for a new file; otherwise preserve the existing `end_date`.
- A tracking-file parse, read, or write failure fails the command with an error; it must not be downgraded to a warning.
- Remove automatic generation of `<report-output>.milestones.png`. Do not add a replacement plot in this feature. The `plot` field remains manually editable tracking metadata.

### Acceptance Criteria

- Running the command without `--milestones-file` creates or updates `milestones.yaml` beside the project input, independent of the process working directory.
- Running with `--milestones-file relative/path.yaml` resolves that path from the process working directory.
- A newly tracked milestone has its project summary as `name`, `plot: true`, and one forecast with the current date and the five simulated percentile end dates.
- A later run appends a forecast for a new date; a run on the same date replaces the prior forecast for that milestone and date.
- Existing `plot` values and records for milestones absent from a run are preserved; names for currently simulated milestones are refreshed from the project.
- The file's initial `start_date` remains unchanged on later runs. Its `end_date` follows the latest p100 among milestones in the latest run; on a first run without milestones it uses the overall simulation p100, and on later runs without milestones it remains unchanged.
- Missing milestone summaries and invalid/unreadable/unwritable tracking files cause a command error.
- The command no longer creates `<report-output>.milestones.png`.

### Edge Cases, Constraints, and Assumptions

- The supplied YAML example defines the tracking schema. Dates are YAML dates serialized as `YYYY-MM-DD`; forecasts contain `date`, `p0`, `p15`, `p50`, `p85`, and `p100`.
- The forecast `date` is interpreted as the machine-local calendar date when the command runs. This follows the repository's existing use of `chrono::Local` for default dates.
- Existing tracking files are expected to follow the schema. Invalid YAML or missing required top-level data is a tracking error; only `plot` has an explicit default.
- The simulation report currently contains milestone IDs and percentile end dates but not project summaries. The implementation must obtain summaries from the parsed project data without changing the serialized report format unless necessary.
- `start_date` and `end_date` are tracking metadata with the lifecycle specified above; they are not the forecast entry date.

## Implementation Plan

### Affected Code

- `src/commands/base_commands.rs`: `SimulateProjectArgs` owns CLI arguments and already has tests for command defaults and argument parsing.
- `src/commands/simulate_cmd.rs`: `simulate_command` runs project simulation, writes auxiliary outputs, and currently creates the automatic milestone PNG.
- `src/commands/mod.rs`: `CommandError` defines user-facing command errors and should represent milestone tracking failures.
- `src/services/project_simulation/project_simulation.rs` and `src/services/project_simulation/simulation_types.rs`: simulation output is built from parsed project work packages; report milestone records currently contain IDs and percentile dates, while source summaries are available on `Issue`.
- `src/services/parsing/project_yaml.rs`: `load_project_from_yaml_file` parses project records and maps summaries onto domain issues.
- `src/services/plotting/milestone_plot.rs` and `src/services/plotting/mod.rs`: implement and register the automatic milestone plot that this feature removes.
- `tests/simulate_project_tests.rs` and unit tests in command/service modules: existing project simulation and CLI coverage can host focused regression tests.
- `docs/src/cli-reference.md`: document the new option and its path semantics.

### Steps

1. Add `--milestones-file` as an optional project simulation argument and test its parsing and omitted-value behavior.
2. Define typed serializable/deserializable tracking structures and a focused service for loading, updating, and writing the YAML dataset. Default omitted `plot` values to `true`.
3. Provide the updater with current simulated milestone percentile dates and source project summaries. Keep summary metadata separate from serialized simulation-report fields unless the existing data flow requires otherwise.
4. Resolve default and explicit paths according to the requirements, update records by ID, replace same-date forecasts, preserve manually maintained plot values and absent milestone records, and apply the date-bound rules.
5. Integrate the tracking update into `simulate_command`; propagate tracking errors as command failures.
6. Remove the automatic milestone PNG call, its plotting module registration and implementation/tests, and update CLI documentation.
7. Run focused CLI, tracking-service, and project simulation tests, then the full Rust test suite.

### Tests and Validation

- Unit-test CLI parsing with and without `--milestones-file`.
- Test tracking-file creation, existing-file updates, ID matching, new and absent milestones, name refresh, `plot` preservation/defaulting, new-date append, same-date replacement, date-bound behavior, and zero-milestone runs both with and without an existing file.
- Test missing summaries and malformed, unreadable, and unwritable files produce errors.
- Add or update command-level coverage verifying tracking output and confirming no milestone PNG is produced.
- Run `cargo test` after focused tests pass.

### Risks and Migration

- Tracking-file updates add a new required output step: file or schema errors will fail a command that otherwise completed simulation, as specified.
- Users relying on the automatic `<report-output>.milestones.png` artifact will no longer receive it. No replacement plot is included.
- Keeping `start_date` from the first run while refreshing `end_date` means the two bounds intentionally describe different update points; consumers must rely on this documented lifecycle.