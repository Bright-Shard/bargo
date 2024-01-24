# Bargo

**Bargo** is a wrapper around Cargo that provides more features to make a more feature-complete build system.
It's goal is to simplify complex workspaces and projects that already push Cargo's boundaries. It acts nearly
like a drop-in Cargo binary; every argument Bargo supports is identical to its argument in Cargo (though it
does not support every Cargo argument, so it's not a complete drop-in).

Bargo only has 1 dependency ([boml](https://github.com/bright-shard/boml), a TOML parser with 0 dependencies),
and should compile in mere seconds.

*This project was written without the assistance of GitHub CodeStealer, OpenAI's ChatGPThief, or other similar
content-stealing predictive algorithms.*

# To-Do

Everything below this is what Bargo should look like when it's finished; unfortunately, it's not all yet implemented.

- [x] Bargo build
	- [x] Release mode
	- [x] Custom targets
	- [x] Features
- [x] Custom crate paths
- [x] Custom build targets
	- [x] Multiple targets per crate
	- [x] Custom targets (target.json)
- [x] Enabling unstable cargo features
- [x] Direct cargo arguments
- [x] The default-build setting
- [x] Prebuild dependencies
	- [x] Crate features
	- [x] Custom targets
	- [x] Prevent cyclic dependences
- [x] Post-build scripts
	- [ ] Environment variables
- [ ] Bargo run
	- [ ] Release mode
	- [ ] Features
- [ ] The default-run setting
- [ ] Custom runners
- [ ] Stable config format

Essentially, `bargo build` is implemented, but `bargo run` is not. Building has a few limitations. I'm working on
what environment variables to provide to post-build scripts. Bargo is also unstable (hence it being version 0), and
there's nothing preventing breaking changes at the moment. I'd like to have editions, similar to Rust.

# What Bargo Adds

Bargo adds:
- **Post-build scripts**: These act like regular build scripts, but run *after* a crate builds, instead of before.
- **Per-crate targets**: Compile different crates in the same workspace for different target triples. Crates can also
be compiled for multiple target triples.
- **Per-crate unstable Cargo features**: Enable different unstable Cargo features for individual crates in a workspace.
- **Binary dependencies**: Guarantee that another binary crate in the workspace gets built before the current one.

These features are just the ones I've needed for my operating system, [bs](https://github.com/bright-shard/bs). If there's
other features that you want, open a GitHub issue and request them!

# Using Bargo

Note: Below is an overview, detailed and searchable information is available in [docs.md](docs.md).

## The Config

Create a `bargo.toml` config file in the root of your workspace. This acts quite similarly to normal `Cargo.toml` files.

To specify crates in the workspace, put them in the `[crates]` table. The name of each crate entry should match the
crate's actual name; bargo will error if they do not match.

```toml
[crates.crate123]
# The path to this crate.
# If unspecified, bargo will look in `<root>/<crate name>`, where `<root>` is the folder with the `bargo.toml` file.
path = "path/to/crate123"
# A build script that runs after the crate finishes building.
postbuild = "builder.rs"
# The target triple to build this crate for.
target = "x86_64-unknown-none"
# Unstable cargo features to enable.
unstable = { build-std = "core", unstable-options = true }
# A custom runner, for when `bargo r`/`bargo run` is used. (Currently unimplemented)
runner = "some-wrapper-script"
# Other workspace members to build before building this crate. This is similar to the unstable artifact dependencies
# feature, but not exactly the same (see the docs).
# Any arguments passed via CLI (like --features) are ignored for prebuilds. You can instead specify crate features
# and targets in the table directly.
prebuild.crate456 = {}
# Pass direct arguments to Cargo
direct-arg = "--build-plan"

[crates.crate456]
target = "x86_64-unknown-none"

[crates.crate123_arm]
target = "aarch64-unknown-none"
```

Note that the `target`, `direct-arg`, and `prebuild` settings can be an array to specify multiple values - ie, to
build a crate for `x86_64` and `arm64`, you could specify: `target = ["x86_64-unknown-none", "aarch64-unknown-none"]`.

Also note that *all* paths defined in the config - the `path` setting, the path to a post-build script, the path
to a custom `target.json` file - are relative to the workspace root (the folder with `bargo.toml` in it).

You can also define a `[workspace]` table, which can define defaults for all of the settings above. If a crate entry
defines the same setting as the workspace table, the crate entry overrides it. We can simplify the targets set in the
above example with this:

```toml
[workspace]
target = "x86_64-unknown-none"

[crates.crate123]
path = "path/to/crate123"
# (other configs cropped here)

[crates.crate456]

[crates.crate123_arm]
# Overrides the workspace default
target = "aarch64-unknown-none"
```

The `workspace` table has a few differences from crate tables:
- It can specify `default-build` and `default-run` values, which specify which crates to build and run by default. The
`default-build` value can be a string (for one crate) or array (for many crates). The `default-run` value must be a string
(it can only specify one crate).
- The `path` key is ignored in the workspace table.

## Post-Build Scripts

Post-build scripts will get automatically run after a crate is successfully compiled. They will not get run if a crate fails
to compile.

Post-build scripts are implemented with the (unstable)
[cargo scripts](https://dev-doc.rust-lang.org/stable/cargo/reference/unstable.html#script) feature. They **do not** have
access to `dev-dependencies` like build scripts do; dependencies should be declared at the top as the documentation describes.

Unless specified otherwise in the config, bargo will look for a file named `postbuild.rs` in the crate's root. If it finds
that file, it runs it after compiling the crate.

Bargo passes a few environment variables to `postbuild.rs`, though not as many `build.rs` files have (environment is wip).
See the docs for more info.

## Bargo Itself

Just like Cargo, bargo has a `build` (or `b`) subcommand and `run` (or `r`) subcommand. The only arguments to these commands
are the crates to build or run (ie, `bargo b crate123` will build crate123).

If `bargo b` is run with no crates specified, bargo will build any crates specified in `default-build` in the `workspace` table.
If `default-build` is unspecified, bargo will build every crate in the workspace.

If `bargo r` is run with no crates specified, bargo will run the crate specified by `default-run` in the `workspace` table. If
`default-run` isn't set, bargo will error. **`bargo r` is currently unimplemented.**

You can build in release mode (`-r`), specify crate features to enable (`--features`), specify the target to build for (`--target`),
or specify which crates to build (`-p`).
