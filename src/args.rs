//! Bargo's argument parser.

use {crate::crate_prelude::*, std::iter::Peekable};

/// The parsed arguments provided to Bargo.
#[cfg_attr(test, derive(PartialEq, Debug))]
pub struct Args<'a> {
    /// The subcommand to run.
    pub subcommand: Option<Subcommand>,
    /// If release mode is enabled.
    pub release: bool,
    /// Crate features that were enabled.
    pub features: Vec<&'a str>,
    /// A custom target that was specified.
    pub targets: Vec<&'a str>,
    /// Crates to build.
    pub crates: Vec<&'a str>,
}

/// Bargo's subcommands.
#[derive(PartialEq)]
#[cfg_attr(test, derive(Debug))]
pub enum Subcommand {
    Build,
    Run,
    Help,
}

/// Bargo's arguments.
#[cfg_attr(test, derive(PartialEq, Debug))]
pub enum ArgType<'a> {
    /// `-r` or `--release`, enables building in release mode.
    Release,
    /// `-F` or `--features`, sets crate features while building.
    Features,
    /// `--target`, sets a custom target while building.
    Target,
    /// `-p` or `--package`, specifies which crates to build.
    Package,
    /// An argument that didn't start with a `-`. Could be a subcommand
    /// or value for one of the above arguments.
    Keyword(&'a str),
}

impl<'a> Args<'a> {
    /// Parses a list of arguments into Bargo args.
    pub fn parse(raw_args: Peekable<impl Iterator<Item = &'a str>>) -> Result<Self, Error> {
        let mut this = Self {
            subcommand: None,
            release: false,
            features: Vec::new(),
            targets: Vec::new(),
            crates: Vec::new(),
        };

        this.parse_(raw_args)?;

        Ok(this)
    }

    /// The actual implementation of [`Args::parse`]. It's just split off so I can use `self`; it's always
    /// inlined, and only called from `parse`.
    #[inline(always)]
    fn parse_(
        &mut self,
        mut raw_args: Peekable<impl Iterator<Item = &'a str>>,
    ) -> Result<(), Error> {
        while let Some(raw_arg) = raw_args.next() {
            // Arguments may be passed as `--arg val1 val2 val3`, or `--arg=val1,val2,val3`, or some weird
            // combination of the two. Commas are handled in `parse_arg_values`. The equals sign is handled
            // here; the argument is split by the equals, the arg name is the first item in the split string,
            // and the arg values are the second item in the split string.
            let mut arg_assignment = raw_arg.splitn(2, '=');
            let raw_arg = arg_assignment.next().unwrap();

            let to_push = match Self::parse_arg(raw_arg)? {
                ArgType::Features => Some(&mut self.features),
                ArgType::Target => Some(&mut self.targets),
                ArgType::Package => Some(&mut self.crates),
                ArgType::Release => {
                    self.release = true;

                    None
                }
                ArgType::Keyword(subcommand) => {
                    if self.subcommand.is_none() {
                        self.subcommand = Some(match subcommand {
                            "r" | "run" => Subcommand::Run,
                            "b" | "build" => Subcommand::Build,
                            "h" | "help" | "?" => Subcommand::Help,
                            _ => return Err(Error::UnknownArgument(subcommand.to_string())),
                        });
                    }

                    None
                }
            };

            if let Some(to_push) = to_push {
                // If there were values after an `=`, push the values
                if let Some(arg_values) = arg_assignment.next() {
                    for arg in arg_values.split(',') {
                        to_push.push(arg);
                    }
                }
                Self::parse_arg_values(to_push, &mut raw_args)?
            }
        }

        Ok(())
    }

    #[inline(always)]
    fn parse_arg_values<'b>(
        list: &mut Vec<&'b str>,
        args: &mut Peekable<impl Iterator<Item = &'b str>>,
    ) -> Result<(), Error> {
        while let Some(arg) = args.peek() {
            if let ArgType::Keyword(keywords) = Self::parse_arg(arg)? {
                for keyword in keywords.split(',') {
                    list.push(keyword);
                }
            } else {
                break;
            }
            args.next();
        }

        Ok(())
    }

    /// If the argument starts with `-` or `--`, consumes the argument and modifies `Args` appropriately.
    /// Otherwise, returns the argument.
    fn parse_arg(arg: &str) -> Result<ArgType<'_>, Error> {
        let arg = arg.trim();
        let bytes = arg.as_bytes();

        if bytes[0] == b'-' {
            match bytes[1] {
                b'-' => match &arg[2..] {
                    "release" => Ok(ArgType::Release),
                    "features" | "feature" => Ok(ArgType::Features),
                    "target" | "targets" => Ok(ArgType::Target),
                    "package" | "packages" => Ok(ArgType::Package),
                    _ => Err(Error::UnknownArgument(arg.to_string())),
                },
                b'r' => Ok(ArgType::Release),
                b'F' => Ok(ArgType::Features),
                b'p' => Ok(ArgType::Package),
                _ => Err(Error::UnknownArgument(arg.to_string())),
            }
        } else {
            Ok(ArgType::Keyword(arg))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test release mode args
    #[test]
    fn release() {
        ArgsTester {
            expected: Args {
                subcommand: None,
                release: true,
                features: Vec::new(),
                targets: Vec::new(),
                crates: Vec::new(),
            },
            inputs: vec![vec!["-r"], vec!["--release"]],
        }
        .test();
    }

    /// Test arguments that accept values
    #[test]
    fn valued_args() {
        ArgsTester {
            expected: Args {
                subcommand: None,
                release: false,
                features: vec!["feat1", "feat2", "feat3"],
                targets: Vec::new(),
                crates: Vec::new(),
            },
            inputs: vec![
                vec!["--features=feat1,feat2,feat3"],
                vec!["-F=feat1,feat2,feat3"],
                vec!["--features", "feat1", "feat2", "feat3"],
                vec!["-F", "feat1", "feat2", "feat3"],
                vec!["--features=feat1", "feat2,feat3"],
                vec!["-F=feat1", "feat2,feat3"],
            ],
        }
        .test();
    }

    /// Sanity check that other arguments are accepted
    #[test]
    fn other_args() {
        ArgsTester {
            expected: Args {
                subcommand: Some(Subcommand::Build),
                release: false,
                features: Vec::new(),
                targets: vec!["x86_64-unknown-none"],
                crates: vec!["main"],
            },
            inputs: vec![vec![
                "build",
                "--target",
                "x86_64-unknown-none",
                "--package",
                "main",
            ]],
        }
        .test();
    }

    struct ArgsTester<'a> {
        pub expected: Args<'a>,
        pub inputs: Vec<Vec<&'a str>>,
    }
    impl ArgsTester<'_> {
        pub fn test(self) {
            for input in self.inputs {
                let output = Args::parse(input.into_iter().peekable()).unwrap();
                assert_eq!(output, self.expected);
            }
        }
    }
}
