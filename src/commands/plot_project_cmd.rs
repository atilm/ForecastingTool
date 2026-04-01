use crate::commands::base_commands::PlotProjectArgs;
use crate::services::parsing::project_yaml::ProjectYamlError;
use crate::services::plotting::project_flow_diagram::ProjectDiagramError;
use crate::services::plotting::project_flow_diagram::write_project_diagram_markdown;

pub fn plot_project_command(args: PlotProjectArgs) {
    let PlotProjectArgs { input, output } = args;
    if let Err(error) = write_project_diagram_markdown(&input, &output) {
        print_diagram_error(&error);
    } else {
        println!("Project diagram written to {output}");
    }
}

fn print_diagram_error(error: &ProjectDiagramError) {
    match error {
        ProjectDiagramError::Parse(ProjectYamlError::Validation(validation_errors)) => {
            eprintln!(
                "Failed to write project diagram: project has {} validation error(s):",
                validation_errors.len()
            );
            for validation_error in validation_errors {
                eprintln!("  - {validation_error}");
            }
        }
        _ => eprintln!("Failed to write project diagram: {error}"),
    }
}
