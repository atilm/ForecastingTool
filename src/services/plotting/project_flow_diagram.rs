use std::io;

use thiserror::Error;

use crate::domain::issue_status::IssueStatus;
use crate::domain::project::Project;
use crate::services::parsing::project_yaml::{ProjectYamlError, load_project_from_yaml_file};

#[derive(Error, Debug)]
pub enum ProjectDiagramError {
    #[error("failed to read project yaml: {0}")]
    Read(#[from] io::Error),
    #[error("failed to parse project yaml: {0}")]
    Parse(#[from] ProjectYamlError),
}

pub fn write_project_diagram_markdown(
    input_path: &str,
    output_path: &str,
) -> Result<(), ProjectDiagramError> {
    let project = load_project_from_yaml_file(input_path, &None)?;
    let markdown = generate_project_markdown(&project);
    std::fs::write(output_path, markdown)?;
    Ok(())
}

pub fn generate_project_markdown(project: &Project) -> String {
    let diagram = generate_flow_diagram(project);
    let descriptions = generate_markdown_descriptions(project);
    if descriptions.is_empty() {
        format!("# Project Dependencies Diagram\n```mermaid\n{diagram}\n```\n")
    } else {
        format!("# Project Dependencies Diagram\n```mermaid\n{diagram}\n```\n\n{descriptions}\n")
    }
}

pub fn generate_flow_diagram(project: &Project) -> String {
    let mut lines = Vec::new();
    lines.push("flowchart TD".to_string());
    lines.push("classDef class_done fill:#aae,stroke:#88a".to_string());
    lines.push("classDef class_progress fill:#eaa,stroke:#a88".to_string());

    for issue in &project.work_packages {
        let id = issue
            .issue_id
            .as_ref()
            .map(|id| id.id.as_str())
            .unwrap_or("");
        let name = issue.summary.as_deref().unwrap_or(id);
        let label = format!("{id}\n    {name}");
        let style = match issue.status {
            Some(IssueStatus::Done) => ":::class_done",
            Some(IssueStatus::InProgress) => ":::class_progress",
            _ => "",
        };
        if issue.is_milestone() {
            lines.push(format!("    {id}{{{label}}}{style}"));
        } else {
            lines.push(format!("    {id}[{label}]{style}"));
        }
    }

    for issue in &project.work_packages {
        let id = issue
            .issue_id
            .as_ref()
            .map(|id| id.id.as_str())
            .unwrap_or("");
        if let Some(deps) = issue.dependencies.as_ref() {
            for dep in deps {
                lines.push(format!("    {} --> {id}", dep.id));
            }
        }
    }

    let mut subgraph_map: std::collections::BTreeMap<String, Vec<String>> =
        std::collections::BTreeMap::new();
    for issue in &project.work_packages {
        if let Some(name) = issue.subgraph.as_deref() {
            subgraph_map.entry(name.to_string()).or_default().push(
                issue
                    .issue_id
                    .as_ref()
                    .map(|id| id.id.clone())
                    .unwrap_or_default(),
            );
        }
    }

    if !subgraph_map.is_empty() {
        lines.push(String::new());
        let mut names: Vec<_> = subgraph_map.keys().cloned().collect();
        names.sort();
        for (idx, name) in names.iter().enumerate() {
            let ids = &subgraph_map[name];
            lines.push(format!("    subgraph {name}"));
            for id in ids {
                lines.push(format!("        {id}"));
            }
            lines.push("    end".to_string());
            if idx + 1 < names.len() {
                lines.push(String::new());
            }
        }
        lines.push(String::new());
    }

    lines.join("\n")
}

pub fn generate_markdown_descriptions(project: &Project) -> String {
    let mut blocks = Vec::new();
    for issue in &project.work_packages {
        let description = match issue.description.as_deref() {
            Some(text) if !text.trim().is_empty() => text.trim_end(),
            _ => continue,
        };
        let id = issue
            .issue_id
            .as_ref()
            .map(|id| id.id.as_str())
            .unwrap_or("");
        let name = issue.summary.as_deref().unwrap_or(id);
        blocks.push(format!("## {id}: {name}\n{description}"));
    }

    blocks.join("\n\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::parsing::project_yaml::deserialize_project_from_yaml_str;

    const YAML_CONTENT: &str = concat!(
        "name: My Project\n",
        "work_packages:\n",
        "  - id: WP1\n",
        "    summary: Work package 1\n",
        "    status: done\n",
        "    start_date: 2026-01-01\n",
        "    done_date: 2026-01-05\n",
        "    description: |\n",
        "      This is\n",
        "      work package 1.\n",
        "    dependencies: null\n",
        "    estimate:\n",
        "      type: story_points\n",
        "      value: 5\n",
        "  - id: WP2\n",
        "    summary: Work package 2\n",
        "    status: inprogress\n",
        "    subgraph: Midphase\n",
        "    start_date: 2026-01-06\n",
        "    estimate:\n",
        "      type: three_point\n",
        "      optimistic: 1\n",
        "      most_likely: 5\n",
        "      pessimistic: 10\n",
        "    dependencies: [WP1]\n",
        "  - id: WP3\n",
        "    summary: Work package 3\n",
        "    description: |\n",
        "      This is another\n",
        "      work package 3.\n",
        "    subgraph: Endphase\n",
        "    estimate:\n",
        "      type: three_point\n",
        "      optimistic: 1\n",
        "      most_likely: 5\n",
        "      pessimistic: 10\n",
        "    dependencies: [WP1]\n",
        "  - id: WP4\n",
        "    summary: Work package 4\n",
        "    subgraph: Endphase\n",
        "    estimate:\n",
        "      type: milestone\n",
        "    dependencies: [WP2, WP3]\n",
    );

    #[test]
    fn generate_flow_diagram_matches_expected() {
        let project = deserialize_project_from_yaml_str(YAML_CONTENT, &None).unwrap();
        let diagram = generate_flow_diagram(&project);

        let expected = concat!(
            "flowchart TD\n",
            "classDef class_done fill:#aae,stroke:#88a\n",
            "classDef class_progress fill:#eaa,stroke:#a88\n",
            "    WP1[WP1\n",
            "    Work package 1]:::class_done\n",
            "    WP2[WP2\n",
            "    Work package 2]:::class_progress\n",
            "    WP3[WP3\n",
            "    Work package 3]\n",
            "    WP4{WP4\n",
            "    Work package 4}\n",
            "    WP1 --> WP2\n",
            "    WP1 --> WP3\n",
            "    WP2 --> WP4\n",
            "    WP3 --> WP4\n",
            "\n",
            "    subgraph Endphase\n",
            "        WP3\n",
            "        WP4\n",
            "    end\n",
            "\n",
            "    subgraph Midphase\n",
            "        WP2\n",
            "    end\n",
        );
        assert_eq!(diagram, expected);
    }

    #[test]
    fn generate_markdown_descriptions_includes_issue_text() {
        let project = deserialize_project_from_yaml_str(YAML_CONTENT, &None).unwrap();
        let descriptions = generate_markdown_descriptions(&project);

        let expected = "## WP1: Work package 1\nThis is\nwork package 1.\n\n## WP3: Work package 3\nThis is another\nwork package 3.";
        assert!(descriptions.contains(expected));
    }
}
