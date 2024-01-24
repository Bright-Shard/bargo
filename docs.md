# Configuration

Bargo can be configured via a `bargo.toml` file. Throughout these docs, this `bargo.toml` file will be called "the config file",
and the folder containing it will be called the "workspace root".

The config file will have two kinds of entries: The workspace table, and crate entries. The workspace table is a TOML table and
defines rules that apply to every crate in the Bargo workspace. Crates in the Bargo workspace are defined with crate entries.
Each entry is a table inside of the `crates` table, whose name matches the crate name. If the name does not match the crate's
name, Bargo will error. This is to prevent Bargo from accidentally resolving and building the wrong crate.

```toml
# Workspace table
[workspace]

# Crate entry, if the crate is named "bargo"
[crates.bargo]
```

The workspace table and crate tables share many settings, described in the "General Configuration" section right below this.
Some settings are exclusive to crate tables, and are described in the "Crate Table Configuration" section below the general
config section. Other settings are exclusive to the workspace table, and are described in the "Workspace Table
Configuration" below the crate table config section.

## General Configuration

These settings are available for the workspace table and crate entries.

## Crate Table Configuration

These settings are only available for entries into the crate table.

## Workspace Table Configuration

These settings are only available for the workspace table.



# Post-Build Scripts

Post-build scripts run after a crate is finished compiling. They only run if the crate compiled successfully. If a crate
is compiled for multiple targets, this script will run after all targets have finished compiling.

Post-build scripts use [cargo scripts](https://dev-doc.rust-lang.org/stable/cargo/reference/unstable.html#script) under
the hood. Because of this, they do **not** have access to `dev-dependencies` and normal environment variables like build
scripts do. You can add dependencies at the top of the file as described in the
[cargo script docs](https://dev-doc.rust-lang.org/stable/cargo/reference/unstable.html#script).

Bargo does make a few environment variables available to post-build scripts, but they're WIP. Currently, post-build
scripts can access the following:
- `BARGO_ROOT`: The path to the root of the bargo workspace. This is also where the build script gets run.
- `PROFILE`: The optimisation profile. Set to `debug` unless Bargo is building in release mode, in which case it's
set to `release`.


# Details on How Bargo Works & Possible Errors

These errors are pretty theoretical, but could happen nevertheless since Bargo is new and relatively untested.

- Bargo's toml parser is [boml](https://github.com/bright-shard/boml), a fast and (nearly) zero-copy TOML parser. It can
parse all TOML except date/time values, and passes the valid tests in TOML's test suite (though it skips date/time tests).
It doesn't pass TOML's invalid tests (yet), meaning it's not yet a perfect parser. This shouldn't happen, since it passes
the valid TOML tests; but if Bargo is parsing your TOML wrong, and you're certain the TOML is valid, please open an issue
on [BOML's GitHub repo](https://github.com/bright-shard/boml).
- Bargo attempts to detect cyclic dependencies by maintaining a queue of crates it hasn't built yet, and erroring if a crate
gets put in that queue twice. I'm fairly certain this approach will prevent cyclic deps; however, if bargo gets stuck in an
infinite loop, please open an issue here.
