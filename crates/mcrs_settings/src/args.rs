use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    #[arg(short, long)]
    pub network_mode: Option<String>,

    #[arg(short, long)]
    pub address_server: Option<String>,

    #[arg(short, long)]
    pub view_distance: Option<u32>,
}
