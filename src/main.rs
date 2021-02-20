use std::borrow::Cow;
use log::info;
use std::env;
use std::path::Path;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;
use std::process::Command;

use gtk::prelude::*;
use libappindicator::{AppIndicator, AppIndicatorStatus};

const CHECK_FOR_UPDATE: &str = "Check for updates";
const CHECKING: &str = "Checking...";
const QUIT: &str = "Quit";

// TODO: there should be a startup delay so we check after eg 5min
// TODO: we should check how long ago the last update check was
const CHECK_INTERVAL: u64 = 3600 * 6;

#[derive(Debug)]
enum Status {
    MissingUpdates(usize),
    Error(String),
}

impl Status {
    fn text(&self) -> Cow<'_, str> {
        match self {
            Status::MissingUpdates(0) => Cow::Borrowed("No missing security updates"),
            Status::MissingUpdates(1) => Cow::Borrowed("1 missing security update"),
            Status::MissingUpdates(n) => Cow::Owned(format!("{} missing security updates", n)),
            Status::Error(err) => Cow::Borrowed(err)
        }
    }

    fn icon(&self) -> &'static str {
        match self {
            Status::MissingUpdates(0) => "check",
            Status::MissingUpdates(_) => "alert",
            Status::Error(_) => "error",
        }
    }
}

fn update() -> anyhow::Result<usize> {
    let output = Command::new("arch-audit")
        .args(&["-u"])
        .output()?;

    info!("arch-audit exited: {}", output.status);

    if output.status.success() {
        if output.stdout.is_empty() {
            Ok(0)
        } else {
            let output = String::from_utf8_lossy(&output.stdout);
            let output = output.trim().split('\n').collect::<Vec<_>>();
            info!("Missing security updates: {:?}", output);
            Ok(output.len())
        }
    } else {
        let err = String::from_utf8_lossy(&output.stderr);
        let err = err.trim();
        anyhow::bail!("{}", err);
    }
}

fn background(update_rx: mpsc::Receiver<()>, result_tx: glib::Sender<Status>) {
    loop {
        let _ = update_rx.recv_timeout(Duration::from_secs(CHECK_INTERVAL));

        info!("Checking for security updates...");
        let msg = update()
            .map(Status::MissingUpdates)
            .unwrap_or_else(|e| Status::Error(format!("{:#}", e)));
        result_tx.send(msg).ok();
        info!("Finished checking for security updates");
    }
}

fn main() -> anyhow::Result<()> {
    env_logger::init();
    gtk::init()?;

    // TODO: consider a mutex and condvar so we don't queue multiple updates
    let (update_tx, update_rx) = mpsc::channel();
    let (result_tx, result_rx) = glib::MainContext::channel(glib::PRIORITY_DEFAULT);

    thread::spawn(move || {
        background(update_rx, result_tx);
    });

    let mut indicator = AppIndicator::new("arch-audit-gtk", "");

    indicator.set_status(AppIndicatorStatus::Active);

    let icon_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("icons");
    indicator.set_icon_theme_path(icon_path.to_str().unwrap());
    indicator.set_icon_full("check", "icon"); // TODO: should indicate we're still fetching the status

    let mut m = gtk::Menu::new();

    let checking_mi = gtk::MenuItem::with_label(CHECK_FOR_UPDATE);
    m.append(&checking_mi);
    let mi = checking_mi.clone();
    checking_mi.connect_activate(move |_| {
        mi.set_label(CHECKING);
        update_tx.send(()).unwrap();
    });

    let status_mi = gtk::MenuItem::with_label(&format!("Starting..."));
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

        checking_mi.set_label(CHECK_FOR_UPDATE);
        status_mi.set_label(&msg.text());
        indicator.set_icon_full(msg.icon(), "icon");

        glib::Continue(true)
    });

    gtk::main();

    Ok(())
}

#[cfg(test)]
mod tests { 
  use super::*;

  #[test]
  fn test() {
  }
}
