Add a new option to the `forecasts simulate project` command:

`-r --story-point-creation-rate  Daily rate with which story points are created [default: 0]`

1. When projects without story points are simulated, this option should not have any effect on the simulation, but
  a corresponding warning should be shown to the user.
2. When projects with story points are simulated, the simulation should comprise the creation of new issues:
   1. This should happen in the algorithm in `cricitcal_path_method.rs` The forward pass should track how many days have passed and
      keep track of how many story points have been created in the meantime according to the creation rate (in story points / day).
   2. As soon as the number of created story points is > 1, then the story points should be converted into network nodes which are
      appended to the current backlog. In this way I expect that further new issues are created while newly created issues are processed.
   3. This could lead to an endless loop when the cration rate is greater than the processing velocity. Because of this the simulation should
      abort when more than 1000 issues have been processed.

Also write unit and integration tests for the acceptance criteria above.