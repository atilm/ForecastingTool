use std::collections::HashMap;

use chrono::NaiveDate;
use petgraph::algo::toposort;
use petgraph::graph::DiGraph;
use thiserror::Error;

use crate::domain::calendar::TeamCalendar;

#[derive(Error, Debug)]
pub enum CriticalPathMethodError {
    #[error("The network contains duplicate node IDs: {0}")]
    DuplicateNodeId(String),
    #[error("The network contains a cycle, which is not allowed in a project schedule.")]
    CycleDetected,
    #[error("A node has a dependency on a non-existent node: {0}")]
    MissingDependency(String),
    #[error(
        "The provided calendar does not have enough capacity to complete the task within a reasonable time frame."
    )]
    InsufficientCalendarCapacity,
}

#[derive(Debug)]
pub struct NetworkNode {
    pub id: String,
    pub duration: f32,
    pub start_date: Option<NaiveDate>,
    pub end_date: Option<NaiveDate>,
    pub dependencies: Vec<String>,
}

pub struct ResultNode {
    pub id: String,
    pub earliest_start: NaiveDate,
    pub latest_start: NaiveDate,
    pub earliest_finish: NaiveDate,
    pub latest_finish: NaiveDate,
    pub free_float: f32,  // ES of next node - EF of current node.
    pub total_float: f32, // LS - ES or LF - EF. If 0, then the node is on the critical path
}

impl ResultNode {
    /// Returns true if this node is on the critical path (total float is zero).
    pub fn is_critical(&self) -> bool {
        self.total_float.abs() < f32::EPSILON
    }

    /// Returns true if this node is a milestone (zero duration).
    pub fn is_milestone(&self) -> bool {
        self.earliest_start == self.earliest_finish
    }
}

pub fn critical_path_method(
    network: Vec<NetworkNode>,
    project_start: NaiveDate,
    calendar: Option<&TeamCalendar>,
) -> Result<Vec<ResultNode>, CriticalPathMethodError> {
    let nodes_count = network.len();
    let sorted_nodes = topological_sort(network)?;

    let mut earliest_finish_dates: HashMap<String, chrono::NaiveDate> =
        HashMap::with_capacity(nodes_count);
    let mut result_nodes: HashMap<String, ResultNode> = HashMap::with_capacity(nodes_count);

    // Forward pass to calculate earliest start and finish times
    for node in &sorted_nodes {
        let earliest_start = if let Some(start_date) = node.start_date {
            start_date
        } else {
            node.dependencies
                .iter()
                .filter_map(|dep| earliest_finish_dates.get(dep))
                .max()
                .cloned()
                .unwrap_or(project_start)
        };

        let earliest_finish = if let Some(end_date) = node.end_date {
            end_date
        } else {
            calculate_end_date(earliest_start, node.duration, calendar)?
        };

        earliest_finish_dates.insert(node.id.clone(), earliest_finish);

        result_nodes.insert(
            node.id.clone(),
            ResultNode {
                id: node.id.clone(),
                earliest_start,
                latest_start: project_start, // Placeholder, will be calculated in backward pass
                earliest_finish,
                latest_finish: project_start, // Placeholder, will be calculated in backward pass 
                free_float: 0.0, // Placeholder, will be calculated in backward pass
                total_float: 0.0, // Placeholder, will be calculated in backward pass
            },
        );
    }

    // Build successor map (reverse of dependencies)
    let mut successors: HashMap<String, Vec<String>> = HashMap::new();
    for node in &sorted_nodes {
        successors.entry(node.id.clone()).or_default();
        for dep in &node.dependencies {
            successors
                .entry(dep.clone())
                .or_default()
                .push(node.id.clone());
        }
    }

    // Find project end date = max of all earliest finish dates
    let project_end = earliest_finish_dates
        .values()
        .max()
        .cloned()
        .unwrap_or(project_start);

    // Backward pass to calculate latest start and finish times
    let mut latest_start: HashMap<String, chrono::NaiveDate> = HashMap::with_capacity(nodes_count);

    for node in sorted_nodes.iter().rev() {
        let latest_finish_date = successors[&node.id]
            .iter()
            .filter_map(|succ| latest_start.get(succ))
            .min()
            .cloned()
            .unwrap_or(project_end);

        let latest_start_date = calculate_start_date(latest_finish_date, node.duration, calendar)?;

        let earliest_start_of_successors = successors[&node.id]
            .iter()
            .filter_map(|succ| result_nodes.get(succ))
            .map(|succ_node| succ_node.earliest_start)
            .min()
            .unwrap_or(project_end);
        let free_float =
            (earliest_start_of_successors - earliest_finish_dates[&node.id]).num_days() as f32;

        if let Some(result_node) = result_nodes.get_mut(&node.id) {
            result_node.latest_finish = latest_finish_date;
            result_node.latest_start = latest_start_date;
            result_node.free_float = free_float.max(0.0); // Free float cannot be negative
            result_node.total_float =
                (latest_start_date - result_node.earliest_start).num_days() as f32;
        }

        latest_start.insert(node.id.clone(), latest_start_date);
    }

    let result_vector: Vec<ResultNode> = sorted_nodes
        .into_iter()
        .map(|node| result_nodes.remove(&node.id).unwrap())
        .collect();

    Ok(result_vector)
}

