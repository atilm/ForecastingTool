use std::io;

use thiserror::Error;

use crate::domain::project::Project;
use crate::services::project_yaml::{load_project_from_yaml_file, ProjectYamlError};

#[derive(Error, Debug)]
pub enum ProjectDiagramError {
    #[error("failed to read project yaml: {0}")]
    Read(#[from] io::Error),
    #[error("failed to parse project yaml: {0}")]
    Parse(#[from] ProjectYamlError),
}

pub async fn write_project_diagram_markdown(
    input_path: &str,
    output_path: &str,
) -> Result<(), ProjectDiagramError> {
    let project = load_project_from_yaml_file(input_path).await?;
    let markdown = generate_project_markdown(&project);
    tokio::fs::write(output_path, markdown).await?;
    Ok(())
}

pub fn generate_project_markdown(project: &Project) -> String {
    let diagram = generate_flow_diagram(project);
    let descriptions = generate_markdown_descriptions(project);
    if descriptions.is_empty() {
        format!("# Project Dependencies Diagram\n```mermaid\n{diagram}\n```\n")
    } else {
        format!(
            "# Project Dependencies Diagram\n```mermaid\n{diagram}\n```\n\n{descriptions}\n"
        )
    }
}

pub fn generate_flow_diagram(project: &Project) -> String {
    let mut lines = Vec::new();
    lines.push("flowchart TD".to_string());

    for issue in &project.work_packages {
        let id = issue.issue_id.as_ref().map(|id| id.id.as_str()).unwrap_or("");
        let name = issue
            .summary
            .as_deref()
            .unwrap_or(id);
        let label = format!("{id}\n    {name}");
        lines.push(format!("    {id}[{label}]"));
    }

    for issue in &project.work_packages {
        let id = issue.issue_id.as_ref().map(|id| id.id.as_str()).unwrap_or("");
        for dep in &issue.dependencies {
            lines.push(format!("    {} --> {id}", dep.id));
        }
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
        let id = issue.issue_id.as_ref().map(|id| id.id.as_str()).unwrap_or("");
        let name = issue.summary.as_deref().unwrap_or(id);
        blocks.push(format!("## {id}: {name}\n{description}"));
    }

    blocks.join("\n\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::project_yaml::deserialize_project_from_yaml_str;

    const YAML_CONTENT: &str = r#"
name: My Project
work_packages:
  - id: WP1
    summary: Work package 1
    description: |
      This is
      work package 1.
    dependencies: null
    estimate:
      type: story_points
      value: 5
  - id: WP2
    summary: Work package 2
    estimate:
      type: three_point
      optimistic: 1
      most_likely: 5
      pessimistic: 10
    dependencies: [WP1]
  - id: WP3
    summary: Work package 3
    description: |
      This is another
      work package 3.
    estimate:
      type: three_point
      optimistic: 1
      most_likely: 5
      pessimistic: 10
    dependencies: [WP1]
  - id: WP4
    summary: Work package 4
    estimate:
      type: three_point
      optimistic: 0
      most_likely: 0
      pessimistic: 0
    dependencies: [WP2, WP3]
"#;

    #[test]
    fn generate_flow_diagram_matches_expected() {
        let project = deserialize_project_from_yaml_str(YAML_CONTENT).unwrap();
        let diagram = generate_flow_diagram(&project);

        let expected = "flowchart TD\n    WP1[WP1\n    Work package 1]\n    WP2[WP2\n    Work package 2]\n    WP3[WP3\n    Work package 3]\n    WP4[WP4\n    Work package 4]\n    WP1 --> WP2\n    WP1 --> WP3\n    WP2 --> WP4\n    WP3 --> WP4";
        assert_eq!(diagram, expected);
    }

    #[test]
    fn generate_markdown_descriptions_includes_issue_text() {
        let project = deserialize_project_from_yaml_str(YAML_CONTENT).unwrap();
        let descriptions = generate_markdown_descriptions(&project);

        let expected = "## WP1: Work package 1\nThis is\nwork package 1.\n\n## WP3: Work package 3\nThis is another\nwork package 3.";
        assert!(descriptions.contains(expected));
    }
}
