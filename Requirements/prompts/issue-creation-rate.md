# Story-point creation rate

Add a new option to the `forecasts simulate project` command:

`-r, --story-point-creation-rate <RATE>  Daily rate with which story points are created [default: 0]`

## Clarified requirements

1. The rate is a finite, non-negative decimal number of story points per calendar day. Negative values, `NaN`, and infinity must be rejected by CLI parsing.
2. A zero rate preserves the existing simulation behavior.
3. For a project without story-point estimates:
   - A positive rate has no effect on the simulation.
   - The command succeeds and prints a warning that the creation rate is being ignored.
   - An omitted or explicitly zero rate does not print this warning.
4. For a project with story-point estimates, the critical-path forward pass models newly created work:
   - Creation starts at the simulation start date and uses elapsed calendar days, including weekends, holidays, and zero-capacity dates.
   - The forward pass maintains a global high-water date so parallel nodes do not count the same elapsed period more than once.
   - Elapsed days add `elapsed_days * story_point_creation_rate` to an accumulator.
   - When the accumulator is at least one point, one generated issue is appended using the largest configured Fibonacci estimate that is less than or equal to the accumulated points. The issue's points are subtracted and any remainder is retained.
   - At most one generated issue is appended per processed-node event. If at least one point remains, it is considered again after the next queued node is processed.
   - Generated issue duration is its story-point estimate divided by the calculated project velocity. Calendar capacity continues to determine its scheduled finish date in the same way as existing story-point work.
5. Generated work forms a serial backlog extension:
   - The first generated issue depends on every terminal node in the original dependency graph.
   - Every later generated issue depends on the previously generated issue.
   - Time spent processing generated issues also creates story points and can append further issues.
6. The safety limit is 1000 processed nodes per Monte Carlo iteration, counting both original and generated nodes. The simulation aborts with a custom error before processing a 1001st node; it must not return a truncated forecast.
7. Generated issues affect project completion dates and appear in the report's `work_packages` details. Their internal IDs must be deterministic and repeatable across Monte Carlo iterations for the same generated-issue sequence, while remaining collision-safe with original project IDs. When a work package, including a generated one, occurs in only a subset of iterations, calculate its percentiles from all and only its occurrences rather than treating absent iterations as observations. `simulated_items` is the maximum number of processed nodes in any iteration, including generated nodes.

## Existing-code analysis

- `src/commands/base_commands.rs` owns `SimulateProjectArgs` and Clap defaults/validation.
- `src/commands/simulate_cmd.rs` destructures the CLI arguments and calls `simulate_project_from_yaml_file`.
- `src/services/project_simulation/project_simulation.rs` detects story-point projects, calculates velocity, builds and sorts sampled network nodes for each iteration, calls the critical-path method, and builds the report.
- `src/services/project_simulation/network_nodes.rs` converts issues to already-sampled `NetworkNode` values. A story-point node's duration has already been divided by velocity before it reaches the critical-path method.
- `src/services/project_simulation/critical_path_method.rs` currently consumes a fixed topologically sorted vector. Its forward pass must become a dynamically extendable queue before the successor map and backward pass are built.
- `src/services/project_simulation/fibonacci.rs` contains the repository's estimate scale. It currently has no helper that returns the largest configured estimate less than or equal to an arbitrary positive value.
- `src/services/plotting/estimate_gantt.rs` also calls `critical_path_method`; that call must explicitly disable issue creation so plotting behavior remains unchanged.
- `tests/simulate_project_tests.rs` is the integration-test surface for command output, validation, warnings, generated-work effects, and limit failures.

## Implementation plan

1. Extend `SimulateProjectArgs` with `story_point_creation_rate: f32`, short option `-r`, long option `--story-point-creation-rate`, and default `0`. Add a focused Clap value parser that accepts only finite non-negative values. Add CLI unit tests for the default, a fractional value, and invalid negative/non-finite values.
2. Thread the rate through `simulate_command`, `simulate_project_from_yaml_file`, `simulate_project`, and `run_simulation`. Preserve source compatibility at unrelated callers where practical by introducing a small critical-path creation configuration rather than loose parameters.
3. In project simulation, enable creation only when the rate is positive and the project has story points. For a positive rate on a non-story-point project, emit one warning per command invocation and pass a disabled creation configuration. Pass calculated velocity into the enabled configuration.
4. Add a Fibonacci helper that returns the largest configured estimate less than or equal to the available points, with unit tests at exact values, between values, below one, and above the largest configured estimate.
5. Refactor the critical-path forward pass to process a mutable queue/vector by index:
   - Determine the original graph's terminal node IDs before appending work.
   - Track the latest forward-pass date reached and accrue points only for positive movement of that global date.
   - After each processed node, append at most one generated node when the accumulator is at least one.
   - Give generated nodes deterministic, repeatable-across-iteration, collision-safe internal IDs based on their generation sequence; give them duration `generated_points / velocity`, no fixed dates, and dependencies as defined above.
   - Enforce the 1000-node processing limit and return a new `CriticalPathMethodError` variant before processing node 1001.
   - Build successors and run the existing backward pass only after dynamic generation has finished, so generated nodes participate in project-end and float calculations.
6. In `run_simulation`, use each iteration's result-node count to track the maximum processed count for `SimulationReport.simulated_items`. Collect work-package samples for both original and generated IDs, aggregating each ID's percentiles over only the iterations in which that ID occurs.
7. Update all critical-path call sites. `estimate_gantt` passes creation disabled; project simulation passes the enabled configuration only for positive-rate story-point simulations.
8. Add critical-path unit tests covering:
   - zero/disabled creation leaves dates and node counts unchanged;
   - calendar days accrue across weekends even though processing observes calendar capacity;
   - parallel original nodes use the global date high-water mark without double-counting elapsed days;
   - exact one-point accumulation triggers creation;
   - Fibonacci selection, one-node-per-event behavior, and retained remainder;
   - first generated dependency on all original terminal nodes and serial dependencies thereafter;
   - generated processing creates additional work;
   - stable-rate completion when creation is below processing velocity;
   - a non-terminating/high-rate case returns the limit error before node 1001.
9. Add project-simulation unit tests for `simulated_items` using the maximum processed count, inclusion of generated IDs in work-package details, deterministic generated IDs across iterations, and percentile calculations based only on a work package's occurrences.
10. Add integration tests in `tests/simulate_project_tests.rs` covering help/default parsing, a positive rate ignored with a warning for a non-story-point project, no warning at zero, a story-point forecast extended by generated work, invalid rates, and the 1000-node abort on stderr with a non-zero exit status.
11. Update the source documentation in `docs/src/cli-reference.md` if it contains checked-in option text, regenerate any derived CLI documentation through the repository's existing documentation workflow if applicable, then run formatting, focused tests, and the full Rust test suite.

## Validation commands for the implementation step

```sh
cargo fmt --check
cargo test critical_path_method
cargo test --test simulate_project_tests
cargo test
```