//! Fancy error and terminal output.

use boml::prelude::*;

const _RESET: &str = "\x1B[0m";

const BOLD: &str = "\x1B[1m";
const UNBOLD: &str = "\x1B[22m";

const GREEN: &str = "\x1B[32m";
const BLUE: &str = "\x1B[36m";
const WHITE: &str = "\x1B[37m";

pub fn help() {
    println!(
        "\
		{WHITE}Help:\n\
		\n\
		{BOLD}Bargo{UNBOLD} is a build system wrapped around Cargo.\n\
		\n\
		{GREEN}Commands:{WHITE}\n\
			\t{BLUE}build{WHITE}, {BLUE}b{WHITE}	If a crate is specified, compiles that \
													crate in the workspace. Otherwise, compiles \
													the whole workspace. \n\
			\t{BLUE}run{WHITE}, {BLUE}r{WHITE}		If a crate is specified, runs that crate in \
													the workspace. Otherwise, runs the default \
													runner, as specified in the `bargo.toml` file. \n\
			\t{BLUE}help{WHITE}, {BLUE}h{WHITE}		Prints this help message. \n\
		\n\
		For more docs and info, see the GitHub repo: https://github.com/bright-shard/bargo\n\
		"
    )
}

pub fn prettify_toml_syntax_error(toml_source: &str, err: TomlError) -> String {
    let contextual_start = if err.start > 15 { err.start - 15 } else { 0 };
    let contextual_end = if toml_source.len() - 16 > err.end {
        err.end + 15
    } else {
        toml_source.len() - 1
    };

    format!(
		"Syntax error: {:?}\nThe error comes from here: `{}`\n...which is in this region of text: `{}`",
		err.kind,
    	&toml_source[err.start..=err.end],
		&toml_source[contextual_start..=contextual_end]
	)
}

pub fn bargo_toml_syntax_error(toml_source: &str, err: TomlError) -> ! {
    panic!(
        "Error while parsing `bargo.toml`: {}",
        prettify_toml_syntax_error(toml_source, err)
    );
}

pub fn toml_type_mismatch(
    key: &str,
    expected_type: TomlValueType,
    actual_type: TomlValueType,
) -> ! {
    panic!(
        "\
		Error while parsing `bargo.toml`:\n\
		Invalid value type for `{key}`. Expected a(n) {expected_type:?}, but found a(n) {actual_type:?}.\
		"
    );
}

pub fn toml_child_type_mismatch(
    parent: &str,
    expected_type: TomlValueType,
    actual_type: TomlValueType,
) -> ! {
    panic!(
        "\
		Error while parsing `bargo.toml`:\n\
		All members of `{parent}` must be a(n) {expected_type:?}, but found a(n) {actual_type:?}.\
		"
    )
}

pub fn invalid_cargo_toml(crate_name: &str, note: &str) -> ! {
    panic!(
        "\
		Error: Failed to read the `Cargo.toml` file for `{crate_name}`: {note}.
		"
    )
}

pub fn unknown_prebuild(crate_name: &str, prebuild_name: &str) -> ! {
    panic!(
        "\
		Error: Crate `{crate_name}` lists `{prebuild_name}` as a crate to prebuild, but `{prebuild_name}` \
		has no entry in the `crates` table.\
		"
    )
}
