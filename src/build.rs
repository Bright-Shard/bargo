use {
    crate::{cargo::*, crate_prelude::*},
    std::{collections::HashSet, path::Path, process::Command},
};

struct BuildCtx<'a> {
    pub ctx: &'a Ctx<'a>,
    /// The location of the `target` (output) directory.
    pub target_dir: &'a str,
    /// Crates that bargo has built.
    pub build_list: HashSet<String>,
    /// Crates that are queued to be built. Used to detect cyclic dependencies.
    pub build_queue: HashSet<String>,
}

pub fn build(ctx: &Ctx) -> Result<(), Error> {
    let target_dir = ctx.root.join("target");
    let target_dir = target_dir.as_os_str().to_str().unwrap();
    let mut build_ctx = BuildCtx {
        ctx,
        target_dir,
        build_list: HashSet::new(),
        build_queue: HashSet::new(),
    };

    if !ctx.args.crates.is_empty() {
        for crate_name in ctx.args.crates.iter() {
            let Ok(crate_cfg) = ctx.crates.get_table(crate_name) else {
                return Err(Error::BuildError(BuildError::UnknownCrate(
                    UnknownCrateSource::CliArg,
                    crate_name.to_string(),
                )));
            };
            build_crate(
                &mut build_ctx,
                Crate::new(ctx, crate_cfg, crate_name)?,
                None,
            )?;
        }
    } else if let Some(defaults) = ctx
        .workspace
        .and_then(|workspace| workspace.get("default-build"))
    {
        match defaults {
            TomlValue::Array(ref default_crates) => {
                for crate_name in default_crates {
                    let Some(crate_name) = crate_name.string() else {
                        return Err(Error::TomlError(TomlError::ChildTypeMismatch(
                            String::from("workspace.default-build"),
                            TomlValueType::String,
                            crate_name.value_type(),
                        )));
                    };
                    let Ok(crate_cfg) = ctx.crates.get_table(crate_name) else {
                        return Err(Error::BuildError(BuildError::UnknownCrate(
                            UnknownCrateSource::DefaultBuild,
                            crate_name.to_string(),
                        )));
                    };

                    build_crate(
                        &mut build_ctx,
                        Crate::new(ctx, crate_cfg, crate_name)?,
                        None,
                    )?;
                }
            }
            TomlValue::String(ref crate_name) => {
                let crate_name = crate_name.as_str();
                let Ok(crate_cfg) = ctx.crates.get_table(crate_name) else {
                    return Err(Error::BuildError(BuildError::UnknownCrate(
                        UnknownCrateSource::DefaultBuild,
                        crate_name.to_string(),
                    )));
                };

                build_crate(
                    &mut build_ctx,
                    Crate::new(ctx, crate_cfg, crate_name)?,
                    None,
                )?;
            }
            _ => {
                return Err(Error::TomlError(TomlError::TypeMismatch(
                    String::from("workspace.default-build"),
                    TomlValueType::Array,
                    defaults.value_type(),
                )));
            }
        }
    } else {
        for (crate_name, crate_cfg) in ctx.crates.iter() {
            let Some(crate_cfg) = crate_cfg.table() else {
                return Err(Error::TomlError(TomlError::ChildTypeMismatch(
                    "crates".to_string(),
                    TomlValueType::Table,
                    crate_cfg.value_type(),
                )));
            };

            build_crate(
                &mut build_ctx,
                Crate::new(ctx, crate_cfg, crate_name)?,
                None,
            )?;
        }
    }

    Ok(())
}

