use anyhow::Result;
use directories::ProjectDirs;

use serde::{Deserialize, Serialize};
use std::fs;

const CONFIG_FILE_NAME: &str = "config.toml";
const DEFAULT_LANGUAGE: &str = "en_US";

use crate::error::XwError;
#[derive(Serialize, Deserialize)]
pub struct Config {
    pub istest: bool,
    pub language: String,
    pub favorite: Vec<(String, String)>, // (address, name)
}

fn read_config(path: &str) -> Result<Config, XwError> {
    let data = fs::read(path)?;
    let text = String::from_utf8(data)?;
    let config: Config = toml::from_str(&text)?;
    Ok(config)
}

fn write_config(config: &Config, path: &str) -> Result<(), XwError> {
    let text = toml::to_string(config)?;
    std::fs::write(path, text)?;
    Ok(())
}

pub fn get_config() -> Result<Config, XwError> {
    let proj_dir =
        ProjectDirs::from("com", "xdagger", "xdag plus").ok_or(XwError::ConfigLocationError)?;
    let path = proj_dir.config_dir().join(CONFIG_FILE_NAME);
    let prefix = path.parent().unwrap();
    if !prefix.exists() {
        let default_config = Config {
            istest: false,
            language: DEFAULT_LANGUAGE.to_string(),
            favorite: vec![(
                "PKcBtHWDSnAWfZntqWPBLedqBShuKSTzS".to_string(),
                "Community Fund".to_string(),
            )],
        };
        std::fs::create_dir_all(prefix)?;
        set_config(&default_config)?;
        return Ok(default_config);
    }
    let path_str = path.to_str().ok_or(XwError::ConfigPath2StrError)?;
    let mut config = read_config(path_str)?;

    if config.favorite.is_empty() {
        config.favorite.push((
            "PKcBtHWDSnAWfZntqWPBLedqBShuKSTzS".to_string(),
            "Community Fund".to_string(),
        ));
    } else if config.favorite[0].0 != "PKcBtHWDSnAWfZntqWPBLedqBShuKSTzS".to_string() {
        config.favorite[0].0 = "PKcBtHWDSnAWfZntqWPBLedqBShuKSTzS".to_string();
        config.favorite[0].1 = "Community Fund".to_string();
    }

    Ok(config)
}

pub fn set_config(config: &Config) -> Result<(), XwError> {
    let proj_dir =
        ProjectDirs::from("com", "xdagger", "xdag plus").ok_or(XwError::ConfigLocationError)?;
    let path = proj_dir.config_dir().join(CONFIG_FILE_NAME);
    let path_str = path.to_str().ok_or(XwError::ConfigPath2StrError)?;
    write_config(config, path_str)?;
    Ok(())
}
#[cfg(test)]
mod test {

    use super::*;
    #[test]
    fn test_write_config() {
        let config = Config {
            istest: false,
            language: "en".to_string(),
            favorite: vec![("key1".to_string(), "value1".to_string())],
        };
        let path = "./test_config.toml";
        let ret = write_config(&config, path);
        assert!(ret.is_ok());
    }
    #[test]
    fn test_read_config() {
        let path = "./test_config.toml";
        let ret = read_config(path);
        assert!(ret.is_ok());
        let config = ret.unwrap();
        assert_eq!(config.language, "en");
        assert_eq!(config.favorite.len(), 1);
        assert_eq!(config.favorite[0].0, "key1");
        assert_eq!(config.favorite[0].1, "value1");
    }

    #[test]
    fn test_set_config() {
        let config = Config {
            istest: true,
            language: "en_US".to_string(),
            favorite: vec![(
                "4duPWMbYUgAifVYkKDCWxLvRRkSByf5gb".to_string(),
                "community".to_string(),
            )],
        };
        let ret = set_config(&config);
        assert!(ret.is_ok());
        let ret = get_config();
        assert!(ret.is_ok());
        let config = ret.unwrap();
        assert_eq!(config.language, "en_US");
        assert_eq!(config.favorite.len(), 1);
        assert_eq!(config.favorite[0].0, "4duPWMbYUgAifVYkKDCWxLvRRkSByf5gb");
        assert_eq!(config.favorite[0].1, "community");
    }
}
