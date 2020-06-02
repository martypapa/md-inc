#[macro_use]
extern crate serde_derive;
use anyhow::{Context, Result};
use std::env::current_dir;
use std::fs::{read_to_string, File};
use std::io::Write;
use std::path::{Path, PathBuf};
pub use structopt::StructOpt;
mod config;
mod parse;
use crate::config::ConfigAndPath;
use crate::parse::Parser;
pub use crate::{
    config::{Config, OutputTo},
    parse::ParserConfig,
};

#[cfg(test)]
mod test;

///
/// Include files in Markdown docs
/// Can be after from command-line arguments using `Args::from_args()` (uses the `StructOpt` trait)
///
#[derive(Debug, StructOpt, Default, Clone)]
#[structopt(name = "md-inc", about = "Include files in Markdown docs")]
pub struct Args {
    ///
    /// A list of files to transform
    ///
    #[structopt(parse(from_os_str))]
    files: Vec<PathBuf>,

    ///
    /// An optional path to output the generated file to.
    /// If not present, the files will be inserted inline
    ///
    #[structopt(long = "out", parse(from_os_str))]
    out_dir: Option<PathBuf>,

    ///
    /// Override the opening tag for a command block
    ///
    #[structopt(
        short = "O",
        long,
        help = "Tag used for opening commands (default: '<!--|')"
    )]
    open_tag: Option<String>,

    ///
    /// Override the closing tag for a command block
    ///
    #[structopt(
        short = "C",
        long,
        help = "Tag used for closing commands (default: '|-->')"
    )]
    close_tag: Option<String>,

    ///
    /// Override the 'end' command name
    ///
    #[structopt(short, long, help = "Command used to end a block (default: 'end')")]
    end_command: Option<String>,

    ///
    /// The base directory used to reference imported files
    ///
    #[structopt(
        short = "b",
        long = "base-dir",
        parse(from_os_str),
        help = "Base directory used when referencing imports"
    )]
    base_dir: Option<PathBuf>,

    ///
    /// Set 1 or more working directories that may contain a '.md-inc.toml' config file.
    ///
    #[structopt(
        short = "d",
        long = "dir",
        parse(from_os_str),
        help = "Working directories"
    )]
    working_dir: Vec<PathBuf>,

    ///
    /// Ignore automatic detection of '.md-inc.toml' config files in the working directory
    ///
    #[structopt(
        short,
        long = "ignore-config",
        help = "Ignore '.md-inc.toml' files in the directory"
    )]
    ignore_config: bool,

    ///
    /// A custom '.toml' config file
    ///
    #[structopt(short, long, parse(from_os_str), help = "Path to a config file")]
    config: Option<PathBuf>,

    ///
    /// If true, the output is not written back to the file
    ///
    #[structopt(short = "R", long = "read-only", help = "Skip writing output to file")]
    read_only: bool,

    ///
    /// Scans all subdirectories for '.md-inc.toml' files
    ///
    #[structopt(
        short = "r",
        long = "recursive",
        help = "Run for all subfolders containing '.md-inc.toml'"
    )]
    recursive: bool,

    ///
    /// Searches the working directory for all matching config files
    /// and transforms files using each config file
    ///
    #[structopt(
        short = "g",
        long = "glob",
        help = "Custom globs used to match config files"
    )]
    glob: Vec<String>,

    ///
    /// Prints the transformed files to stdout
    ///
    #[structopt(short, long, help = "Print output to stdout")]
    print: bool,
}

///
/// Transforms a list of input files
///
/// # Returns
/// A Result containing the transformed input and vec of directories to also check
///
/// # Parameters
/// * `args` An struct of configuration settings.
///
///
pub fn transform_files_with_args(args: Args, config: Option<ConfigAndPath>) -> Result<Vec<String>> {
    let mut out_dir: Option<PathBuf> = None;
    if let Some(x) = args.working_dir.first() {
        if x.exists() {
            std::env::set_current_dir(&x)
                .with_context(|| format!("Could not set working directory: {:?}", &x))?;
        }
    }
    let (mut parser, files) = if let Some(cfg) = config {
        let parent = cfg.parent_path()?;
        out_dir = cfg.config.out_dir.as_ref().map(|x| parent.join(x));
        cfg.into_parser()?
    } else {
        (ParserConfig::default(), args.files)
    };

    if let Some(x) = args.out_dir {
        out_dir = Some(x);
    }
    if let Some(x) = args.open_tag {
        parser.tags.opening = x;
    }
    if let Some(x) = args.close_tag {
        parser.tags.closing = x;
    }
    if let Some(x) = args.end_command {
        parser.end_command = x;
    }
    if let Some(x) = args.base_dir {
        parser.base_dir = x;
    }
    transform_files(
        parser,
        &files,
        OutputTo {
            read_only: args.read_only,
            print: args.print,
            out_dir,
        },
    )
}

