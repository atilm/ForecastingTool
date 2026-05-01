# ForecastingTool

[![CI](https://github.com/atilm/ForecastingTool/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/atilm/ForecastingTool/actions/workflows/ci.yml)
[![Docs](https://github.com/atilm/ForecastingTool/actions/workflows/docs.yml/badge.svg?branch=main)](https://github.com/atilm/ForecastingTool/actions/workflows/docs.yml)

A project estimation and forecasting tool using monte carlo simulations

User documentation: https://atilm.github.io/ForecastingTool/

## Simulation by story points

Structure your `project.yaml` like this:

* Done tasks first
* The first done task needs to have `dependencies = null`
* Each task will depend implicitly on the task before it
* If you need a special start_date of the first todo task, you must set it explicitly
* **The simulation start date cli argument will only have an effect, if the start date of a task without dependencies is not set!** 

## Install Shell Completions

```shell
forecasts util completions bash > ~/.local/share/bash-completion/completions/forecasts
source ~/.local/share/bash-completion/completions/forecasts
```

## Publishing releases

To publish a release, push a tag like git tag v1.0.0 && git push origin v1.0.0.