#[macro_use]
extern crate serde_derive;
use anyhow::{Context, Result};
use std::env::current_dir;
use std::fs::{read_to_string, File};
use std::io::Write;
use std::path::{Path, PathBuf};
pub use structopt::StructOpt;
mod parse;
mod config;
pub use crate::{config::{OutputTo, Config}, parse::{ParserConfig}};
use crate::parse::Parser;

#[cfg(test)]
mod test;

///
/// Include files in Markdown docs
/// Can be generated from command-line arguments using `Args::from_args()` (uses the `StructOpt` trait)
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
    /// Override the working directory. Used to locate the '.md-inc.toml' file
    ///
    #[structopt(
        short = "d",
        long = "dir",
        parse(from_os_str),
        help = "Working directory"
    )]
    working_dir: Option<PathBuf>,

    ///
    /// Ignore automatic detection of '.md-inc.toml' files in the working directory
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
        help = "A custom glob used to match config files"
    )]
    glob: Option<String>,

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
/// A Result containing the transformed input
///
/// # Parameters
/// * `args` An struct of configuration settings.
///
pub fn transform_files_with_args(args: Args) -> Result<Vec<String>> {
    if let Some(x) = args.working_dir {
        if x.exists() {
            std::env::set_current_dir(&x)
                .with_context(|| format!("Could not set working directory: {:?}", &x))?;
        }
    }
    let (mut parser, files) = if let Some(config) = args.config {
        println!("Found config");
        let f = read_to_string(&config).with_context(|| format!("Reading {:?}", &config))?;
        toml::from_str::<Config>(&f)
            .with_context(|| format!("Error reading toml config file: '{}'", &f))
            // Set the base directory to the one containing the config file
            .map(|x| -> Result<_> {
                let dir = config
                    .parent()
                    .context("No parent directory")?
                    .to_path_buf();
                Ok(x.into_parser(&dir))
            })??
    } else if args.ignore_config {
        (ParserConfig::default(), args.files)
    } else {
        // Find '.md-inc.toml' in the input file directory
        let dir = current_dir()?;
        match config::find_config(dir)? {
            Some((parser, mut files)) => {
                if !args.files.is_empty() {
                    files = args.files
                }
                (parser, files)
            }
            _ => (ParserConfig::default(), args.files),
        }
    };

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
    let (read_only, print) = (prefs.read_only, prefs.print);
    Ok(files
        .iter()
        .map(|file| {
            let file = file.as_ref();
            let file_parser = Parser::new(parser.clone(), read_to_string(file.clone())?);
            let res = file_parser.parse()?;
            if !read_only {
                if res != file_parser.content {
                    let mut f = File::create(file.clone())?;
                    f.write_all(res.as_bytes())?;
                    println!("Updated {}", file.to_str().unwrap())
                } else {
                    println!("No changes for {}", file.to_str().unwrap());
                }
            }
            if print {
                println!("{}", res);
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
    if let Some(x) = &args.working_dir {
        std::env::set_current_dir(x)?;
    }
    let subdirs: Vec<PathBuf> = if args.recursive {
        args.recursive = false;
        glob::glob(args.glob.as_ref().map(|x| x.as_str()).unwrap_or("**/.md-inc.toml"))
            .expect("Failed to read glob pattern")
            .filter_map(|path| path.ok().and_then(|x| x.parent().map(|x| x.to_path_buf())))
            .collect()
    } else {
        vec![]
    };
    if subdirs.is_empty() {
        Ok(vec![transform_files_with_args(args)?])
    } else {
        subdirs
            .into_iter()
            .map(|x| {
                args.working_dir = Some(x);
                transform_files_with_args(args.clone())
            })
            .collect::<Result<Vec<_>>>()
    }
}