///
/// Transforms files
///
/// # Parameters
/// * `parser` A parser which contains override configuration and a base directory
/// * `files` A list of files to be transformed
/// * `prefs` Output configuration settings
///
/// # Example
///
/// ```
/// use md_inc::{transform_files, OutputTo, ParserConfig};
/// transform_files(ParserConfig::default(), &["README.md"], OutputTo::stdout());
/// ```
///
pub fn transform_files<P: AsRef<Path>>(
    parser: ParserConfig,
    files: &[P],
    prefs: OutputTo,
) -> Result<Vec<String>> {
    let (read_only, print, out_dir) = (prefs.read_only, prefs.print, prefs.out_dir);
    Ok(files
        .iter()
        .map(|file| {
            let file = file.as_ref();
            print!(" {}", &file.to_str().unwrap_or_default());
            let file_parser = Parser::new(parser.clone(), read_to_string(file.clone())?);
            let res = file_parser.parse()?;
            if !read_only {
                match &out_dir {
                    Some(path) => {
                        let mut path = path.clone();
                        if path.is_dir() {
                            let name = file.file_name().and_then(|x| x.to_str()).unwrap_or("out");
                            path = path.join(name)
                        }
                        if path.is_file() {
                            // Check if contents has changed
                            let contents = read_to_string(&path)?;
                            if contents == res {
                                println!(" [[No changes]]");
                                return Ok(res); // Next file
                            }
                        }
                        let mut f = File::create(&path)?;
                        f.write_all(res.as_bytes())?;
                        println!(" [[Updated!]]")
                    }
                    _ => {
                        if res != file_parser.content {
                            let mut f = File::create(&file)?;
                            f.write_all(res.as_bytes())?;
                            println!(" [[Updated!]]")
                        } else {
                            println!(" [[No changes]]");
                        }
                    }
                }
            }
            if print {
                println!("\n{}", res);
            }
            Ok(res)
        })
        .collect::<Result<Vec<_>>>()?)
}

///
/// Transform files based on the arguments in `args`.
///
/// If `recursive` is true, the `glob` will be used to find matching config files,
/// (or "**/.md-inc.toml" if not set)
/// if `files` is set, they will be transformed, otherwise the `files` field in the config file(s)
/// will be used.
/// Similarly, any fields in the config file will be overridden if also set in `args`.
///
pub fn walk_transform(mut args: Args) -> Result<Vec<Vec<String>>> {
    if let Some(x) = &args.working_dir.first() {
        std::env::set_current_dir(x)?;
    }
    let mut subdirs: Vec<PathBuf> = args.working_dir.clone();
    if args.recursive {
        args.recursive = false;
        let find_glob = |g| {
            glob::glob(g)
                .expect("Failed to read glob pattern")
                .filter_map(|path| path.ok().and_then(|x| x.parent().map(|x| x.to_path_buf())))
                .collect::<Vec<_>>()
        };
        if args.glob.is_empty() {
            subdirs.append(&mut find_glob("**/.md-inc.toml"));
        }
        for g in &args.glob {
            subdirs.append(&mut find_glob(g.as_str()));
        }
        if subdirs.is_empty() {
            return Err(anyhow::anyhow!("Did not find any matches for globs"));
        }
    }

    let config: Option<ConfigAndPath> = if let Some(path) = &args.config {
        Some(ConfigAndPath {
            config: Config::try_from_path(&path)?,
            path: path.to_path_buf(),
        })
    } else if !args.ignore_config {
        Config::try_from_dir(current_dir()?)?
    } else {
        None
    };

    let config = if let Some(x) = config {
        let parent = x
            .path
            .parent()
            .context("Directory of config file could not be determined")?;
        if subdirs.is_empty() {
            subdirs = vec![current_dir()?];
        }
        subdirs = x
            .config
            .depend_dirs
            .iter()
            .map(|x| parent.join(x))
            .chain(subdirs.into_iter())
            .chain(x.config.next_dirs.iter().map(|x| parent.join(x)))
            .collect();
        Some(x)
    } else {
        None
    };

    let res = if subdirs.is_empty() {
        let res = transform_files_with_args(args.clone(), config.clone())?;
        vec![res]
    } else {
        let mut res = vec![];
        for x in subdirs {
            println!(">> {}", x.to_str().unwrap_or_default());
            match &config {
                Some(cfg) if cfg.path == x => {
                    args.working_dir = vec![x];
                    res.push(transform_files_with_args(args.clone(), config.clone())?);
                }
                _ => {
                    if let Some(config) = Config::try_from_dir(&x)? {
                        args.working_dir = vec![x];
                        res.push(transform_files_with_args(args.clone(), Some(config))?);
                    }
                }
            }
        }
        res
    };
    Ok(res)
}