fn calculate_end_date(
    start_date: chrono::NaiveDate,
    duration_days: f32,
    calendar: Option<&TeamCalendar>,
) -> Result<chrono::NaiveDate, CriticalPathMethodError> {
    if duration_days <= 0.0 {
        return Ok(start_date);
    }

    if let Some(calendar) = calendar {
        end_date_from_capacity_days(start_date, duration_days, calendar)
    } else {
        let whole_days = duration_days.ceil() as i64;
        Ok(start_date + chrono::Duration::days(whole_days))
    }
}

fn calculate_start_date(
    end_date: chrono::NaiveDate,
    duration_days: f32,
    calendar: Option<&TeamCalendar>,
) -> Result<chrono::NaiveDate, CriticalPathMethodError> {
    if duration_days <= 0.0 {
        return Ok(end_date);
    }

    if let Some(calendar) = calendar {
        start_date_from_capacity_days(end_date, duration_days, calendar)
    } else {
        let whole_days = duration_days.ceil() as i64;
        Ok(end_date - chrono::Duration::days(whole_days))
    }
}

fn end_date_from_capacity_days(
    start_date: chrono::NaiveDate,
    days_at_full_capacity: f32,
    calendar: &TeamCalendar,
) -> Result<chrono::NaiveDate, CriticalPathMethodError> {
    if days_at_full_capacity <= 0.0 {
        return Ok(start_date);
    }

    let mut remaining_capacity = days_at_full_capacity;
    let mut date = start_date;
    for _ in 0..(365 * 200) {
        let todays_capacity_fraction = calendar.get_capacity(date);
        if todays_capacity_fraction > 0.0 {
            remaining_capacity -= todays_capacity_fraction;
        }

        date += chrono::Duration::days(1);

        if remaining_capacity <= 0.0 {
            let next_non_zero_capacity_date = (0..365 * 200)
                .map(|i| date + chrono::Duration::days(i))
                .find(|d| calendar.get_capacity(*d) > 0.0)
                .unwrap_or(date); // If we can't find a non-zero capacity date within a reasonable time frame, just return the current date

            return Ok(next_non_zero_capacity_date);
        }
    }

    Err(CriticalPathMethodError::InsufficientCalendarCapacity)
}

