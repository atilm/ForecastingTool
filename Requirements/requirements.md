# ToDos

* [.] Take weekends and holidays into account
* [ ] Stop unit tests being run 2 times (1 for main and  1 for lib)
* [ ] Modify the CLI to use subcommands
* [ ] Write a readme
* [ ] CI/CD
* [ ] Compare current and legacy beta parameter calculation
* [ ] Simulate in parallel
* [ ] Use the anyhow crate

# Requirements Specification

* New Estimate Type: fixed time box
* Start-Date for Tasks, so that they can be plotted correctly

* [ ] Download data from web APIs
  * [ ] Supported APIs
    * [x] ~~JIRA~~
  * [x] ~~Export throughput data~~
    * [x] ~~Configurable by query string in config file~~
  * [x] ~~Export velocity data~~
  * [x] ~~Export list of issues with estimates~~
  * [x] ~~Plot data~~
    * [x] ~~Plot throughput data~~
  * [ ] different output formats
    * [x] ~~`yaml`~~
    * [ ] **markdown**
* [ ] Generate Forecasts
  * [ ] By Monte Carlo Simulation
    * [x] ~~start_date: configurable, but default is current date~~
    * [x] ~~Based on empirical throughput data~~
      * [x] ~~choose daily throughput randomly~~
    * [ ] Based on empirical velocity as story / points per day
      * [x] ~~calculate velocity from done tasks~~
      * [x] ~~choose size of work package randomly from beta distribution~~
        * [x] ~~by story point estimation e.g. in interval (3-5-8) for estimate 5~~
        * [x] ~~by three point estimation in days~~
      * [x] ~~Simulate dependencies between work packages~~
      * [ ] **Simulate team capacity based on calendar for multiple team members**
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
      * [ ] **Show completed items in Gantt chart**
      * [ ] Write to Gantt diagram in topological order?
      * [ ] Add milestones to Gantt diagram
        * [ ] Display duration 0 entries as milestones in Gantt diagram
    * [x] ~~Histogram (`png`)~~
    * [ ] Burn down chart (`png`)
  * [ ] Extra tool to update the project Readme
    * [ ] List of risks
    * [ ] Essential Gantt chart
    * [ ] ...