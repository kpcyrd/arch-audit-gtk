use crate::errors::*;
use inotify::{Inotify, WatchMask};
use std::fs::File;
use std::sync::mpsc;
use std::thread;

#[derive(Debug)]
pub enum Event {
    Click,
    Inotify,
}

pub fn pacman_notify() -> Result<()> {
    let path = "/run/arch-audit-gtk/notify";
    File::create(path)
        .with_context(|| anyhow!("Failed to touch file: {:?}", path))?;
    Ok(())
}

pub fn setup_inotify_thread(tx: mpsc::Sender<Event>) -> Result<()> {
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

pub fn debug_inotify() -> Result<()> {
    let (tx, rx) = mpsc::channel();
    setup_inotify_thread(tx)?;

    for event in rx {
        println!("{:?}", event);
    }

    Ok(())
}