fn start_date_from_capacity_days(
    end_date: chrono::NaiveDate,
    days_at_full_capacity: f32,
    calendar: &TeamCalendar,
) -> Result<chrono::NaiveDate, CriticalPathMethodError> {
    if days_at_full_capacity <= 0.0 {
        return Ok(end_date);
    }

    let mut remaining_capacity = days_at_full_capacity;
    let mut date = end_date;
    for _ in 0..(365 * 200) {
        date -= chrono::Duration::days(1);

        let todays_capacity_fraction = calendar.get_capacity(date);
        if todays_capacity_fraction > 0.0 {
            remaining_capacity -= todays_capacity_fraction;
        }

        if remaining_capacity <= 0.0 {
            return Ok(date);
        }
    }

    Err(CriticalPathMethodError::InsufficientCalendarCapacity)
}

fn topological_sort(
    network: Vec<NetworkNode>,
) -> Result<Vec<NetworkNode>, CriticalPathMethodError> {
    let mut graph: DiGraph<String, ()> = DiGraph::new();
    let mut nodes_by_index = HashMap::new();
    let mut index_by_id = HashMap::new();

    // Add nodes to graph
    for node in network {
        let graph_node_index = graph.add_node(node.id.clone());

        if index_by_id.contains_key(&node.id) {
            return Err(CriticalPathMethodError::DuplicateNodeId(node.id.clone()));
        }

        index_by_id.insert(node.id.clone(), graph_node_index);
        nodes_by_index.insert(graph_node_index, node);
    }

    // Add edges to graph based on dependencies
    for (graph_node_index, node) in &nodes_by_index {
        for dependency in &node.dependencies {
            let dependency_index = index_by_id
                .get(dependency)
                .ok_or_else(|| CriticalPathMethodError::MissingDependency(dependency.clone()))?;
            graph.add_edge(*dependency_index, *graph_node_index, ());
        }
    }

    // Perform topological sort
    let sorted_indices =
        toposort(&graph, None).map_err(|_| CriticalPathMethodError::CycleDetected)?;

    // Create sorted vector of nodes based on sorted indices
    let sorted_nodes: Vec<NetworkNode> = sorted_indices
        .iter()
        .map(|index| nodes_by_index.remove(index).unwrap())
        .collect();

    Ok(sorted_nodes)
}

#[cfg(test)]
mod tests {
    use crate::test_support::on_date;

    use super::*;
    use chrono::NaiveDate;

    fn build_network_node(id: &str, duration: f32, dependencies: &[&str]) -> NetworkNode {
        NetworkNode {
            id: id.to_string(),
            duration,
            start_date: None,
            end_date: None,
            dependencies: dependencies.into_iter().map(|s| s.to_string()).collect(),
        }
    }

    #[test]
    fn duplicate_node_ids_are_detected() {
        let network = vec![
            build_network_node("WP1", 1.0, &[]),
            build_network_node("WP1", 1.0, &[]), // Duplicate ID
        ];
        let project_start = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
        let result = critical_path_method(network, project_start, None);
        assert!(matches!(
            result,
            Err(CriticalPathMethodError::DuplicateNodeId(_))
        ));
    }

    #[test]
    fn empty_input_leads_to_empty_output() {
        let network = vec![];
        let project_start = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
        let result = critical_path_method(network, project_start, None).unwrap();
        assert!(result.is_empty());
    }

    // The dependency graph for the test is:
    //
    //    WP0      WP1
    //     |        |
    //     |        |
    //     |    +---+----+
    //     |    |        |
    //     +---WP2      WP3
    //     |    |        |
    //     +----+--+-----+
    //            |
    //           FIN
    #[test]
    fn a_topologically_sorted_vec_of_result_nodes_is_returned() {
        let network = vec![
            build_network_node("WP3", 1.0, &["WP1"]),
            build_network_node("FIN", 0.0, &["WP0", "WP2", "WP3"]),
            build_network_node("WP1", 1.0, &[]),
            build_network_node("WP2", 1.0, &["WP0", "WP1"]),
            build_network_node("WP0", 1.0, &[]),
        ];
        let project_start = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
        let result = critical_path_method(network, project_start, None).unwrap();
        let expected_order = vec!["WP0", "WP1", "WP2", "WP3", "FIN"];
        let result_order: Vec<String> = result.iter().map(|node| node.id.clone()).collect();
        assert_eq!(result_order, expected_order);
    }

