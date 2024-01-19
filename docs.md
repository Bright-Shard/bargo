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

Post-build scripts run after a crate is finished compiling. They only run if the crate compiled successfully.

Post-build scripts use [cargo scripts](https://dev-doc.rust-lang.org/stable/cargo/reference/unstable.html#script) under
the hood. Because of this, they do **not** have access to `dev-dependencies` and normal environment variables like build
scripts do. You can add dependencies at the top of the file as described in the
[cargo script docs](https://dev-doc.rust-lang.org/stable/cargo/reference/unstable.html#script).

Bargo does make a few environment variables available to post-build scripts, but they're WIP. Currently, post-build
scripts can access the following:
- `BARGO_ROOT`: The path to the root of the bargo workspace. This is also where the build script gets run.


# Other Bargo Notes

- Bargo's toml parser is [boml](https://github.com/bright-shard/boml). The upside of using boml is that it has no dependencies
and low overhead. The downside is that it's new and not tested as well as other TOML parsers. If bargo has an issue while
reading your `bargo.toml` config, and you're certain the toml is valid, please open an issue in
[boml's repo](https://github.com/bright-shard/boml).
