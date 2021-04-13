use crate::gui::{Icon, Theme};
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
pub struct Args {
    #[structopt(short, long, parse(from_occurrences))]
    pub verbose: u8,
    #[structopt(long, group = "action")]
    pub pacman_notify: bool,
    #[structopt(long, group = "action")]
    pub debug_inotify: bool,
    /// Show a specific icon to debug your design
    #[structopt(long, group = "action")]
    pub debug_icon: Option<Icon>,
    /// Name of the icon theme
    #[structopt(long)]
    pub icon_theme: Option<Theme>,
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