    struct WorkPackageTestCase {
        id: &'static str,
        duration: f32,
        dependencies: Vec<&'static str>,
        expected_earliest_start_day: f32,
        expected_earliest_finish_day: f32,
        expected_latest_start_day: f32,
        expected_latest_finish_day: f32,
        expected_free_float: f32,
        expected_total_float: f32,
    }

    impl WorkPackageTestCase {
        fn new(
            id: &'static str,
            duration: f32,
            dependencies: Vec<&'static str>,
            expected_earliest_start_day: f32,
            expected_earliest_finish_day: f32,
            expected_latest_start_day: f32,
            expected_latest_finish_day: f32,
            expected_free_float: f32,
            expected_total_float: f32,
        ) -> Self {
            Self {
                id,
                duration,
                dependencies,
                expected_earliest_start_day,
                expected_earliest_finish_day,
                expected_latest_start_day,
                expected_latest_finish_day,
                expected_free_float,
                expected_total_float,
            }
        }
    }

    #[test]
    fn critical_path_method_is_calculated_correctly() {
        struct TestCase {
            id: &'static str,
            work_packages: Vec<WorkPackageTestCase>,
        }

        let base = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();

        let tests = vec![
            TestCase {
                id: "Test Case 1",
                work_packages: vec![
                    WorkPackageTestCase::new("WP0", 1.0, vec![], 0.0, 1.0, 0.0, 1.0, 0.0, 0.0),
                    WorkPackageTestCase::new("WP1", 1.0, vec![], 0.0, 1.0, 0.0, 1.0, 0.0, 0.0),
                    WorkPackageTestCase::new(
                        "WP2",
                        1.0,
                        vec!["WP0", "WP1"],
                        1.0,
                        2.0,
                        1.0,
                        2.0,
                        0.0,
                        0.0,
                    ),
                    WorkPackageTestCase::new("WP3", 1.0, vec!["WP1"], 1.0, 2.0, 1.0, 2.0, 0.0, 0.0),
                    WorkPackageTestCase::new(
                        "FIN",
                        0.0,
                        vec!["WP0", "WP2", "WP3"],
                        2.0,
                        2.0,
                        2.0,
                        2.0,
                        0.0,
                        0.0,
                    ),
                ],
            },
            TestCase {
                id: "Test Case 2",
                work_packages: vec![
                    WorkPackageTestCase::new("WP0", 6.0, vec![], 0.0, 6.0, 0.0, 6.0, 0.0, 0.0),
                    WorkPackageTestCase::new("WP1", 1.0, vec![], 0.0, 1.0, 4.0, 5.0, 0.0, 4.0),
                    WorkPackageTestCase::new(
                        "WP2",
                        0.0,
                        vec!["WP0", "WP1"],
                        6.0,
                        6.0,
                        6.0,
                        6.0,
                        0.0,
                        0.0,
                    ),
                    WorkPackageTestCase::new("WP3", 1.0, vec!["WP1"], 1.0, 2.0, 5.0, 6.0, 4.0, 4.0),
                    WorkPackageTestCase::new(
                        "FIN",
                        0.0,
                        vec!["WP0", "WP2", "WP3"],
                        6.0,
                        6.0,
                        6.0,
                        6.0,
                        0.0,
                        0.0,
                    ),
                ],
            },
            TestCase {
                id: "Test Case 3",
                work_packages: vec![
                    WorkPackageTestCase::new("WP0", 2.0, vec![], 0.0, 2.0, 0.0, 2.0, 0.0, 0.0),
                    WorkPackageTestCase::new("WP1", 1.0, vec![], 0.0, 1.0, 1.0, 2.0, 0.0, 1.0),
                    WorkPackageTestCase::new(
                        "WP2",
                        4.0,
                        vec!["WP0", "WP1"],
                        2.0,
                        6.0,
                        2.0,
                        6.0,
                        0.0,
                        0.0,
                    ),
                    WorkPackageTestCase::new("WP3", 1.0, vec!["WP1"], 1.0, 2.0, 5.0, 6.0, 4.0, 4.0),
                    WorkPackageTestCase::new(
                        "FIN",
                        0.0,
                        vec!["WP0", "WP2", "WP3"],
                        6.0,
                        6.0,
                        6.0,
                        6.0,
                        0.0,
                        0.0,
                    ),
                ],
            },
            TestCase {
                id: "Test Case 4",
                work_packages: vec![
                    WorkPackageTestCase::new("WP0", 1.0, vec![], 0.0, 1.0, 4.0, 5.0, 4.0, 4.0),
                    WorkPackageTestCase::new("WP1", 5.0, vec![], 0.0, 5.0, 0.0, 5.0, 0.0, 0.0),
                    WorkPackageTestCase::new(
                        "WP2",
                        2.0,
                        vec!["WP0", "WP1"],
                        5.0,
                        7.0,
                        5.0,
                        7.0,
                        0.0,
                        0.0,
                    ),
                    WorkPackageTestCase::new("WP3", 1.0, vec!["WP1"], 5.0, 6.0, 6.0, 7.0, 1.0, 1.0),
                    WorkPackageTestCase::new(
                        "FIN",
                        0.0,
                        vec!["WP0", "WP2", "WP3"],
                        7.0,
                        7.0,
                        7.0,
                        7.0,
                        0.0,
                        0.0,
                    ),
                ],
            },
            TestCase {
                id: "Test Case 5",
                work_packages: vec![
                    WorkPackageTestCase::new("WP0", 1.0, vec![], 0.0, 1.0, 7.0, 8.0, 4.0, 7.0),
                    WorkPackageTestCase::new("WP1", 5.0, vec![], 0.0, 5.0, 0.0, 5.0, 0.0, 0.0),
                    WorkPackageTestCase::new(
                        "WP2",
                        1.0,
                        vec!["WP0", "WP1"],
                        5.0,
                        6.0,
                        8.0,
                        9.0,
                        3.0,
                        3.0,
                    ),
                    WorkPackageTestCase::new("WP3", 4.0, vec!["WP1"], 5.0, 9.0, 5.0, 9.0, 0.0, 0.0),
                    WorkPackageTestCase::new(
                        "FIN",
                        0.0,
                        vec!["WP0", "WP2", "WP3"],
                        9.0,
                        9.0,
                        9.0,
                        9.0,
                        0.0,
                        0.0,
                    ),
                ],
            },
        ];

        for test in tests {
            let network: Vec<NetworkNode> = test
                .work_packages
                .iter()
                .map(|wp| {
                    build_network_node(
                        wp.id,
                        wp.duration,
                        &wp.dependencies.iter().map(|s| *s).collect::<Vec<_>>(),
                    )
                })
                .collect();

            let result = critical_path_method(network, base, None).unwrap();

            for wp in &test.work_packages {
                let result_node = result.iter().find(|node| node.id == wp.id).unwrap();
                assert_eq!(
                    result_node.earliest_start,
                    base + chrono::Duration::days(wp.expected_earliest_start_day as i64),
                    "Earliest start mismatch for {} in {}",
                    wp.id,
                    test.id
                );
                assert_eq!(
                    result_node.earliest_finish,
                    base + chrono::Duration::days(wp.expected_earliest_finish_day as i64),
                    "Earliest finish mismatch for {} in {}",
                    wp.id,
                    test.id
                );
                assert_eq!(
                    result_node.latest_start,
                    base + chrono::Duration::days(wp.expected_latest_start_day as i64),
                    "Latest start mismatch for {} in {}",
                    wp.id,
                    test.id
                );
                assert_eq!(
                    result_node.latest_finish,
                    base + chrono::Duration::days(wp.expected_latest_finish_day as i64),
                    "Latest finish mismatch for {} in {}",
                    wp.id,
                    test.id
                );
                assert!(
                    (result_node.free_float - wp.expected_free_float).abs() < 0.01,
                    "Free float mismatch for {} in {}",
                    wp.id,
                    test.id
                );
                assert!(
                    (result_node.total_float - wp.expected_total_float).abs() < 0.01,
                    "Total float mismatch for {} ({} vs. {}) in {}",
                    wp.id,
                    result_node.total_float,
                    wp.expected_total_float,
                    test.id,
                );
            }
        }
    }

