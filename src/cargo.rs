//! Cargo wrapper types.

use {
    crate::crate_prelude::*,
    std::{fs, path::PathBuf},
};

/// Metadata for a cargo command (eg `cargo build`).
pub struct CargoCommand<'a> {
    /// The crate this cargo command interacts with.
    pub pkg: &'a Crate<'a>,
    /// Unstable cargo features enabled in the config. These have been formatted as arguments.
    pub unstable_features: Vec<String>,
    /// Direct cargo arguments specified in the config.
    pub direct_arguments: Vec<&'a str>,
}
impl<'a> CargoCommand<'a> {
    pub fn new(pkg: &'a Crate<'a>) -> Result<Self, Error> {
        let unstable_features = if let Some(unstable) = pkg.get("unstable") {
            let Some(unstable) = unstable.table() else {
                return Err(Error::TomlError(TomlError::TypeMismatch(
                    "unstable".to_string(),
                    TomlValueType::Table,
                    unstable.value_type(),
                )));
            };

            let mut args = Vec::with_capacity(unstable.len());

            for (feature_name, feature_value) in unstable.iter() {
                match feature_value {
                    TomlValue::Boolean(enable_feature) => {
                        if *enable_feature {
                            args.push(format!("-Z{feature_name}"));
                        }
                    }
                    TomlValue::String(value) => {
                        args.push(format!("-Z{feature_name}={value}"));
                    }
                    TomlValue::Array(feature_values) => {
                        let mut arg = format!("-Z{feature_name}=");

                        for (idx, value) in feature_values.iter().enumerate() {
                            let Some(value) = value.string() else {
                                return Err(Error::TomlError(TomlError::ChildTypeMismatch(
                                    "unstable".to_string(),
                                    TomlValueType::String,
                                    value.value_type(),
                                )));
                            };

                            arg.push_str(value);
                            if idx < feature_values.len() - 1 {
                                arg.push(',');
                            }
                        }

                        args.push(arg);
                    }
                    other_value => {
                        return Err(Error::TomlError(TomlError::TypeMismatch(
                            "unstable".to_string(),
                            TomlValueType::String,
                            other_value.value_type(),
                        )));
                    }
                }
            }

            args
        } else {
            Vec::with_capacity(0)
        };

        let direct_arguments = if let Some(direct_args) = pkg.get("direct-arg") {
            match direct_args {
                TomlValue::String(arg) => {
                    vec![arg.as_str()]
                }
                TomlValue::Array(args_src) => {
                    let mut args = Vec::with_capacity(args_src.len());

                    for arg in args_src {
                        let Some(arg) = arg.string() else {
                            return Err(Error::TomlError(TomlError::ChildTypeMismatch(
                                "direct-arg".to_string(),
                                TomlValueType::String,
                                arg.value_type(),
                            )));
                        };

                        args.push(arg);
                    }

                    args
                }
                other => {
                    return Err(Error::TomlError(TomlError::TypeMismatch(
                        "direct-arg".to_string(),
                        TomlValueType::String,
                        other.value_type(),
                    )));
                }
            }
        } else {
            Vec::with_capacity(0)
        };

        Ok(Self {
            pkg,
            unstable_features,
            direct_arguments,
        })
    }

    pub fn is_unstable(&self) -> bool {
        !self.unstable_features.is_empty()
    }
}

/// Metadata for a crate in a bargo workspace.
pub struct Crate<'a> {
    /// The crate's config in the `bargo.toml` file.
    pub table: &'a TomlTable<'a>,
    /// A reference to the `workspace` table in the `bargo.toml` file.
    workspace: Option<&'a TomlTable<'a>>,
    /// The crate's name.
    pub name: &'a str,
    /// The path to the crate.
    pub path: PathBuf,
}
impl<'a> Crate<'a> {
    pub fn new(ctx: &'a Ctx<'a>, table: &'a TomlTable<'a>, name: &'a str) -> Result<Self, Error> {
        let path = if let Some(cfg_path) = table.get("path") {
            let Some(cfg_path) = cfg_path.string() else {
                return Err(Error::TomlError(TomlError::TypeMismatch(
                    "path".to_string(),
                    TomlValueType::String,
                    cfg_path.value_type(),
                )));
            };

            ctx.root.join(cfg_path)
        } else {
            ctx.root.join(name)
        };

        // Verify the crate's path
        let cargo_toml_path = path.join("Cargo.toml");
        if !cargo_toml_path.exists() {
            return Err(Error::BuildError(BuildError::NoCargoToml(
                name.to_string(),
                cargo_toml_path.to_str().unwrap().to_string(),
            )));
        }
        let cargo_toml_contents = fs::read_to_string(&cargo_toml_path).unwrap_or_else(|err| {
            panic!(
                "Failed to read file at `{}`: {err}",
                cargo_toml_path.display()
            );
        });
        let cargo_toml = Toml::parse(&cargo_toml_contents).map_err(|err| {
            Error::BuildError(BuildError::InvalidCargoToml(
                name.to_string(),
                TomlError::ParseError(err, cargo_toml_contents.clone()),
            ))
        })?;
        let Ok(package) = cargo_toml.get_table("package") else {
            return Err(Error::BuildError(BuildError::InvalidCargoToml(
                name.to_string(),
                TomlError::MissingKey("package".to_string(), TomlValueType::Table),
            )));
        };
        let Ok(name_in_cfg) = package.get_string("name") else {
            return Err(Error::BuildError(BuildError::InvalidCargoToml(
                name.to_string(),
                TomlError::MissingKey("package.name".to_string(), TomlValueType::String),
            )));
        };
        if name_in_cfg != name {
            return Err(Error::BuildError(BuildError::CrateNameMismatch(
                name.to_string(),
                name_in_cfg.to_string(),
            )));
        }

        Ok(Self {
            table,
            workspace: ctx.workspace,
            name,
            path,
        })
    }

    pub fn get(&self, key: &str) -> Option<&TomlValue> {
        self.table
            .get(key)
            .or_else(|| self.workspace.and_then(|workspace| workspace.get(key)))
    }
}
