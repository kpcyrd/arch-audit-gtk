mod args;

use anyhow::{anyhow, bail, Result, Context};
use arch_audit::types::{Avg, Severity};
use crate::args::Args;
use env_logger::Env;
use inotify::{Inotify, WatchMask};
use std::borrow::Cow;
use log::{warn, info, debug};
use rand::Rng;
use std::env;
use std::fs::File;
use std::path::Path;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};
use std::process::Command;
use structopt::StructOpt;
use gtk::prelude::*;
use libappindicator::{AppIndicator, AppIndicatorStatus};

const CHECK_FOR_UPDATE: &str = "Check for updates";
const CHECKING: &str = "Checking...";
const QUIT: &str = "Quit";

// TODO: there should be a startup delay so we check after eg 5min
// TODO: we should check how long ago the last update check was
const CHECK_INTERVAL: u64 = 3600 * 2; // 2 hours
const CHECK_JITTER: u64 = 3600 * 4; // 4 hours

#[derive(Debug)]
pub struct Update {
    severity: Severity,
    pkg: String,
    text: String,
    link: String,
}

#[derive(Debug)]
enum Status {
    MissingUpdates(Vec<Update>),
    Error(String),
}

impl Status {
    fn text(&self) -> Cow<'_, str> {
        match self {
            Status::MissingUpdates(list) => match list.len() {
                0 => Cow::Borrowed("No missing security updates"),
                1 => Cow::Borrowed("1 missing security update"),
                n => Cow::Owned(format!("{} missing security updates", n)),
            },
            Status::Error(err) => Cow::Owned(format!("ERROR: {}", err))
        }
    }

    fn icon(&self) -> &'static str {
        match self {
            Status::MissingUpdates(list) => if list.is_empty() {
                "check"
            } else {
                "alert"
            },
            Status::Error(_) => "cross",
        }
    }
}

#[derive(Debug)]
enum Event {
    Click,
    Inotify,
}

