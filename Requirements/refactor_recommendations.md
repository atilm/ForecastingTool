# High-impact refactors (services focus)

* Split oversized “god modules” into submodules with single responsibilities:
  * project_simulation.rs currently mixes: YAML entrypoint, date parsing, dependency graph building, sampling, calendar scheduling, percentile/stats, and report assembly (and is ~700+ LOC).
  * project_yaml.rs mixes: YAML schema structs, project <-> record mapping, date/status parsing, report-file parsing for reference estimates, and serialization (also ~500+ LOC).
  * Suggested shape:
  *   services/project_simulation/{mod.rs, error.rs, graph.rs, duration.rs, calendar_schedule.rs, engine.rs, stats.rs}
  *   services/project_yaml/{mod.rs, error.rs, schema.rs, parse.rs, write.rs, report_reference.rs, date.rs}
  * Benefit: easier navigation, smaller diffs, clearer test targets, fewer “helper” functions with wide visibility.
* Extract shared utilities (currently duplicated):
    * data_source_name() exists in both simulation.rs and project_simulation.rs.
    * Percentile helpers are duplicated (and a third variant exists in gantt_diagram.rs).
    * Optional calendar loading (load_team_calendar_if_provided) is duplicated in both simulators.
    * Recommendation: one small module like services/util/{mod.rs, dates.rs, stats.rs, io.rs} (or similar) and re-use everywhere.
* Make APIs more idiomatic by pushing parsing to the edges:
    * Today several service functions take &str dates and parse internally (e.g. simulate_project(... start_date: &str ...) in project_simulation.rs, and throughput simulation does similar in simulation.rs).
    * Prefer: parse in commands/* once and pass NaiveDate down. This reduces error variants, avoids repeated format strings, and makes services more reusable from non-CLI callers.
* Fix the “dates as strings” inconsistency in your domain-ish DTOs:
  * Your own project guideline says “Consistently use NaiveDate as data type for dates”, but SimulationReport.start_date and SimulationPercentile.date are String in simulation_types.rs.
  * Recommendation: store NaiveDate in SimulationReport/SimulationPercentile and use Serde helpers to serialize as "YYYY-MM-DD". This removes a lot of parsing/validation code in YAML/report paths and prevents invalid dates from ever existing in-memory.

# Error handling & boundaries

Consolidate error “shape” and preserve sources:

Many errors currently drop the underlying cause (e.g. DataSourceError::Connection, Parse) in data_source.rs and mapping in jira_api.rs.
Prefer typed variants with #[from] and source fields (still using thiserror), e.g. Reqwest(reqwest::Error), SerdeJson(serde_json::Error), Io(std::io::Error) where it helps debugging.
Consider a layered approach: jira_api::Error (rich, includes HTTP status/body where safe) that converts into a more general DataSourceError if you want to keep the trait stable.
Tighten visibility:

mod.rs exports everything as pub mod ... (mod.rs). If the library API isn’t meant to be stable for all services, change most service fns/types to pub(crate) and expose only what commands/* need (or re-export a curated API from services).
This alone tends to improve maintainability because you can refactor internals without ripple effects.

# Algorithm/structure improvements that also help readability

Simplify dependency sorting in project simulation:

In project_simulation.rs, topological_sort builds id_by_index even though the graph already stores String node weights—ordered can be built directly from sorted by looking up node weights.
Also consider modeling IDs as a small newtype (e.g. WorkPackageId(String)) to avoid accidental mixing of “issue IDs” vs other strings.
Separate “immutable model” from “per-run state”:

SimulationNode currently stores samples: Vec<f32> and is reused across iterations. This works, but it couples configuration (id/estimate/deps) with results accumulation.
A more idiomatic split is NodeConfig { id, estimate, deps } + RunState { earliest_finish map } + Results { per_node_samples }. It makes the simulation loop easier to reason about and test.

# Formatting & string-building

Prefer fmt::Write + writeln! for generated markdown:
gantt_diagram.rs and project_flow_diagram.rs push lots of String lines; using a single String buffer with writeln!(&mut out, ...) reduces noise and allocations.
Also centralize date formatting rules (you currently use both %Y-%m-%d and DD-MM-YYYY in Mermaid output).

# Jira/data-source layer

Make DataQuery more specific:
DataQuery::StringQuery(String) in data_source.rs forces allocations and loses intent. Consider DataQuery::Jql(String) (and/or DataQuery::ProjectKey(String) etc.), or accept &str/Cow<'a, str> if you want zero-copy.
Remove thin wrapper types:
JiraConfigParser in jira_api.rs doesn’t add much beyond JiraProjectMetaData::from_yaml_file; collapsing that reduces surface area.

# Tests (what to add next)

You already have solid unit tests in simulation modules. Two high-value additions:
Property-ish tests for percentile helpers (monotonicity, bounds at 0/100, empty input behavior) once you extract them into a shared module.
Deterministic simulation tests for project simulation by injecting RNG/sampler (you already do this nicely for throughput via run_simulation_with_rng in simulation.rs).
If you want, I can pick one concrete refactor to implement end-to-end (with tests) as a starting point—my suggestion would be extracting a shared services::stats percentile module + de-duplicating it across the three call sites, because it’s low-risk and immediately improves readability.