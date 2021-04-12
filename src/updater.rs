use arch_audit::types::{Avg, Severity};
use crate::errors::*;
use crate::gui::Icon;
use crate::notify::Event;
use rand::Rng;
use std::borrow::Cow;
use std::env;
use std::process::Command;
use std::sync::mpsc;
use std::time::{Duration, Instant};

// TODO: there should be a startup delay so we check after eg 5min
// TODO: we should check how long ago the last update check was
const CHECK_INTERVAL: u64 = 3600 * 2; // 2 hours
const CHECK_JITTER: u64 = 3600 * 4; // 4 hours

#[derive(Debug)]
pub enum Status {
    MissingUpdates(Vec<Update>),
    Error(String),
}

impl Status {
    pub fn text(&self) -> Cow<'_, str> {
        match self {
            Status::MissingUpdates(list) => match list.len() {
                0 => Cow::Borrowed("No missing security updates"),
                1 => Cow::Borrowed("1 missing security update"),
                n => Cow::Owned(format!("{} missing security updates", n)),
            },
            Status::Error(err) => Cow::Owned(format!("ERROR: {}", err))
        }
    }

    pub fn icon(&self) -> Icon {
        match self {
            Status::MissingUpdates(list) => {
                if list.is_empty() {
                    Icon::Check
                } else {
                    Icon::Alert
                }
            }
            Status::Error(_) => Icon::Cross,
        }
    }
}

#[derive(Debug)]
pub struct Update {
    pub severity: Severity,
    pub pkg: String,
    pub text: String,
    pub link: String,
}

pub fn check_for_updates() -> Result<Vec<Update>> {
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

pub fn background(update_rx: mpsc::Receiver<Event>, result_tx: glib::Sender<Status>) {
    loop {
        info!("Checking for security updates...");
        let arch_audit_result = check_for_updates();
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
