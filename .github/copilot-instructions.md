Write unit tests for public functions and methods, ensuring that they cover a variety of cases, including edge cases and error conditions.
I/O operations should be async.
Consistently use the `thiserror` crate for error handling, defining custom error types where appropriate.
Consistently use NaiveDate as data type for dates. When writing dates to strings, use the format "YYYY-MM-DD".
Use a clean coding style, following Rust conventions and best practices. This includes proper naming, modularization, and documentation.
Try to keep source files under 300 lines. If a file exceeds this limit, consider refactoring it into smaller modules.
Also split files when the single responsibility principle is violated, meaning that a file contains code that serves more than one purpose or functionality.