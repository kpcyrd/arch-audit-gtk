use structopt::StructOpt;

#[derive(Debug, StructOpt)]
pub struct Args {
    #[structopt(short, long, parse(from_occurrences))]
    pub verbose: u8,
    #[structopt(long)]
    pub pacman_notify: bool,
    #[structopt(long)]
    pub debug_inotify: bool,
}

impl Args {
    pub fn log_level(&self) -> &str {
        match self.verbose {
            0 => "off",
            1 => "info",
            _ => "debug",
        }
    }
}
