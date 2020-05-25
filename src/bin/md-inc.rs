use anyhow::Result;
use md_inc;
use structopt::StructOpt;

fn main() -> Result<()> {
    let args: md_inc::Args = md_inc::Args::from_args();
    let _ = md_inc::walk_transform(args)?;
    Ok(())
}