fn check_updates() -> Result<Vec<Update>> {
    // Select the arch-audit binary
    let bin = env::var("ARCH_AUDIT_BIN");
    let bin = bin.as_ref()
        .map(|x| x.as_str())
        .unwrap_or("arch-audit");

    // Run the arch-audit binary
    let output = Command::new(bin)
        .args(&["-u", "--json"])
        .output()
        .context("Failed to run arch-audit")?;

    info!("arch-audit exited: {}", output.status);

    if output.status.success() {
        // if arch-audit didn't indicate an error, parse the output as json
        let affected: Vec<Avg> = serde_json::from_slice(&output.stdout)
            .context("Failed to parse arch-audit json output")?;

        let mut updates = affected.into_iter()
            .flat_map(|avg| {
                avg.packages.iter().map(|pkg| {
                    let text = format!("{}: {} ({})", avg.severity, pkg, avg.kind);
                    Update {
                        severity: avg.severity,
                        pkg: pkg.to_string(),
                        text,
                        link: format!("https://security.archlinux.org/{}", avg.name),
                    }
                }).collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();

        updates.sort_by(|a, b| {
            a.severity.cmp(&b.severity).reverse()
            .then(a.pkg.cmp(&b.pkg))
        });

        if !updates.is_empty() {
            info!("Missing security updates: {:?}", output);
        }

        Ok(updates)
    } else {
        let err = String::from_utf8_lossy(&output.stderr);
        let err = err.trim();
        bail!("{}", err);
    }
}

fn background(update_rx: mpsc::Receiver<Event>, result_tx: glib::Sender<Status>) {
    loop {
        info!("Checking for security updates...");
        let arch_audit_result = check_updates();
        let (needs_updates, msg) = match arch_audit_result {
            Ok(updates) => (!updates.is_empty(), Status::MissingUpdates(updates)),
            Err(e) => (true, Status::Error(format!("{:#}", e))),
        };
        result_tx.send(msg).ok();
        info!("Finished checking for security updates");

        let mut rng = rand::thread_rng();
        let jitter = rng.gen_range(0..CHECK_JITTER);
        let delay = Duration::from_secs(CHECK_INTERVAL + jitter);

        let start = Instant::now();
        while let Some(remaining) = delay.checked_sub(start.elapsed()) {
            info!("Sleeping for {}", humantime::format_duration(remaining));
            if let Ok(event) = update_rx.recv_timeout(remaining) {
                debug!("Received event: {:?}", event);
                match event {
                    Event::Click => break,
                    Event::Inotify => {
                        if needs_updates {
                            break;
                        } else {
                            info!("There are no missing security updates so we aren't checking if we're missing any");
                        }
                    },
                }
            } else {
                break;
            }
        }
    }
}

fn gtk_main(args: Args) -> Result<()> {
    gtk::init()?;

    // TODO: consider a mutex and condvar so we don't queue multiple updates
    let (update_tx, update_rx) = mpsc::channel();
    let (result_tx, result_rx) = glib::MainContext::channel(glib::PRIORITY_DEFAULT);

    setup_inotify_thread(update_tx.clone())?;

    thread::spawn(move || {
        background(update_rx, result_tx);
    });

    let mut indicator = AppIndicator::new("arch-audit-gtk", "");

    indicator.set_status(AppIndicatorStatus::Active);

    'outer: for path in &["./icons", "/usr/share/arch-audit-gtk/icons"] {
        for theme in &[&args.icon_theme, "default"] {
            if let Ok(theme_path) = Path::new(path).join(theme).canonicalize() {
                let icon = theme_path.join("check.svg");
                if icon.exists() {
                    indicator.set_icon_theme_path(theme_path.to_str().unwrap());
                    break 'outer;
                }
            }
        }
    }
    indicator.set_icon_full("check", "icon"); // TODO: should indicate we're still fetching the status

    let mut m = gtk::Menu::new();

    let checking_mi = gtk::MenuItem::with_label(CHECK_FOR_UPDATE);
    m.append(&checking_mi);
    let mi = checking_mi.clone();
    checking_mi.connect_activate(move |_| {
        mi.set_label(CHECKING);
        update_tx.send(Event::Click).unwrap();
    });

    let status_mi = gtk::MenuItem::with_label("Starting...");
    m.append(&status_mi);

    let mi = gtk::MenuItem::with_label(QUIT);
    m.append(&mi);
    mi.connect_activate(|_| {
        gtk::main_quit();
    });

    indicator.set_menu(&mut m);
    m.show_all();

    result_rx.attach(None, move |msg| {
        log::info!("Received from thread: {:?}", msg);

        // update text in main menu
        checking_mi.set_label(CHECK_FOR_UPDATE);
        status_mi.set_label(&msg.text());

        match msg {
            Status::MissingUpdates(ref updates) if !updates.is_empty() => {
                let m = gtk::Menu::new();

                for update in updates {
                    let mi = gtk::MenuItem::with_label(&update.text);
                    m.append(&mi);
                    let link = update.link.to_string();
                    mi.connect_activate(move |_| {
                        if let Err(err) = opener::open(&link) {
                            eprintln!("Failed to open link: {:#}", err);
                        }
                    });
                }

                m.show_all();
                status_mi.set_submenu(Some(&m));
            },
            _ => {
                status_mi.set_submenu(None::<&gtk::Menu>);
            }
        }

        indicator.set_icon_full(msg.icon(), "icon");

        glib::Continue(true)
    });

    gtk::main();

    Ok(())
}

fn pacman_notify_main() -> Result<()> {
    let path = "/run/arch-audit-gtk/notify";
    File::create(path)
        .with_context(|| anyhow!("Failed to touch file: {:?}", path))?;
    Ok(())
}

fn setup_inotify_thread(tx: mpsc::Sender<Event>) -> Result<()> {
    let mut inotify = Inotify::init()
        .context("Failed to init inotify")?;

    // Watch for modify and close events.
    let result = inotify
        .add_watch(
            "/run/arch-audit-gtk",
            WatchMask::CLOSE_WRITE,
        );

    if let Err(err) = result {
        warn!("Failed to add file watch: {:#}", err);
    } else {
        thread::spawn(move || {
            // Read events that were added with `add_watch` above.
            let mut buffer = [0; 1024];

            loop {
                let events = inotify.read_events_blocking(&mut buffer)
                    .expect("Error while reading events");

                // we don't need to send multiple signals, one is enough
                debug!("Received events: {:?}", events.collect::<Vec<_>>());
                if tx.send(Event::Inotify).is_err() {
                    break;
                }
            }
        });
    }

    Ok(())
}

fn debug_inotify_main() -> Result<()> {
    let (tx, rx) = mpsc::channel();
    setup_inotify_thread(tx)?;

    for event in rx {
        println!("{:?}", event);
    }

    Ok(())
}

fn main() -> Result<()> {
    let args = Args::from_args();

    env_logger::init_from_env(Env::default()
        .default_filter_or(args.log_level()));

    // Ensure the theme name can not be exploited for path traversal or
    // other havoc. After this point icon_theme is safe to be used within
    // a path.
    if args.icon_theme.contains(|ch| !('a'..='z').contains(&ch)) {
        panic!("Invalid theme name. Only characters a to z are allowed.");
    }

    if args.pacman_notify {
        pacman_notify_main()
    } else if args.debug_inotify {
        debug_inotify_main()
    } else {
        gtk_main(args)
    }
}
