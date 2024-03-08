# Configuration

Bargo can be configured via a `bargo.toml` file. Throughout these docs, this `bargo.toml` file will be called "the config file", and the folder containing it will be called the "workspace root".

The config file will have two kinds of entries: The workspace table, and crate entries. The workspace table is a TOML table and defines rules that apply to every crate in the Bargo workspace. Crates in the Bargo workspace are defined with crate entries. Each entry is a table inside of the `crates` table, whose name matches the crate name. If the name does not match the crate's name, Bargo will error. This is to prevent Bargo from accidentally resolving and building the wrong crate.

```toml
# Workspace table
[workspace]

# Crate entry, if the crate is named "bargo"
[crates.bargo]
```

The workspace table and crate tables share many settings, described in the "General Configuration" section right below this. Some settings are exclusive to crate tables, and are described in the "Crate Table Configuration" section below the general config section. Other settings are exclusive to the workspace table, and are described in the "Workspace Table Configuration" below the crate table config section.

## General Configuration

These settings are available for the workspace table and crate entries. If any of these keys are defined in the workspace table, they're used as default values for any crates that don't also define that key.

#### `direct-arg`

Type: String or array of strings

Specifies a direct argument, which is fed to Cargo exactly as provided. Can also be an array of arguments, which are given to Cargo exactly as provided in the same order they're provided.

#### `postbuild`

Type: String

Specifies a path to a Rust script that will be executed after the crate finishes compiling. See the "Post-Build Scripts" section below for more information.

#### `prebuild`

Type: Table

Each entry in the prebuild table specifies a crate to be built before the current crate is built. An entry's name is the name of the crate that needs to be prebuilt. Each entry is also a table that can optionally have `target` and `feature` keys, both of which can either be a string or array of strings. The first defines the target(s) the prebuild crate should be built for,
and the latter enables specific features on the prebuild crate.

#### `target`

Type: String or array of strings

Defines a target or targets to build the crate for. If multiple targets are specified, Bargo will recompile the crate, once for each target specified. Each target must either be a target triple (the same ones Cargo uses) or a path to a custom target file. Bargo assumes the target is a custom target if it contains `.json`. All paths are relative to the workspace root.

#### `unstable`

Type: Table

Specifies unstable Cargo features to be enabled when building the crate. Each key in the table is the name of an unstable feature to enable. Each value to the table must be a boolean, string, or array of strings.

Booleans will be converted into flags; for example, `unstable = { unstable-options = true }` will result in `-Zunstable-options` being passed to Cargo.

Strings or array of strings will be converted into `key=value` arguments; for example, `unstable = { build-std = "core" }` will result in `-Zbuild-std=core` being passed to Cargo, and `unstable = { build-std = ["core", "alloc"] }` will result in `-Zbuild-std=core,alloc` being passed to Cargo.

## Crate Table Configuration

These settings are only available for entries into the crate table.

#### `path`

Type: String

Specifies the path to the crate being configured. All paths are relative to the workspace root.

If unspecified, Bargo will look for the crate at `<workspace root>/<crate name>`.

## Workspace Table Configuration

These settings are only available for the workspace table.

#### `default-build`

Type: String or array of strings

When `bargo build` is run, if no crates are specified in the command, Bargo will build the crates specified here. If it's a string, Bargo will treat it as one crate to build. If it's an array, Bargo will build every crate in the array.

#### `default-run`

Type: String

When `bargo run` is run, if no crate is specified in the command, bargo will build the crate specified here. Note that, unlike `default-build` and `bargo build`, `default-run` and `bargo run` only support 1 crate (it doesn't make sense to run multiple crates simultaneously).

# Post-Build Scripts

Post-build scripts run after a crate is finished compiling. They only run if the crate compiled successfully. If a crate is compiled for multiple targets, this script will run after all targets have finished compiling.

Post-build scripts use [cargo scripts](https://dev-doc.rust-lang.org/stable/cargo/reference/unstable.html#script) under the hood. Because of this, they do **not** have access to `dev-dependencies` and normal environment variables like build scripts do. You can add dependencies at the top of the file as described in the [cargo script docs](https://dev-doc.rust-lang.org/stable/cargo/reference/unstable.html#script).

Bargo does make a few environment variables available to post-build scripts, but they're WIP. Currently, post-build scripts can access the following:

- `BARGO_ROOT`: The path to the root of the bargo workspace. This is also where the build script gets run.
- `PROFILE`: The optimisation profile. Set to `debug` unless Bargo is building in release mode, in which case it's set to `release`.

# Details on How Bargo Works & Possible Errors

These errors are pretty theoretical, but could happen nevertheless since Bargo is new and relatively untested.

- Bargo's toml parser is [boml](https://github.com/bright-shard/boml), a fast and (nearly) zero-copy TOML parser. It can parse all TOML except date/time values, and passes the valid tests in TOML's test suite (though it skips date/time tests). It doesn't pass TOML's invalid tests (yet), meaning it's not yet a perfect parser. This shouldn't happen, since it passes the valid TOML tests; but if Bargo is parsing your TOML wrong, and you're certain the TOML is valid, please open an issue on [BOML's GitHub repo](https://github.com/bright-shard/boml).
- Bargo attempts to detect cyclic dependencies by maintaining a queue of crates it hasn't built yet, and erroring if a crate gets put in that queue twice. I'm fairly certain this approach will prevent cyclic deps; however, if bargo gets stuck in an infinite loop, please open an issue here.
