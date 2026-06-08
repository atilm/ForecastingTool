# Quickstart

## 1. Prepare input data

Create a `project.yaml` file with your issues and estimates.
See the Project YAML Format page for the supported estimate types, including `story_points`, `three_point`, `reference`, `histogram_reference`, and `milestone`.

## 2. Run a simulation

```bash
forecasts simulate project \
  --project-file project.yaml \
  --output-file report.yaml
```

## 3. Plot a gantt chart from simulation output

```bash
forecasts plot simulation-gantt \
  --report-file report.yaml \
  --output-file report.gantt.md
```

## 4. Generate shell completions (optional)

```bash
forecasts util completions bash > ~/.local/share/bash-completion/completions/forecasts
source ~/.local/share/bash-completion/completions/forecasts
```
