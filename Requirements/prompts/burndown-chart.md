Add a new command plot-burndown to the application. The command should:

* parse a project yaml file using #project_yaml
* parse a result.yaml using #simulation_report_yaml.rs
* plot a burndown chart showing done issues with their actual end dates and showing in-progress and todo issues with their simulated end-dates

Valid issue estimates for this command are either StoryPointEstimate or None. If an estimate is None,
the command should convert it to a StoryPointEstimate of 1. If any issue has another type of Estimate,
the command should return an error. 

The burndown chart should have the following properties:

* The x-Axis is a date axis showing the range from the first actual end date of done issues until the latest p85 end date of the todo/in-progress issues
* The y-Axis should show the remaining story points at the date
* Each issue should be drawn as a filled circle at its actual or simulated end-date. The y-position should be the number of remaining story points a this date.
  Done issue circles should be gray, the other circles should be blue
* For not-done issues, three circles should be drawn corresponding to the 15th, 50th and 85th percentile. The area between the 15th percentile series and the 85th percentile series
  should be filled with a transparent light blue.

Implement the plot in a new module with unit tests.
Also implement an integration test in the tests directory.