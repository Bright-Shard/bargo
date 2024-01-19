use {
    boml::{prelude::*, table::Table as TomlTable},
    std::{env, fs, path::PathBuf},
};

mod build;
mod cargo;
mod output;
mod run;

fn main() {
    let args: Vec<String> = env::args().collect();
    let args: Vec<&str> = args.iter().map(|arg| arg.as_str()).collect();

    let Some(subcommand) = args.get(1) else {
        println!("Error: No command provided.");
        output::help();
        return;
    };

    match *subcommand {
        "r" | "run" => todo!(),
        "b" | "build" => {
            let (root, cfg_source) = get_bargo_cfg();
            let toml = get_cfg_toml(&cfg_source);
            let ctx = Ctx::new(root, &toml, &args[2..]);
            build::build(&ctx)
        }
        "h" | "help" => output::help(),
        _ => {
            println!("Error: Unknown command.");
            output::help();
        }
    }
}

fn get_bargo_cfg() -> (PathBuf, String) {
    let cwd = env::current_dir().unwrap();
    let mut cfg_path = None;

    for dir in cwd.ancestors() {
        let test = dir.join("bargo.toml");
        if test.exists() {
            cfg_path = Some(test);
        }
    }
    let Some(cfg_path) = cfg_path else {
        panic!("Failed to find `bargo.toml` file. Is this a bargo workspace?");
    };

    let cfg_source = fs::read_to_string(&cfg_path).expect("Failed to read `bargo.toml`");
    let root = cfg_path.parent().unwrap().to_path_buf();

    (root, cfg_source)
}

fn get_cfg_toml(cfg_source: &str) -> Toml<'_> {
    match Toml::parse(cfg_source) {
        Ok(toml) => toml,
        Err(err) => output::bargo_toml_syntax_error(cfg_source, err),
    }
}

pub struct Ctx<'a> {
    /// Arguments provided to bargo.
    pub args: &'a [&'a str],
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
impl<'a> Ctx<'a> {
    pub fn new(root: PathBuf, cfg: &'a Toml<'a>, args: &'a [&'a str]) -> Self {
        let workspace = match cfg.get_table("workspace") {
            Ok(table) => Some(table),
            Err(err) => match err {
                TomlGetError::InvalidKey => None,
                TomlGetError::TypeMismatch(_, ty) => {
                    output::toml_type_mismatch("workspace", TomlValueType::Table, ty)
                }
            },
        };

        let crates = match cfg.get_table("crates") {
            Ok(crates) => crates,

            Err(err) => match err {
                TomlGetError::InvalidKey => panic!("No crates in workspace, exiting..."),
                TomlGetError::TypeMismatch(_, ty) => {
                    output::toml_type_mismatch("crates", TomlValueType::Table, ty)
                }
            },
        };

        let target_dir = root.join("target");
        if !target_dir.exists() {
            fs::create_dir(&target_dir).expect("Error: Failed to create `target` directory");
        }

        Self {
            args,
            root,
            target_dir,
            cfg,
            workspace,
            crates,
        }
    }
}
