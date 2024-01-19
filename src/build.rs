use {
    crate::{cargo::*, output, Ctx},
    boml::prelude::*,
    std::{collections::HashSet, process::Command, process::ExitStatus},
};

struct BuildCtx<'a> {
    pub ctx: &'a Ctx<'a>,
    /// The location of the `target` (output) directory.
    pub target_dir: &'a str,
    /// Crates that bargo has built.
    pub build_list: HashSet<String>,
    /// Crates that bargo has encountered.
    pub encounter_list: HashSet<String>,
}

pub fn build(ctx: &Ctx) {
    if ctx.crates.is_empty() {
        println!("No crates defined in `bargo.toml`, nothing to do...");
        return;
    }

    let target_dir = ctx.root.join("target");
    let target_dir = target_dir.as_os_str().to_str().unwrap();
    let mut build_ctx = BuildCtx {
        ctx,
        target_dir,
        build_list: HashSet::new(),
        encounter_list: HashSet::new(),
    };

    if !ctx.args.is_empty() {
        for crate_name in ctx.args {
            let Ok(crate_cfg) = ctx.crates.get_table(crate_name) else {
                panic!("Error: Unknown crate passed to `bargo build`: {crate_name}");
            };
            build_crate(&mut build_ctx, Crate::new(ctx, crate_cfg, crate_name));
        }
    } else if let Some(defaults) = ctx
        .workspace
        .and_then(|workspace| workspace.get("default-build"))
    {
        match defaults {
            TomlValue::Array(ref default_crates) => {
                for crate_name in default_crates {
                    let Some(crate_name) = crate_name.string() else {
                        panic!("Error: `workspace.default-build` can only store strings (for crate names)");
                    };
                    let Ok(crate_cfg) = ctx.crates.get_table(crate_name) else {
                        panic!("Error: Unknown crate in `workspace.default-build`: {crate_name}");
                    };

                    build_crate(&mut build_ctx, Crate::new(ctx, crate_cfg, crate_name));
                }
            }
            TomlValue::String(ref crate_name) => {
                let crate_name = crate_name.as_str();
                let Ok(crate_cfg) = ctx.crates.get_table(crate_name) else {
                    panic!("Error: Unknown crate in `workspace.default-build`: {crate_name}");
                };

                build_crate(&mut build_ctx, Crate::new(ctx, crate_cfg, crate_name));
            }
            _ => output::toml_type_mismatch(
                "workspace.default-build",
                TomlValueType::Array,
                defaults.value_type(),
            ),
        }
    } else {
        for (crate_name, crate_cfg) in ctx.crates.iter() {
            let Some(crate_cfg) = crate_cfg.table() else {
                panic!("Error: All crates defined in `bargo.toml` must be tables (entry for `{crate_name}` is not)");
            };

            build_crate(&mut build_ctx, Crate::new(ctx, crate_cfg, crate_name));
        }
    }
}