    #[test]
    fn done_tasks_and_tasks_with_fixed_start_date_are_handled_correctly() {
        let network = vec![
            NetworkNode {
                id: "WP0".to_string(),
                duration: 3.0,
                start_date: Some(on_date(2026, 1, 5)),
                end_date: Some(on_date(2026, 1, 8)),
                dependencies: vec![],
            },
            NetworkNode {
                id: "WP1".to_string(),
                duration: 4.0,
                start_date: Some(on_date(2026, 1, 12)),
                end_date: None,
                dependencies: vec!["WP0".to_string()],
            },
            NetworkNode {
                id: "WP2".to_string(),
                duration: 4.0,
                start_date: None,
                end_date: None,
                dependencies: vec!["WP1".to_string()],
            },
        ];
        // Project start does not have any effect in this case
        let project_start = NaiveDate::from_ymd_opt(2026, 6, 6).unwrap();
        let result = critical_path_method(network, project_start, None).unwrap();

        let wp0 = result.iter().find(|node| node.id == "WP0").unwrap();
        assert_eq!(wp0.earliest_start, on_date(2026, 1, 5));
        assert_eq!(wp0.earliest_finish, on_date(2026, 1, 8));

        let wp1 = result.iter().find(|node| node.id == "WP1").unwrap();
        assert_eq!(wp1.earliest_start, on_date(2026, 1, 12));
        assert_eq!(wp1.earliest_finish, on_date(2026, 1, 16));

        let wp2 = result.iter().find(|node| node.id == "WP2").unwrap();
        assert_eq!(wp2.earliest_start, on_date(2026, 1, 16));
        assert_eq!(wp2.earliest_finish, on_date(2026, 1, 20));
    }

