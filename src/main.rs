mod args;
mod errors;
mod gui;
mod notify;
mod updater;

use crate::args::Args;
use crate::errors::*;
use crate::gui::Icon;
use env_logger::Env;
use structopt::StructOpt;

fn main() -> Result<()> {
    let args = Args::from_args();

    env_logger::init_from_env(Env::default()
        .default_filter_or(args.log_level()));

    // Ensure the theme name can not be exploited for path traversal or
    // other havoc. After this point icon_theme is safe to be used within
    // a path.
    if args.icon_theme.contains(|ch| !('a'..='z').contains(&ch)) {
        bail!("Invalid theme name. Only characters a to z are allowed.");
    }

    if args.pacman_notify {
        notify::pacman_notify()
    } else if args.debug_inotify {
        notify::debug_inotify()
    } else if let Some(icon) = &args.debug_icon {
        gui::debug_icon(&args, &icon)
    } else {
        gui::main(args)
    }
}