fn build_crate(
    build_ctx: &mut BuildCtx,
    pkg: Crate,
    prebuild_metadata: Option<PrebuildMetadata>,
) -> Result<(), Error> {
    let inserted = build_ctx.build_queue.insert(pkg.name.to_string());
    if !inserted {
        return Err(Error::BuildError(BuildError::CyclicDependency(
            pkg.name.to_string(),
        )));
    }

    build_prebuilds(build_ctx, &pkg, pkg.name)?;

    let cargo_cmd = CargoCommand::new(&pkg)?;
    let mut args = Vec::new();

    if cargo_cmd.is_unstable() {
        args.push("+nightly");
    }
    args.push("build");
    args.push("--target-dir");
    args.push(build_ctx.target_dir);
    args.extend(cargo_cmd.direct_arguments.iter());
    args.extend(cargo_cmd.unstable_features.iter().map(|val| val.as_str()));
    if build_ctx.ctx.args.release {
        args.push("--release");
    }

    if prebuild_metadata.is_some() && !prebuild_metadata.as_ref().unwrap().features.is_empty() {
        args.push("--features");
        for feature in prebuild_metadata.as_ref().unwrap().features.iter() {
            args.push(feature);
        }
    } else if !build_ctx.ctx.args.features.is_empty() {
        args.push("--features");
        for feature in build_ctx.ctx.args.features.iter() {
            args.push(feature);
        }
    }

    if prebuild_metadata.is_some() && !prebuild_metadata.as_ref().unwrap().targets.is_empty() {
        for target in prebuild_metadata.as_ref().unwrap().targets.iter() {
            spawn_cargo_with_target(&pkg.path, &args, target, build_ctx)?;
        }
    } else if !build_ctx.ctx.args.targets.is_empty() {
        for target in build_ctx.ctx.args.targets.iter() {
            spawn_cargo_with_target(&pkg.path, &args, target, build_ctx)?;
        }
    } else if let Some(val) = pkg.get("target") {
        match val {
            TomlValue::Array(targets) => {
                for target in targets {
                    let Some(target) = target.string() else {
                        return Err(Error::TomlError(TomlError::ChildTypeMismatch(
                            String::from("target"),
                            TomlValueType::String,
                            target.value_type(),
                        )));
                    };
                    spawn_cargo_with_target(&pkg.path, &args, target, build_ctx)?;
                }
            }
            TomlValue::String(target) => {
                spawn_cargo_with_target(&pkg.path, &args, target.as_str(), build_ctx)?;
            }
            other_val => {
                return Err(Error::TomlError(TomlError::ChildTypeMismatch(
                    String::from("target"),
                    TomlValueType::String,
                    other_val.value_type(),
                )));
            }
        }
    } else {
        let status = Command::new("cargo")
            .current_dir(&pkg.path)
            .args(args)
            .status();

        if status.is_err() || !status.unwrap().success() {
            return Err(Error::BuildError(BuildError::CargoFailed));
        }
    }

    run_postbuild(build_ctx, &pkg, build_ctx.ctx.args.release)?;
    build_ctx.build_queue.remove(pkg.name);
    build_ctx.build_list.insert(pkg.name.to_string());

    Ok(())
}

struct PrebuildMetadata<'a> {
    pub features: Vec<&'a str>,
    pub targets: Vec<&'a str>,
}

