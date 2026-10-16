# CLI Reference

Use help output to inspect all commands:

```bash
forecasts --help
```

Inspect a command group:

```bash
forecasts simulate --help
forecasts plot --help
forecasts get --help
```

Inspect a concrete command:

```bash
forecasts simulate project --help
```

Tip: keep this page concise and rely on command help text for exact flags as they evolve.

## Project Milestone Tracking

`forecasts simulate project` creates or updates a milestone tracking YAML file on
every successful run, including runs without milestones:

```bash
forecasts simulate project --input projects/project.yaml --output report.yaml \
	--start-date 2026-01-01 --milestones-file tracking/milestones.yaml
```

`--milestones-file <PATH>` is optional. Without it, the command uses
`milestones.yaml` in the input project's directory, independently of the working
directory. An explicitly supplied relative path is relative to the process working
directory. The destination's parent directory must already exist.

```yaml
start_date: 2026-01-01
end_date: 2026-09-14
milestones:
	- id: MS_1
		name: First milestone title
		plot: true
		forecasts:
			- date: 2026-02-25
				p0: 2026-03-24
				p15: 2026-06-10
				p50: 2026-07-09
				p85: 2026-08-14
				p100: 2026-09-14
```

Each simulated milestone is matched by ID and uses its project summary as `name`.
The command refreshes names and records all five percentile end dates using the
machine-local calendar date as the forecast's `date`. A later run on the same date
replaces that milestone's forecast; a run on another date appends one. Milestones
absent from a run and their history are retained.

New milestones default to `plot: true`; an omitted `plot` also defaults to `true`.
Existing values are preserved for manual editing. This field is metadata only.

`start_date` is initialized from `--start-date` and preserved thereafter.
`end_date` becomes the latest p100 end date among milestones in the current run.
For a new file on a run without milestones, it uses the overall simulation p100;
later runs without milestones preserve the existing bounds.

Missing milestone summaries, invalid tracking YAML (including missing required
fields other than `plot`), and tracking-file read or write failures fail the command.
