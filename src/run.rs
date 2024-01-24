use {
	crate::{build::BuildCtx, cargo::*, crate_prelude::*},
	std::{collections::HashSet, process::Command},
};

pub fn run(ctx: &Ctx) -> Result<(), Error> {
	let to_run = if !ctx.args.crates.is_empty() {
		if ctx.args.crates.len() > 1 {
			return Err(Error::RunError(RunError::TooManyCrates));
		}
		ctx.args.crates.first().unwrap()
	} else if let Some(default) = ctx
		.workspace
		.and_then(|workspace| workspace.get("default-run"))
	{
		let Some(default) = default.string() else {
			return Err(Error::TomlError(TomlError::TypeMismatch(
				"default-run".to_string(),
				TomlValueType::String,
				default.value_type(),
			)));
		};
		default
	} else {
		return Err(Error::RunError(RunError::NoCrateToRun));
	};
	let to_run_cfg = match ctx.crates.get_table(to_run) {
		Ok(table) => table,
		Err(err) => match err {
			TomlGetError::InvalidKey => {
				return Err(Error::RunError(RunError::UnknownCrate(to_run.to_string())))
			}
			TomlGetError::TypeMismatch(_, ty) => {
				return Err(Error::TomlError(TomlError::ChildTypeMismatch(
					String::from("crates"),
					TomlValueType::Table,
					ty,
				)))
			}
		},
	};

	let to_run = Crate::new(ctx, to_run_cfg, to_run)?;

	let mut build_ctx = BuildCtx {
		ctx,
		build_list: HashSet::new(),
		build_queue: HashSet::new(),
	};

	crate::build::build_crate(&mut build_ctx, &to_run, None)?;

	let mut cargo = Command::new("cargo");
	cargo.current_dir(&to_run.path).arg("run");
	if ctx.args.release {
		cargo.arg("--release");
	}
	if !ctx.args.features.is_empty() {
		cargo.arg("--features");
		for feature in ctx.args.features.iter() {
			cargo.arg(feature);
		}
	}
	let status = cargo.status();

	if status.is_err() | !status.unwrap().success() {
		Err(Error::RunError(RunError::CargoError))
	} else {
		Ok(())
	}
}
