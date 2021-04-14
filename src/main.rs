mod args;
mod config;
mod errors;
mod gui;
mod notify;
mod updater;

use crate::args::Args;
use crate::config::Config;
use crate::errors::*;
use env_logger::Env;
use structopt::StructOpt;

fn main() -> Result<()> {
    let args = Args::from_args();

    env_logger::init_from_env(Env::default()
        .default_filter_or(args.log_level()));

    let config = Config::load(&args)
        .context("Failed to load config")?;

    if args.pacman_notify {
        notify::pacman_notify()
    } else if args.debug_inotify {
        notify::debug_inotify()
    } else if let Some(icon) = &args.debug_icon {
        gui::debug_icon(&config, &icon)
    } else {
        gui::main(&config)
    }
}
