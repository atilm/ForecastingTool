# ForecastingTool
A project estimation and forecasting tool using monte carlo simulations

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