use clap::Parser;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Directory path (optional)
    #[arg(long)]
    pub dir: Option<String>,

    /// DB filename (optional)
    #[arg(long)]
    pub dbfilename: Option<String>,
}
