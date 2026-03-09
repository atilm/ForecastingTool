Ok, please do the following for me: Move the function get_project from the trait DataSource and its implementation
into a new concrete struct ProjectFactory. ProjectFactory should use a DataSource to get the issues defined by a DataQuery.
Then ProjectFactory should sort the issue the following way:

* first all issues with status Done
* then all issues with status InProgress
* last all issues with status ToDo

Furthermore Project Factory should set the dependency field of the following issues to null:

* The very first issue
* The first issue which is not Done

Then ProjectFactory should instantiate and return a Project.
Also implement unit tests and at least one integration test.
Please ask me any questions you need to gain more necessary context.