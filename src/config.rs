use crate::parse::{CommandTags};
use std::path::PathBuf;
use std::fs::read_to_string;
use anyhow::Context;
use anyhow::Result;
use crate::ParserConfig;


pub static DEFAULT_TAG_BEGIN: &'static str = "<!--{";
pub static DEFAULT_TAG_END: &'static str = "}-->";
pub static DEFAULT_END_COMMAND: &'static str = "end";

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(default)]
pub struct Config {
    pub open_tag: String,
    pub close_tag: String,
    pub end_command: String,
    pub base_dir: String,
    pub files: Vec<String>,
    pub next_dirs: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            open_tag: DEFAULT_TAG_BEGIN.to_string(),
            close_tag: DEFAULT_TAG_END.to_string(),
            end_command: DEFAULT_END_COMMAND.to_string(),
            base_dir: "".to_string(),
            files: vec![],
            next_dirs: vec![],
        }
    }
}

impl Config {
    pub(crate) fn try_from_path(dir: PathBuf) -> Result<Self> {
        let file = read_to_string(dir)?;
        Ok(toml::from_str::<Config>(&file)
            .context("Error in config file")?
            .into())
    }

    pub(crate) fn into_parser(self, dir: &PathBuf) -> (ParserConfig, Vec<PathBuf>) {
        (
            ParserConfig {
                tags: CommandTags::new(self.open_tag, self.close_tag),
                end_command: self.end_command,
                base_dir: dir.join(self.base_dir),
            },
            self.files.iter().map(|x| dir.join(x)).collect(),
        )
    }
    /// Returns None if no file exists
    /// Returns Some(Err) if file exists, but there was a problem reading it
    pub(crate) fn try_from_dir<P: Into<PathBuf>>(dir: P) -> Result<Option<Self>> {
        let dir = dir.into();
        let file = dir.join(".md-inc.toml");
        if file.exists() && file.is_file() {
            Ok(Some(Config::try_from_path(file)?))
        } else {
            Ok(None)
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct OutputTo {
    pub read_only: bool,
    pub print: bool,
}
impl OutputTo {
    pub fn stdout() -> Self {
        Self {
            read_only: true,
            print: true,
        }
    }
    pub fn file() -> Self {
        Self {
            read_only: false,
            print: false,
        }
    }
}


