# Requirements Specification

* [ ] Download data from web APIs
  * [ ] Supported APIs
    * [ ] JIRA
  * [x] Export throughput data
    * [ ] Configurable by query string in config file
    * [ ] as csv ?
  * [ ] Export velocity data
  * [ ] Export list of issues with estimates
  * [ ] different output formats
    * [ ] `yaml`
    * [ ] markdown
* [ ] Generate Forecasts
  * [ ] By Monte Carlo Simulation
    * [x] Based on empirical throughput data
      * [x] choose daily throughput randomly
    * [ ] Based on empirical velocity as story / points per day
      * [ ] calculate velocity from 
      * [ ] choose size of work package randomly from beta distribution
        * [ ] by story point estimation e.g. in interval (3-5-8) for estimate 5
        * [ ] by three point estimation in days
      * [ ] Simulate dependencies between work packages
      * [ ] Simulate team capacity based on calendar for multiple team members
    * [x] Configure simulation type and data sources from config file
    * [ ] Simulate a project hierarchically
      * [ ] top level with dependencies based on three point estimate
      * [ ] update the three point estimate from detailed sub-simulations
  * [ ] Output simulation inputs and results
    * [ ] report in yaml format
      * [x] used input source
      * [x] different percentiles: 0, 50, 85, 100
      * [x] durations in days
    * [ ] `stdout`
      * [x] info from yaml report, plus
      * [x] start and completion dates
    * [ ] Dependency diagram (mermaid)
    * [ ] Gantt diagram (mermaid)
    * [ ] Histogram (`png`)
    * [ ] Burn down chart (`png`)