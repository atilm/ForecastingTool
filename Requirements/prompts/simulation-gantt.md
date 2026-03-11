Add a new command plot-simulation-gantt to the cli. The command

* The command should take the following parameters
  * a project file path
  * a report file path
  * an output file path
* should read a project.yaml using #project_yaml.rs and a report.yaml file using #simulation_report_yaml.rs
* It should create a markdown file with a mermaid gantt diagram following these rules:
  * Each WorkPackageSimulation from the simulation report should be a work_package in the gantt chart
  * The work package should be labeled with the work package id and the summary from the project.yaml
  * The work package's start date should be the latest 85th percentile end_date of the work package's dependencies.
  * The work package's end date should be its 85th percentile end_date
  * Show milestones as milestones

Implement unit tests for the new module.
Also implement one integration test in the tests directory.