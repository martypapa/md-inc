use crate::parse::CommandTags;
use crate::ParserConfig;
use anyhow::Context;
use anyhow::Result;
use std::fs::read_to_string;
use std::path::{Path, PathBuf};

pub static DEFAULT_TAG_BEGIN: &'static str = "<!--{";
pub static DEFAULT_TAG_END: &'static str = "}-->";
pub static DEFAULT_END_COMMAND: &'static str = "end";

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(default)]
pub struct Config {
    /// The opening tag for commands
    pub open_tag: String,

    /// The closing tag for commands
    pub close_tag: String,

    /// The command used to end a block
    pub end_command: String,

    /// Relative path of the base directory used to reference imported files
    pub base_dir: String,

    /// Relative paths of files to process
    pub files: Vec<String>,

    /// Relative paths of directories to process after this one
    pub next_dirs: Vec<String>,

    /// Relative paths of directories to process before this one
    pub depend_dirs: Vec<String>,

    /// Relative path of output directory
    pub out_dir: Option<String>,
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
            depend_dirs: vec![],
            out_dir: None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct ConfigAndPath {
    pub config: Config,
    pub path: PathBuf,
}

impl ConfigAndPath {
    pub(crate) fn parent_path(&self) -> Result<PathBuf> {
        Ok(self
            .path
            .parent()
            .context("Could not access parent directory of config file.")?
            .to_path_buf())
    }
    pub(crate) fn into_parser(self) -> Result<(ParserConfig, Vec<PathBuf>)> {
        let parent = self.parent_path()?;
        Ok((
            ParserConfig {
                tags: CommandTags::new(self.config.open_tag, self.config.close_tag),
                end_command: self.config.end_command,
                base_dir: parent.join(self.config.base_dir),
            },
            self.config.files.iter().map(|x| parent.join(x)).collect(),
        ))
    }
}
impl Config {
    pub(crate) fn try_from_path<P: AsRef<Path>>(dir: P) -> Result<Self> {
        let file =
            read_to_string(dir.as_ref()).with_context(|| format!("Reading {:?}", dir.as_ref()))?;
        Ok(toml::from_str::<Config>(&file)
            .context("Error in toml config file")?
            .into())
    }

    /// Returns None if no file exists
    /// Returns Some(Err) if file exists, but there was a problem reading it
    pub(crate) fn try_from_dir<P: Into<PathBuf>>(dir: P) -> Result<Option<ConfigAndPath>> {
        let dir = dir.into();
        let file = dir.join(".md-inc.toml");
        if file.exists() && file.is_file() {
            Ok(Some(ConfigAndPath {
                config: Config::try_from_path(&file)?,
                path: file,
            }))
        } else {
            Ok(None)
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct OutputTo {
    pub read_only: bool,
    pub print: bool,
    pub out_dir: Option<PathBuf>,
}
impl OutputTo {
    pub fn stdout() -> Self {
        Self {
            read_only: true,
            print: true,
            out_dir: None,
        }
    }
    pub fn file() -> Self {
        Self {
            read_only: false,
            print: false,
            out_dir: None,
        }
    }
    pub fn different_file<P: Into<PathBuf>>(path: P) -> Self {
        Self {
            read_only: false,
            print: false,
            out_dir: Some(path.into()),
        }
    }
}
