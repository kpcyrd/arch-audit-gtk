use crate::args::Args;
use crate::errors::*;
use crate::gui::Theme;
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};

pub struct Config {
    pub icon_theme: Theme,
}

impl Config {
    pub fn load(args: &Args) -> Result<Self> {
        let mut merged = ConfigFile::default();

        let mut configs = vec![PathBuf::from("/etc/arch-audit/gui.toml")];
        if let Some(dir) = dirs_next::config_dir() {
            let path = dir.join("arch-audit/gui.toml");
            configs.push(path);
        }

        for path in configs {
            let c = ConfigFile::load_from(&path)
                .with_context(|| anyhow!("Failed to load config file: {:?}"))?;
            if let Some(config) = c {
                debug!("Applying config from {:?}", path);
                merged.update(config);
            }
        }

        let mut config = Self {
            icon_theme: merged.design.icon_theme.unwrap_or_else(Theme::default),
        };

        if let Some(icon_theme) = &args.icon_theme {
            config.icon_theme = icon_theme.clone();
        }

        Ok(config)
    }
}

#[derive(Debug, Default, Deserialize)]
pub struct ConfigFile {
    #[serde(default)]
    design: DesignConfigFile,
}

impl ConfigFile {
    pub fn load_from<P: AsRef<Path>>(path: P) -> Result<Option<Self>> {
        let path = path.as_ref();
        if path.exists() {
            let file = fs::read_to_string(path)?;
            let cf = toml::from_str(&file)?;
            Ok(Some(cf))
        } else {
            Ok(None)
        }
    }

    pub fn update(&mut self, config: Self) {
        Self::update_field(&mut self.design.icon_theme, config.design.icon_theme);
    }

    pub fn update_field<T: PartialEq>(old: &mut Option<T>, new: Option<T>) {
        if let Some(new) = new {
            *old = Some(new);
        }
    }
}

#[derive(Debug, Default, Deserialize)]
pub struct DesignConfigFile {
    icon_theme: Option<Theme>,
}
