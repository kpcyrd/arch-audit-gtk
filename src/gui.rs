use crate::args::Args;
use crate::updater::{self, Status};
use crate::errors::*;
use crate::notify::{Event, setup_inotify_thread};
use gtk::prelude::*;
use libappindicator::{AppIndicator, AppIndicatorStatus};
use std::path::Path;
use std::str::FromStr;
use std::sync::mpsc;
use std::thread;

const CHECK_FOR_UPDATE: &str = "Check for updates";
const CHECKING: &str = "Checking...";
const QUIT: &str = "Quit";

#[derive(Debug)]
pub enum Icon {
    Check,
    Alert,
    Cross,
}

impl Icon {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Check => "check",
            Self::Alert => "alert",
            Self::Cross => "cross",
        }
    }
}

impl FromStr for Icon {
    type Err = Error;

    fn from_str(s: &str) -> Result<Icon> {
        match s {
            "check" => Ok(Self::Check),
            "alert" => Ok(Self::Alert),
            "cross" => Ok(Self::Cross),
            _ => bail!("Invalid icon name: {:?}", s),
        }
    }
}

struct TrayIcon {
    indicator: AppIndicator,
}

impl TrayIcon {
    fn create(icon_theme: &str, icon: &Icon) -> Result<Self> {
        let mut indicator = AppIndicator::new("arch-audit-gtk", "");
        indicator.set_status(AppIndicatorStatus::Active);

        'outer: for path in &["./icons", "/usr/share/arch-audit-gtk/icons"] {
            for theme in &[icon_theme, "default"] {
                if let Ok(theme_path) = Path::new(path).join(theme).canonicalize() {
                    let icon = theme_path.join("check.svg");
                    if icon.exists() {
                        indicator.set_icon_theme_path(theme_path.to_str().unwrap());
                        break 'outer;
                    }
                }
            }
        }

        indicator.set_icon_full(icon.as_str(), "icon");

        Ok(TrayIcon { indicator })
    }

    pub fn set_icon(&mut self, icon: &Icon) {
        self.indicator.set_icon_full(icon.as_str(), "icon");
    }

    pub fn set_menu(&mut self, m: &mut gtk::Menu) {
        self.indicator.set_menu(m);
    }
}

pub fn main(args: Args) -> Result<()> {
    gtk::init()?;

    // TODO: consider a mutex and condvar so we don't queue multiple updates
    let (update_tx, update_rx) = mpsc::channel();
    let (result_tx, result_rx) = glib::MainContext::channel(glib::PRIORITY_DEFAULT);

    setup_inotify_thread(update_tx.clone())?;

    thread::spawn(move || {
        updater::background(update_rx, result_tx);
    });

    let mut tray_icon = TrayIcon::create(&args.icon_theme, &Icon::Check)?;

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

    tray_icon.set_menu(&mut m);
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

        tray_icon.set_icon(&msg.icon());

        glib::Continue(true)
    });

    gtk::main();

    Ok(())
}

pub fn debug_icon(args: &Args, icon: &Icon) -> Result<()> {
    gtk::init()?;

    let _tray_icon = TrayIcon::create(&args.icon_theme, icon)?;

    gtk::main();

    Ok(())
}
