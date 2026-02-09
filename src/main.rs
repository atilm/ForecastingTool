mod domain;
mod services;

use clap::Parser;

#[derive(Parser)]
struct CliArgs {
    #[arg(short, long)]
    name: String,
}

fn main() {
    let args = CliArgs::parse();
    println!("Hello, {}!", args.name);
}
