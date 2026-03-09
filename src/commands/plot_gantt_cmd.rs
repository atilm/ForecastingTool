use crate::commands::base_commands::Commands;
use crate::commands::date_parsing;
use crate::services::estimate_gantt::write_pert_gantt_markdown;

pub fn plot_gantt_command(cmd: Commands) {
    if let Commands::PlotGantt {
        input,
        output,
        calendar_dir,
        start_date,
    } = cmd
    {
        let start_date = date_parsing::parse_date(&start_date).unwrap_or_else(|e| {
            eprintln!("Failed to parse start date: {e}");
            std::process::exit(1);
        });

        if let Err(e) =
            write_pert_gantt_markdown(&input, &output, start_date, calendar_dir.as_deref())
        {
            eprintln!("Failed to write Gantt diagram: {e:?}");
        } else {
            println!("Gantt diagram written to {output}");
        }
    }
}
