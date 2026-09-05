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

## Implementation Plan

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