    #[test]
    fn when_a_calendar_is_given_it_is_applied() {
        use crate::domain::calendar::Calendar;
        use crate::domain::calendar::FreeDateRange;
        use chrono::Weekday;

        let calendar = TeamCalendar {
            calendars: vec![
                Calendar {
                    free_weekdays: vec![Weekday::Sat, Weekday::Sun],
                    free_date_ranges: vec![FreeDateRange {
                        start_date: on_date(2026, 1, 12),
                        end_date: on_date(2026, 1, 23),
                    }],
                },
                Calendar {
                    free_weekdays: vec![Weekday::Sat, Weekday::Sun],
                    free_date_ranges: vec![],
                },
            ],
        };

        let project_start = NaiveDate::from_ymd_opt(2026, 1, 5).unwrap();

        let network = vec![
            build_network_node("WP0", 5.0, &[]), // Should be finished after 5 days on Sat. 10th.
            build_network_node("WP1", 3.0, &["WP0"]), // Should take 6 days, because we work at half capacity. -> Finished on Tue. 20th
        ];

        let result =
            critical_path_method(network, project_start, Some(&calendar)).unwrap();

        let wp0 = result.iter().find(|node| node.id == "WP0").unwrap();
        assert_eq!(wp0.earliest_start, project_start);
        assert_eq!(wp0.earliest_finish, on_date(2026, 1, 12));

        let wp1 = result.iter().find(|node| node.id == "WP1").unwrap();
        assert_eq!(wp1.earliest_start, on_date(2026, 1, 12));
        assert_eq!(wp1.earliest_finish, on_date(2026, 1, 20));
    }