fn build_crate(build_ctx: &mut BuildCtx, pkg: Crate) {
    let inserted = build_ctx.encounter_list.insert(pkg.name.to_string());
    if !inserted {
        panic!(
            "Cyclical prebuild dependency encountered for crate `{}`.",
            pkg.name
        );
    }

    build_prebuilds(build_ctx, &pkg);

    let cargo_cmd = CargoCommand::new(&pkg);

    let mut args = Vec::with_capacity(cargo_cmd.num_arguments());

    if cargo_cmd.is_unstable() {
        args.push("+nightly");
    }
    args.push("build");
    args.push("--target-dir");
    args.push(build_ctx.target_dir);
    for unstable_feature in cargo_cmd.unstable_features.iter() {
        args.push(unstable_feature)
    }
    for direct_arg in cargo_cmd.direct_arguments {
        args.push(direct_arg)
    }

    let build_succeeded = match pkg.get("target") {
        Some(TomlValue::Array(targets)) => {
            let mut status = Ok(ExitStatus::default());
            args.push("--target");

            for target in targets {
                let Some(target) = target.string() else {
                    output::toml_child_type_mismatch(
                        "target",
                        TomlValueType::String,
                        target.value_type(),
                    )
                };

                let mut cargo = Command::new("cargo");
                cargo.current_dir(&pkg.path).args(&args);

                if target.contains(".json") {
                    let path = build_ctx.ctx.root.join(target);
                    cargo.arg(path.as_os_str().to_str().unwrap());
                } else {
                    cargo.arg(target);
                }

                status = cargo.status();
            }

            status
        }
        Some(TomlValue::String(target)) => {
            let target = target.as_str();
            let mut cargo = Command::new("cargo");
            cargo.current_dir(&pkg.path).args(args).arg("--target");

            if target.contains(".json") {
                let path = build_ctx.ctx.root.join(target);
                cargo.arg(path.as_os_str().to_str().unwrap());
            } else {
                cargo.arg(target);
            }

            cargo.status()
        }
        Some(other_val) => {
            output::toml_type_mismatch("target", TomlValueType::String, other_val.value_type())
        }
        None => Command::new("cargo")
            .current_dir(&pkg.path)
            .args(args)
            .status(),
    };

    if build_succeeded.is_err() || !build_succeeded.unwrap().success() {
        panic!("A cargo command failed, exiting...");
    }

    run_postbuild(build_ctx, &pkg);
    build_ctx.build_list.insert(pkg.name.to_string());
}

fn build_prebuilds(build_ctx: &mut BuildCtx, pkg: &Crate) {
    if let Some(prebuild) = pkg.get("prebuild") {
        match prebuild {
            TomlValue::Array(prebuild_queue) => {
                for prebuild in prebuild_queue {
                    let Some(prebuild) = prebuild.string() else {
                        output::toml_child_type_mismatch(
                            "prebuild",
                            TomlValueType::String,
                            prebuild.value_type(),
                        );
                    };

                    if !build_ctx.build_list.contains(prebuild) {
                        let Ok(crate_cfg) = build_ctx.ctx.crates.get_table(prebuild) else {
                            output::unknown_prebuild(pkg.name, prebuild);
                        };

                        build_crate(build_ctx, Crate::new(build_ctx.ctx, crate_cfg, prebuild));
                    }
                }
            }
            TomlValue::String(prebuild) => {
                let prebuild = prebuild.as_str();
                if !build_ctx.build_list.contains(prebuild) {
                    let Ok(crate_cfg) = build_ctx.ctx.crates.get_table(prebuild) else {
                        output::unknown_prebuild(pkg.name, prebuild);
                    };

                    build_crate(build_ctx, Crate::new(build_ctx.ctx, crate_cfg, prebuild));
                }
            }
            _ => {
                let ty = prebuild.value_type();
                output::toml_type_mismatch("prebuild", TomlValueType::String, ty)
            }
        }
    }
}

fn run_postbuild(build_ctx: &mut BuildCtx, pkg: &Crate) {
    let (postbuild_path, custom) = pkg
        .get("postbuild")
        .map(|val| {
            let Some(val) = val.string() else {
                output::toml_type_mismatch("postbuild", TomlValueType::String, val.value_type())
            };
            (build_ctx.ctx.root.join(val), true)
        })
        .unwrap_or((pkg.path.join("postbuild.rs"), false));

    if postbuild_path.exists() {
        let script_status = Command::new("cargo")
            .env("BARGO_ROOT", build_ctx.ctx.root.to_str().unwrap())
            .current_dir(&build_ctx.ctx.root)
            .arg("+nightly")
            .arg("-Zscript")
            .arg(postbuild_path.as_os_str())
            .status();

        if script_status.is_err() || !script_status.unwrap().success() {
            panic!("Post-build script failed to run, stopping...");
        }
    } else if custom {
        panic!(
            "Error: The config for `{}` specified a postbuild script at `{}`, but that file doesn't exist.",
            pkg.name,
            postbuild_path.display()
        );
    }
}