fn build_prebuilds(
    build_ctx: &mut BuildCtx,
    pkg: &Crate,
    dependent_name: &str,
) -> Result<(), Error> {
    if let Some(prebuilds) = pkg.get("prebuild") {
        let Some(prebuilds) = prebuilds.table() else {
            return Err(Error::TomlError(TomlError::TypeMismatch(
                String::from("prebuild"),
                TomlValueType::Table,
                prebuilds.value_type(),
            )));
        };

        for (pkg, overrides) in prebuilds.iter() {
            let Ok(cfg) = build_ctx.ctx.crates.get_table(pkg) else {
                return Err(Error::BuildError(BuildError::UnknownCrate(
                    UnknownCrateSource::Prebuild(pkg.to_string()),
                    dependent_name.to_string(),
                )));
            };
            let Some(overrides) = overrides.table() else {
                return Err(Error::TomlError(TomlError::ChildTypeMismatch(
                    String::from("prebuild"),
                    TomlValueType::Table,
                    overrides.value_type(),
                )));
            };

            let features = match overrides.get("features") {
                Some(val) => match val {
                    TomlValue::Array(vals) => {
                        let mut neovals = Vec::with_capacity(vals.len());

                        for val in vals {
                            let Some(val) = val.string() else {
                                return Err(Error::TomlError(TomlError::ChildTypeMismatch(
                                    String::from("prebuild.features"),
                                    TomlValueType::Table,
                                    val.value_type(),
                                )));
                            };
                            neovals.push(val)
                        }

                        neovals
                    }
                    TomlValue::String(val) => vec![val.as_str()],
                    _ => {
                        return Err(Error::TomlError(TomlError::TypeMismatch(
                            String::from("prebuild.features"),
                            TomlValueType::String,
                            val.value_type(),
                        )))
                    }
                },
                None => Vec::new(),
            };
            let targets = match overrides.get("targets") {
                Some(val) => match val {
                    TomlValue::Array(vals) => {
                        let mut neovals = Vec::with_capacity(vals.len());

                        for val in vals {
                            let Some(val) = val.string() else {
                                return Err(Error::TomlError(TomlError::ChildTypeMismatch(
                                    String::from("prebuild.targets"),
                                    TomlValueType::Table,
                                    val.value_type(),
                                )));
                            };
                            neovals.push(val)
                        }

                        neovals
                    }
                    TomlValue::String(val) => vec![val.as_str()],
                    _ => {
                        return Err(Error::TomlError(TomlError::TypeMismatch(
                            String::from("prebuild.targets"),
                            TomlValueType::String,
                            val.value_type(),
                        )))
                    }
                },
                None => Vec::new(),
            };
            let metadata = PrebuildMetadata { features, targets };

            build_crate(
                build_ctx,
                Crate::new(build_ctx.ctx, cfg, pkg)?,
                Some(metadata),
            )?;
        }
    }
    // let prebuilds = pkg.get("prebuild").and_then(|val| val.table())
    // let prebuilds = if let Some(prebuild) = pkg.get("prebuild") {
    //     match prebuild {
    //         TomlValue::Array(prebuilds) => {
    //             let mut new_prebuilds = Vec::with_capacity(prebuilds.len());

    //             for prebuild in prebuilds {
    //                 let Some(prebuild) = prebuild.string() else {
    //                     return Err(Error::TomlError(TomlError::ChildTypeMismatch(
    //                         String::from("prebuild"),
    //                         TomlValueType::String,
    //                         prebuild.value_type(),
    //                     )));
    //                 };
    //                 new_prebuilds.push(prebuild);
    //             }

    //             new_prebuilds
    //         }
    //         TomlValue::String(prebuild) => {
    //             let prebuild = prebuild.as_str();
    //             vec![prebuild]
    //         }
    //         _ => {
    //             return Err(Error::TomlError(TomlError::TypeMismatch(
    //                 "prebuild".to_string(),
    //                 TomlValueType::String,
    //                 prebuild.value_type(),
    //             )));
    //         }
    //     }
    // } else {
    //     Vec::with_capacity(0)
    // };

    // for prebuild in prebuilds {
    //     if !build_ctx.build_list.contains(prebuild) {
    //         let Ok(crate_cfg) = build_ctx.ctx.crates.get_table(prebuild) else {
    //             return Err(Error::BuildError(BuildError::UnknownCrate(
    //                 UnknownCrateSource::Prebuild(pkg.name.to_string()),
    //                 prebuild.to_string(),
    //             )));
    //         };

    //         build_crate(
    //             build_ctx,
    //             Crate::new(build_ctx.ctx, crate_cfg, prebuild)?,
    //             true,
    //         )?;
    //     }
    // }

    Ok(())
}

fn spawn_cargo_with_target(
    path: &Path,
    args: &[&str],
    target: &str,
    build_ctx: &BuildCtx,
) -> Result<(), Error> {
    let mut cargo = Command::new("cargo");
    cargo.current_dir(path).args(args).arg("--target");

    if target.contains(".json") {
        let path = build_ctx.ctx.root.join(target);
        cargo.arg(path.as_os_str().to_str().unwrap());
    } else {
        cargo.arg(target);
    }

    let status = cargo.status();
    if status.is_err() || !status.unwrap().success() {
        Err(Error::BuildError(BuildError::CargoFailed))
    } else {
        Ok(())
    }
}

fn run_postbuild(build_ctx: &mut BuildCtx, pkg: &Crate, release: bool) -> Result<(), Error> {
    let (postbuild_path, custom) = pkg
        .get("postbuild")
        .map(|val| {
            let Some(val) = val.string() else {
                return Err(Error::TomlError(TomlError::TypeMismatch(
                    "postbuild".to_string(),
                    TomlValueType::String,
                    val.value_type(),
                )));
            };
            Ok((build_ctx.ctx.root.join(val), true))
        })
        .unwrap_or(Ok((pkg.path.join("postbuild.rs"), false)))?;

    if postbuild_path.exists() {
        let mut script = Command::new("cargo");
        script
            .env("BARGO_ROOT", build_ctx.ctx.root.to_str().unwrap())
            .current_dir(&build_ctx.ctx.root)
            .arg("+nightly")
            .arg("-Zscript")
            .arg(postbuild_path.as_os_str());

        if release {
            script.env("PROFILE", "release");
        } else {
            script.env("PROFILE", "debug");
        }

        let script_status = script.status();

        if script_status.is_err() || !script_status.unwrap().success() {
            return Err(Error::BuildError(BuildError::PostBuildFailed));
        }
    } else if custom {
        return Err(Error::BuildError(BuildError::PostBuildNotFound(
            pkg.name.to_string(),
            postbuild_path.to_str().unwrap().to_string(),
        )));
    }

    Ok(())
}
