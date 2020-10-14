use anyhow::Context;
use std::path::PathBuf;
use std::str::FromStr;
use structopt::StructOpt;

mod build;
mod init;

pub struct Size(pub i32, pub i32);

impl FromStr for Size {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> anyhow::Result<Self> {
        let mut parts = s.split('x');
        let w = parts
            .next()
            .unwrap()
            .parse::<i32>()
            .context("couldn't parse width")?;
        let h = parts
            .next()
            .context("couldn't find height, expected WxH")?
            .parse::<i32>()
            .context("couldn't parse height")?;
        anyhow::ensure!(
            parts.next().is_none(),
            "unexpected 3rd part of size argument"
        );
        Ok(Size(w, h))
    }
}

#[derive(StructOpt)]
struct SizeArg {
    /// The size of the window in WxH format.
    #[structopt(short, long, value_name = "SIZE", default_value = "800x600")]
    size: Size,
}

#[derive(StructOpt)]
#[structopt(name = "pickitup")]
enum Pyckitup {
    Run {
        #[structopt(default_value = "run.py", value_name = "FNAME", parse(from_os_str))]
        filename: PathBuf,
        #[structopt(flatten)]
        size: SizeArg,
    },
    /// Initialize a new pyckitup project
    Init {
        /// The name of the project
        #[structopt(parse(from_os_str))]
        project: PathBuf,
    },
    /// Build for web
    Build {
        #[structopt(default_value = "run.py", value_name = "FNAME", parse(from_os_str))]
        filename: PathBuf,
        #[structopt(flatten)]
        size: SizeArg,
    },
}

fn main() -> anyhow::Result<()> {
    let opts = Pyckitup::from_args();
    match opts {
        Pyckitup::Run { filename, size } => {
            if !filename.exists() {
                println!("File `./run.py` doesn't exist. Doing nothing.");
                std::process::exit(1);
            }

            let Size(w, h) = size.size;

            pyckitup_core::FNAME.set(&filename.into(), || {
                pyckitup_core::run(w, h);
            });
        }
        Pyckitup::Init { project } => init::pyckitup_init(project)?,
        Pyckitup::Build { filename, size } => build::pyckitup_build(filename, size.size)?,
    }
    Ok(())
}