    /// A linear chain of tasks (WP0 -> WP1 -> WP2 -> WP3) spanning several weeks
    /// with a calendar that has weekends off. All tasks should be on the critical path
    /// because there is only one path through the network.
    #[test]
    fn linear_sequence_with_calendar_all_tasks_on_critical_path() {
        use crate::domain::calendar::Calendar;
        use chrono::Weekday;

        let calendar = TeamCalendar {
            calendars: vec![Calendar {
                free_weekdays: vec![Weekday::Sat, Weekday::Sun],
                free_date_ranges: vec![],
            }],
        };

        // Start on Monday 2026-01-05
        let project_start = on_date(2026, 1, 5);

        // Linear chain: WP0 -> WP1 -> WP2 -> WP3
        // Each task takes 5 working days (one full work week).
        let network = vec![
            build_network_node("WP0", 5.0, &[]),
            build_network_node("WP1", 5.0, &["WP0"]),
            build_network_node("WP2", 5.0, &["WP1"]),
            build_network_node("WP3", 5.0, &["WP2"]),
        ];

        let result = critical_path_method(network, project_start, Some(&calendar)).unwrap();

        // WP0: Mon 5 Jan - Fri 9 Jan (next working day: Mon 12 Jan)
        let wp0 = result.iter().find(|n| n.id == "WP0").unwrap();
        assert_eq!(wp0.earliest_start, on_date(2026, 1, 5));
        assert_eq!(wp0.earliest_finish, on_date(2026, 1, 12));

        // WP1: Mon 12 Jan - Fri 16 Jan (next working day: Mon 19 Jan)
        let wp1 = result.iter().find(|n| n.id == "WP1").unwrap();
        assert_eq!(wp1.earliest_start, on_date(2026, 1, 12));
        assert_eq!(wp1.earliest_finish, on_date(2026, 1, 19));

        // WP2: Mon 19 Jan - Fri 23 Jan (next working day: Mon 26 Jan)
        let wp2 = result.iter().find(|n| n.id == "WP2").unwrap();
        assert_eq!(wp2.earliest_start, on_date(2026, 1, 19));
        assert_eq!(wp2.earliest_finish, on_date(2026, 1, 26));

        // WP3: Mon 26 Jan - Fri 30 Jan (next working day: Mon 2 Feb)
        let wp3 = result.iter().find(|n| n.id == "WP3").unwrap();
        assert_eq!(wp3.earliest_start, on_date(2026, 1, 26));
        assert_eq!(wp3.earliest_finish, on_date(2026, 2, 2));

        // All tasks must be on the critical path (total_float == 0)
        for node in &result {
            assert!(
                node.is_critical(),
                "Node {} should be on the critical path but has total_float = {}",
                node.id,
                node.total_float,
            );
        }

        // Verify latest == earliest for all nodes (another way to confirm critical path)
        for node in &result {
            assert_eq!(
                node.earliest_start, node.latest_start,
                "Node {} latest_start should equal earliest_start on critical path",
                node.id,
            );
            assert_eq!(
                node.earliest_finish, node.latest_finish,
                "Node {} latest_finish should equal earliest_finish on critical path",
                node.id,
            );
        }
    }

    #[test]
    fn missing_dependency_is_detected() {
        let network = vec![
            build_network_node("WP0", 1.0, &["WP1"]), // WP1 does not exist
        ];
        let project_start = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
        let result = critical_path_method(network, project_start, None);
        assert!(matches!(
            result,
            Err(CriticalPathMethodError::MissingDependency(_))
        ));
    }

    #[test]
    fn cycle_detection_works() {
        let network = vec![
            build_network_node("WP0", 1.0, &["WP1"]),
            build_network_node("WP1", 1.0, &["WP0"]),
        ];
        let project_start = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
        let result = critical_path_method(network, project_start, None);
        assert!(matches!(
            result,
            Err(CriticalPathMethodError::CycleDetected)
        ));
    }
}
