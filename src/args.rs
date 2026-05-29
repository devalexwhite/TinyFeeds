use clap::Parser;

#[derive(Parser, Debug, Clone)]
#[clap(author, version, about, long_about = None)]
pub struct Args {
    #[arg(long, short, action)]
    pub dev_mode: bool,
}
