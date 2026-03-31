# ToDos

* [ ] Status InProgress or Done and no start date should trigger an error
* [ ] Start-Date or End-Date set and non-matching status should also be an error
* [ ] Projects should either be all-story-points or no story-points
* [ ] Show InProgress Status in Gantt Diagram
* [.] I would like to be able to see InProgress tasks in the Gantt diagram with their actual start date and their currently estimated end date
  * Problem at the moment: report files of sub-projects contain only the remaining number of days
  * ~~This leads to the master simulation being wrong, when a start-date for InProgress tasks is specified~~
    * START-DATES ARE ALSO SPECIFIED IMPLICITLY THROUGH DEPENDENCIES!
      * -> When using references the end date is the START DATE UNDTIL TODAY + THE SIMULATED REMAINING TIME
      * in project_yaml.rs estimate_from_record could perform this conversion
  * [ ] Also done issues are not handled correctly in simulation_gantt.rs compute_start_date -> a set start date of the record is ignored
    * In the gantt, the end-date is then before the start-date
* [ ] Output errors to stderr
* [x] ~~Plot capacity in burn-down chart (perhaps background-transparency mapped to capacity)~~
* [ ] Milestone tracking yaml and plot as output of simulation
* [ ] Improve error reporting in project_yaml.rs

* [ ] If possible, make it easier to configure project simulations.
  * [x] ~~Add explicit milestones~~
  * [x] ~~Add simulation steps with fixed duration~~ -> Done by three-point estimates with equal values
  * [ ] Cases
    * [ ] Simulate subproject based on story-points
    * [ ] Simulate subproject based on throughput
    * [ ] Simulate subproject based on three-point estimations
    * [ ] Use multiple calendars to simulate increase of team size (e.g. 2 files to 3 files to increase velocity by factor 1.5)
    * [ ] Simulate master project file with three-point estimations and references

* [x] ~~Modify the CLI to use subcommands~~
* [.] Group services by command in directories
* [ ] Write a readme
* [ ] CI/CD
* [ ] Simulate in parallel
* [ ] Use the anyhow crate

# Requirements Specification

* ~~New Estimate Type: fixed time box~~ -> workaround: three-point-estimate with three equal values
* ~~Start-Date for Tasks, so that they can be plotted correctly~~

* [ ] Download data from web APIs
  * [ ] Supported APIs
    * [x] ~~JIRA~~
  * [x] ~~Export throughput data~~
    * [x] ~~Configurable by query string in config file~~
  * [x] ~~Export velocity data~~
  * [x] ~~Export list of issues with estimates~~
  * [x] ~~Plot data~~
    * [x] ~~Plot throughput data~~
  * [ ] different output / input formats
    * [x] ~~`yaml`~~
    * [ ] **markdown**
* [ ] Generate Forecasts
  * [ ] By Monte Carlo Simulation
    * [x] ~~start_date: configurable, but default is current date~~
    * [x] ~~Based on empirical throughput data~~
      * [x] ~~choose daily throughput randomly~~
      * [x] ~~Simulate team capacity based on calendar for multiple team members~~ 
    * [x] ~~Based on three-point estimations in absolute days (weekends and holidays included in these days and not handled)~~
    * [x] ~~Based on empirical velocity as story / points per day~~
      * [x] ~~calculate velocity from done tasks~~
      * [x] ~~choose size of work package randomly from beta distribution~~
        * [x] ~~by story point estimation e.g. in interval (3-5-8) for estimate 5~~
        * [x] ~~by three point estimation in days~~
      * [x] ~~Simulate dependencies between work packages~~
      * [x] ~~Simulate team capacity based on calendar for multiple team members~~
    * [x] ~~Configure data sources from config file~~
    * [ ] **Simulate a project hierarchically**
      * [ ] ~~top level with dependencies based on three point estimate~~
      * [ ] ~~update the three point estimate from detailed sub-simulations~~
        * [ ] specify the update command sequence in the project file for automatic execution
  * [x] ~~Output simulation inputs and results~~
    * [x] ~~report in yaml format~~
      * [x] ~~used input source~~
      * [x] ~~velocity~~
      * [x] ~~different percentiles: 0, 50, 85, 100~~
      * [x] ~~durations in days~~
      * [x] ~~end dates~~
    * [x]~ `stdout`~
      * [x] ~~info from yaml report, plus~~
      * [x] ~~start and completion dates~~
    * [x] ~~Dependency diagram (mermaid)~~
      * [ ] **Mark completed items in dependency chart**
    * [x] ~~Gantt diagram (mermaid)~~
      * [x] ~~Show completed items in Gantt chart~~
      * [x] ~~Add milestones to Gantt diagram~~
        * [x] ~~Display duration 0 entries as milestones in Gantt diagram~~
    * [x] ~~Histogram (`png`)~~
    * [x]~~ Plot box plot diagram of milestone finish percentiles (labels are not aligned yet)~~
    * [ ] Burn down chart (`png`)
  * [ ] Extra tool to update the project Readme
    * [ ] List of risks
    * [ ] Essential Gantt chart
    * [ ] ...