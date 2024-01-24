//! Fancy error and terminal output.

use {
    crate::crate_prelude::*,
    std::fmt::{self, Debug},
};

#[allow(clippy::enum_variant_names)]
pub enum Error {
    /// An error while parsing TOML in the `bargo.toml` file.
    TomlError(TomlError),
    /// An error in the `build` subcommand.
    BuildError(BuildError),
    /// No command was given to Bargo.
    NoCommand,
    /// An unknown command was given to Bargo. Stores the unknown command.
    UnknownCommand(String),
    /// No `bargo.toml` file could be found in this folder nor its parents.
    NoConfig,
    /// No crates are defined in `bargo.toml`.
    NoCrates,
    /// An unknown argument was given to Bargo. Stores the unknown argument.
    UnknownArgument(String),
    /// Multiple subcommands were passed to Bargo. Stores the subcommands.
    MultipleSubcommands(String, String),
}
impl Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoCrates => write!(f, "No crates are defined in `bargo.toml`, nothing to do..."),
            Self::NoCommand => {
                write!(f, "No command provided.\n{}", help())
            }
            Self::UnknownCommand(cmd) => {
                write!(f, "Unknown command `{cmd}`.\n{}", help())
            }
            Self::TomlError(err) => write!(f, "Error while parsing `bargo.toml`: {err:?}"),
            Self::BuildError(err) => write!(f, "Error while building: {err:?}"),
            Self::NoConfig => write!(
                f,
                "Couldn't find a `bargo.toml` file in this folder nor its parent folders. Is this a Bargo workspace?"
            ),
            Self::UnknownArgument(arg) => write!(f, "Unknown argument: `{arg}`\n{}", help()),
            Self::MultipleSubcommands(sub1, sub2) => write!(f, "Multiple subcommands given: `{sub1}` and `{sub2}`")
        }
    }
}

/// Errors while parsing TOML.
pub enum TomlError {
    /// The TOML had invalid syntax. Stores the parsing error and the original TOML source.
    ParseError(BomlError, String),
    /// A TOML value was the wrong type. Stores the key name, expected type, and actual type.
    TypeMismatch(String, TomlValueType, TomlValueType),
    /// A TOML value in an array or table was the wrong type. Stores the parent's key, expected type, and actual type.
    ChildTypeMismatch(String, TomlValueType, TomlValueType),
    /// An expected TOML key was missing. Stores the key and the expected type of the key's value.
    MissingKey(String, TomlValueType),
}
impl Debug for TomlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ParseError(err, src) => write!(f, "{}", prettify_toml_error(err, src)),
            Self::TypeMismatch(key, expected_type, actual_type) => {
                write!(
                    f,
                    "Expected `{key}` to be a {expected_type:?}, but it was a {actual_type:?}."
                )
            }
            Self::ChildTypeMismatch(key, expected_type, actual_type) => {
                write!(f, "Expected children of `{key}` to be a `{expected_type:?}`, but found a `{actual_type:?}`.")
            }
            Self::MissingKey(key, ty) => {
                write!(
                    f,
                    "Couldn't find the key `{key}` (which should store a `{ty:?}`)."
                )
            }
        }
    }
}

