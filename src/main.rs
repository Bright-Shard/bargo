mod args;
mod build;
mod cargo;
mod output;
mod run;

mod crate_prelude {
    pub use crate::{args::*, output::*, Ctx};
    pub use boml::prelude::{
        Toml, TomlError as BomlError, TomlGetError, TomlTable, TomlValue, TomlValueType,
    };
}

use {
    crate_prelude::*,
    std::{env, fs, path::PathBuf},
};

fn main() -> Result<(), Error> {
    // Parse arguments
    let raw_args: Vec<String> = env::args().collect();
    let raw_args = raw_args[1..].iter().map(|arg| arg.as_str()).peekable();
    let args = Args::parse(raw_args)?;

    // If it's a help command, or no command, stop here
    // If we continue, we'll try to find the cfg and error if it's not there
    // If someone's trying to see help outside a workspace we don't want to error, we want to show help
    if args.subcommand == Some(Subcommand::Help) || args.subcommand.is_none() {
        println!("{}", output::help());
        return Ok(());
    }

    // Find the `bargo.toml` by searching this folder and all its parents, then parse the toml
    let mut cfg_path = None;
    for dir in env::current_dir().unwrap().ancestors() {
        let test = dir.join("bargo.toml");
        if test.exists() {
            cfg_path = Some(test);
        }
    }
    let Some(cfg_path) = cfg_path else {
        return Err(Error::NoConfig);
    };
    let cfg_source = fs::read_to_string(&cfg_path).expect("Failed to read `bargo.toml`");
    let cfg = &Toml::parse(&cfg_source).map_err(|err| {
        Error::TomlError(output::TomlError::ParseError(err, cfg_source.to_string()))
    })?;

    // Get the workspace and crates tables
    let workspace = match cfg.get_table("workspace") {
        Ok(table) => Some(table),
        Err(err) => match err {
            TomlGetError::InvalidKey => None,
            TomlGetError::TypeMismatch(_, ty) => {
                return Err(Error::TomlError(TomlError::TypeMismatch(
                    "workspace".to_string(),
                    TomlValueType::Table,
                    ty,
                )));
            }
        },
    };
    let crates = match cfg.get_table("crates") {
        Ok(crates) => crates,

        Err(err) => match err {
            TomlGetError::InvalidKey => return Err(Error::NoCrates),
            TomlGetError::TypeMismatch(_, ty) => {
                return Err(Error::TomlError(TomlError::TypeMismatch(
                    "crates".to_string(),
                    TomlValueType::Table,
                    ty,
                )));
            }
        },
    };
    if crates.is_empty() {
        return Err(Error::NoCrates);
    }

    let root = cfg_path.parent().unwrap().to_path_buf();
    let target_dir = root.join("target");
    // if !target_dir.exists() {
    //     fs::create_dir(&target_dir).expect("Error: Failed to create `target` directory");
    // }

    let ctx = Ctx {
        args,
        root,
        target_dir,
        cfg,
        workspace,
        crates,
    };

    match ctx.args.subcommand {
        Some(Subcommand::Build) => build::build(&ctx)?,
        Some(Subcommand::Run) => todo!(),
        _ => unreachable!(),
    }

    Ok(())
}

pub struct Ctx<'a> {
    /// Arguments provided to bargo.
    pub args: Args<'a>,
    /// The root of the bargo workspace - the folder with the `bargo.toml` file.
    pub root: PathBuf,
    /// The path to the target directory.
    pub target_dir: PathBuf,
    /// The parsed contents of the `bargo.toml` file.
    pub cfg: &'a Toml<'a>,
    /// The workspace table.
    pub workspace: Option<&'a TomlTable<'a>>,
    /// The crates table.
    pub crates: &'a TomlTable<'a>,
}
