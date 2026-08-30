use crate::UpdateInterval;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Profile {
    pub name: String,
    pub path: String,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub update_interval: Option<UpdateInterval>,
    #[serde(default)]
    pub last_update: Option<String>,
    #[serde(default)]
    pub is_raw: bool,
}

impl Profile {
    pub fn new(name: impl Into<String>, path: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            path: path.into(),
            url: None,
            update_interval: None,
            last_update: None,
            is_raw: false,
        }
    }

    pub fn subscription(
        name: impl Into<String>,
        path: impl Into<String>,
        url: impl Into<String>,
        interval: UpdateInterval,
    ) -> Self {
        Self {
            name: name.into(),
            path: path.into(),
            url: Some(url.into()),
            update_interval: Some(interval),
            last_update: None,
            is_raw: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProfileStore {
    pub profiles: Vec<Profile>,
    pub active: Option<String>,
}

impl ProfileStore {
    pub fn find(&self, name: &str) -> Option<&Profile> {
        self.profiles.iter().find(|p| p.name == name)
    }

    pub fn find_mut(&mut self, name: &str) -> Option<&mut Profile> {
        self.profiles.iter_mut().find(|p| p.name == name)
    }

    pub fn add_or_replace(&mut self, profile: Profile) {
        if let Some(existing) = self.find_mut(&profile.name) {
            *existing = profile;
        } else {
            self.profiles.push(profile);
        }
    }

    pub fn remove(&mut self, name: &str) -> bool {
        let before = self.profiles.len();
        self.profiles.retain(|p| p.name != name);
        if self.active.as_deref() == Some(name) {
            self.active = None;
        }
        self.profiles.len() != before
    }
}

pub fn profiles_dir() -> Option<PathBuf> {
    crate::config_dir().map(|d| d.join("profiles"))
}

pub fn ensure_profiles_dir() -> anyhow::Result<PathBuf> {
    let dir = profiles_dir().ok_or_else(|| anyhow::anyhow!("no config dir"))?;
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

pub fn profile_store_path() -> Option<PathBuf> {
    crate::config_dir().map(|d| d.join("profiles.json"))
}

pub fn load_profile_store() -> ProfileStore {
    let Some(p) = profile_store_path() else {
        return ProfileStore::default();
    };
    if !p.exists() {
        return ProfileStore::default();
    }
    let Ok(s) = std::fs::read_to_string(&p) else {
        return ProfileStore::default();
    };
    serde_json::from_str(&s).unwrap_or_default()
}

pub fn save_profile_store(store: &ProfileStore) -> anyhow::Result<()> {
    let dir = crate::ensure_config_dir()?;
    let path = dir.join("profiles.json");
    let s = serde_json::to_string_pretty(store)?;
    crate::atomic_write(&path, s.as_bytes())?;
    Ok(())
}

pub fn profile_path_for_name(name: &str) -> Option<PathBuf> {
    let dir = profiles_dir()?;
    let safe: String = name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    Some(dir.join(format!("{safe}.yaml")))
}

pub fn import_profile_file(src: &Path, name: &str) -> anyhow::Result<Profile> {
    let dir = ensure_profiles_dir()?;
    let dest = dir.join(format!(
        "{}.yaml",
        name.chars()
            .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            })
            .collect::<String>()
    ));
    let content = std::fs::read(src)?;
    let _: serde_yaml::Value = serde_yaml::from_slice(&content)?;
    let tmp = dest.with_extension("tmp");
    std::fs::write(&tmp, &content)?;
    if dest.exists() {
        let _ = std::fs::remove_file(&dest);
    }
    std::fs::rename(&tmp, &dest)?;
    Ok(Profile::new(name, dest.display().to_string()))
}

pub fn import_profile_content(content: &str, name: &str) -> anyhow::Result<Profile> {
    let _: serde_yaml::Value = serde_yaml::from_str(content)?;
    let dir = ensure_profiles_dir()?;
    let dest = dir.join(format!(
        "{}.yaml",
        name.chars()
            .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            })
            .collect::<String>()
    ));
    crate::atomic_write(&dest, content.as_bytes())?;
    Ok(Profile::new(name, dest.display().to_string()))
}