/// Errors for the `bargo build` command.
pub enum BuildError {
    /// An unknown crate was specified. Stores where the unknown crate was specified and the crate's name.
    UnknownCrate(UnknownCrateSource, String),
    /// Two crates depend on each other with bargo's `prebuild` feature. Stores the cyclic crate's name.
    CyclicDependency(String),
    /// A post-build script failed to run.
    PostBuildFailed,
    /// A post-build script was specified, but couldn't be found. Stores the crate with a postbuild setting
    /// and the path it set.
    PostBuildNotFound(String, String),
    /// A Cargo command failed to run.
    CargoFailed,
    /// A crate in the bargo workspace had an invalid `Cargo.toml`. Stores the crate with the invalid cfg
    /// and the TOML error in the cfg.
    InvalidCargoToml(String, TomlError),
    /// A crate's name in the bargo config didn't match its name in its Cargo.toml config. Stores the
    /// name in the bargo config and the name in the Cargo.toml config.
    CrateNameMismatch(String, String),
    /// Bargo couldn't find a crate's Cargo.toml. Stores the crate's name and the path Bargo searched.
    NoCargoToml(String, String),
}
impl Debug for BuildError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PostBuildFailed => write!(f, "A post-build script failed to run, exiting..."),
            Self::CargoFailed => write!(f, "A Cargo command failed to run, exiting..."),
            Self::CyclicDependency(pkg) => {
                write!(f, "Cyclic dependency detected for crate `{pkg}`.")
            }
            Self::UnknownCrate(source, name) => {
                write!(f, "Unknown crate `{name}` specified in {source:?}.")
            }
            Self::PostBuildNotFound(pkg, path) => write!(
                f,
                "Couldn't find the post-build script for `{pkg}` at `{path}`."
            ),
            Self::InvalidCargoToml(pkg, err) => {
                write!(f, "Crate `{pkg}`'s Cargo.toml has a syntax error: {err:?}")
            }
            Self::CrateNameMismatch(bargo_name, cargo_name) => {
                write!(f, "A crate is named `{bargo_name}` in `bargo.toml`, but is named `{cargo_name}` in its `Cargo.toml`.")
            }
            Self::NoCargoToml(pkg, attempted_path) => {
                write!(f, "Bargo couldn't find the `Cargo.toml` file for `{pkg}`. It searched at `{attempted_path}`.")
            }
        }
    }
}

/// See [`BuildError::UnknownCrate`].
pub enum UnknownCrateSource {
    /// An unknown crate was specified in the `prebuild` setting of a crate. Stores the name of the crate with
    /// the unknown prebuild.
    Prebuild(String),
    /// An unknown crate was specified in the `workspace.default-build` setting.
    DefaultBuild,
    /// An unknown crate was passed as an argument to `bargo build`.
    CliArg,
}
impl Debug for UnknownCrateSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Prebuild(pkg) => write!(f, "`{pkg}`'s `prebuild` setting"),
            Self::DefaultBuild => write!(f, "the `workspace.default-build` table"),
            Self::CliArg => write!(f, "`bargo build`'s arguments"),
        }
    }
}

fn prettify_toml_error(err: &BomlError, src: &str) -> String {
    let contextual_start = if err.start > 15 { err.start - 15 } else { 0 };
    let contextual_end = if src.len() - 16 > err.end {
        err.end + 15
    } else {
        src.len() - 1
    };

    format!(
        "Syntax error:\nError type: {:?}\nThe error comes from here: `{}`\n...which is in this region of text: `{}`",
        err.kind,
        &src[err.start..=err.end],
        &src[contextual_start..=contextual_end]
    )
}

pub mod style {
    pub const _RESET: &str = "\x1B[0m";

    pub const BOLD: &str = "\x1B[1m";
    pub const UNBOLD: &str = "\x1B[22m";

    pub const GREEN: &str = "\x1B[32m";
    pub const BLUE: &str = "\x1B[36m";
    pub const WHITE: &str = "\x1B[37m";
}

pub fn help() -> String {
    use style::*;

    format!(
        "\
        {WHITE}Help: \n\
        \n\
        {BOLD}Bargo{UNBOLD} is a build system wrapped around Cargo. \n\
        \n\
        {GREEN}Commands:{WHITE} \n\
            \t{BLUE}build{WHITE}, {BLUE}b{WHITE}        If a crate is specified, compiles that \
                                                        crate in the workspace. Otherwise, compiles \
                                                        the whole workspace. \n\
            \t{BLUE}run{WHITE}, {BLUE}r{WHITE}		If a crate is specified, runs that crate in \
                                                    the workspace. Otherwise, runs the default \
                                                    runner, as specified in the `bargo.toml` file. \n\
            \t{BLUE}help{WHITE}, {BLUE}h{WHITE}		Prints this help message. \n\
        \n\
        {GREEN}Arguments:{WHITE} \n\
            \t{BLUE}--release{WHITE}, {BLUE}-r{WHITE}   Build crates in release mode. \n\
            \t{BLUE}--features{WHITE}, {BLUE}-F{WHITE}  A comma-separated list of crate features to enable. \n\
            \t{BLUE}--target{WHITE}        A comma-separated list of targets to build for. \n\
            \t{BLUE}--package{WHITE}, {BLUE}-p{WHITE}   A comma-separated list of crates to build/run. \n\
        \n\
        For more docs and info, see the GitHub repo: https://github.com/bright-shard/bargo \n\
        "
    )
}
