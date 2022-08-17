use crate::{Meta, Named};

#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
enum ItemKind {
    Flag,
    Command,
    Decor,
    Positional,
}

#[doc(hidden)]
#[derive(Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
pub enum Item {
    Decor {
        help: Option<String>,
    },
    Positional {
        metavar: &'static str,
        help: Option<String>,
    },
    Command {
        name: &'static str,
        short: Option<char>,
        help: Option<String>,
    },
    Flag {
        name: ShortLong,
        help: Option<String>,
    },
    Argument {
        name: ShortLong,
        metavar: &'static str,
        env: Option<&'static str>,
        help: Option<String>,
    },
}

impl Item {
    fn kind(&self) -> ItemKind {
        match self {
            Item::Decor { .. } => ItemKind::Decor,
            Item::Positional { .. } => ItemKind::Positional,
            Item::Command { .. } => ItemKind::Command,
            Item::Flag { .. } | Item::Argument { .. } => ItemKind::Flag,
        }
    }

    fn help(&self) -> Option<&String> {
        match self {
            Item::Decor { help }
            | Item::Command { help, .. }
            | Item::Flag { help, .. }
            | Item::Argument { help, .. }
            | Item::Positional { help, .. } => help.as_ref(),
        }
    }
}

#[doc(hidden)]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
pub enum ShortLong {
    Short(char),
    Long(&'static str),
    ShortLong(char, &'static str),
}

impl From<&Named> for ShortLong {
    fn from(named: &Named) -> Self {
        match (named.short.is_empty(), named.long.is_empty()) {
            (true, true) => unreachable!("Named should have either short or long name"),
            (true, false) => Self::Long(named.long[0]),
            (false, true) => Self::Short(named.short[0]),
            (false, false) => Self::ShortLong(named.short[0], named.long[0]),
        }
    }
}

impl ShortLong {
    fn full_width(&self) -> usize {
        match self {
            ShortLong::Short(_) => 2,
            ShortLong::Long(l) | ShortLong::ShortLong(_, l) => 6 + l.len(),
        }
    }
}

impl std::fmt::Display for ShortLong {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if f.alternate() {
            match self {
                ShortLong::Short(short) => write!(f, "-{}", short),
                ShortLong::Long(long) => write!(f, "    --{}", long),
                ShortLong::ShortLong(short, long) => write!(f, "-{}, --{}", short, long),
            }
        } else {
            match self {
                ShortLong::Short(short) | ShortLong::ShortLong(short, _) => write!(f, "-{}", short),
                ShortLong::Long(long) => write!(f, "--{}", long),
            }
        }
    }
}

/// {} renders shorter version that can be used in a short usage string
/// {:#} renders a full width version that can be used in --help body and complete, this version
/// supports padding of the help by some max width
impl std::fmt::Display for Item {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // alternate version is used to render the option list
        if f.alternate() {
            match self {
                Item::Flag { name, help: _ } => write!(f, "    {:#}", name),
                Item::Argument {
                    name,
                    metavar,
                    help: _,
                    env,
                } => {
                    write!(f, "    {:#} <{}>", name, metavar)?;

                    if let Some((width, env)) = f.width().zip(*env) {
                        let pad = width - self.full_width();
                        let val = match std::env::var(env) {
                            Ok(val) => format!(" = {:?}", val),
                            Err(std::env::VarError::NotPresent) => ": N/A".to_string(),
                            Err(std::env::VarError::NotUnicode(_)) => {
                                ": current value is not utf8".to_string()
                            }
                        };
                        let next_pad = 4 + self.full_width();
                        write!(
                            f,
                            "{:pad$}  [env:{}{}]\n{:width$}",
                            "",
                            env,
                            val,
                            "",
                            pad = pad,
                            width = next_pad,
                        )?;
                    }
                    Ok(())
                }
                Item::Decor { help } => {
                    if help.is_some() {
                        write!(f, "    ")?
                    }
                    Ok(())
                }
                Item::Positional { metavar, help: _ } => write!(f, "    <{}>", metavar),
                Item::Command {
                    name,
                    help: _,
                    short,
                } => match short {
                    Some(s) => write!(f, "    {}, {}", name, s),
                    None => write!(f, "    {}", name),
                },
            }?;

            // width must be specified on the top level
            let width = f.width().unwrap();
            if let Some(help) = self.help() {
                let pad = width - self.full_width();
                for (ix, line) in help.split('\n').enumerate() {
                    {
                        if ix == 0 {
                            write!(f, "{:pad$}  {}", "", line, pad = pad)
                        } else {
                            write!(f, "\n{:pad$}      {}", "", line, pad = width)
                        }
                    }?
                }
            }
            Ok(())
        } else {
            // this version is used to render usage and missing fields parts
            match self {
                Item::Decor { .. } => Ok(()),
                Item::Positional { metavar, help: _ } => write!(f, "<{}>", metavar),
                Item::Command { .. } => write!(f, "COMMAND ..."),
                Item::Flag { name, help: _ } => write!(f, "{}", name),
                Item::Argument {
                    name,
                    metavar,
                    help: _,
                    env: _,
                } => write!(f, "{} {}", name, metavar),
            }
        }
    }
}

impl Item {
    #[must_use]
    pub(crate) fn required(self, required: bool) -> Meta {
        if required {
            Meta::Item(self)
        } else {
            Meta::Optional(Box::new(Meta::Item(self)))
        }
    }

    #[must_use]
    /// Full width for the name, including implicit short flag, space and comma
    /// betwen short and log parameters and metavar variable if present
    pub(crate) fn full_width(&self) -> usize {
        match self {
            Item::Decor { .. } => 0,
            Item::Flag { name, .. } => name.full_width(),
            Item::Argument { name, metavar, .. } => name.full_width() + metavar.len() + 3,
            Item::Positional { metavar, .. } => metavar.len() + 2,
            Item::Command { name, short, .. } => name.len() + short.map_or(0, |_| 3),
        }
    }

    #[must_use]
    pub(crate) fn decoration<M>(help: Option<M>) -> Self
    where
        M: Into<String>,
    {
        Item::Decor {
            help: help.map(Into::into),
        }
    }

    #[must_use]
    pub(crate) fn is_command(&self) -> bool {
        match self.kind() {
            ItemKind::Command => true,
            ItemKind::Flag | ItemKind::Decor | ItemKind::Positional => false,
        }
    }

    #[must_use]
    pub(crate) fn is_flag(&self) -> bool {
        match self.kind() {
            ItemKind::Flag | ItemKind::Decor => true,
            ItemKind::Command | ItemKind::Positional => false,
        }
    }

    #[must_use]
    pub(crate) fn is_positional(&self) -> bool {
        match self.kind() {
            ItemKind::Positional => self.help().is_some(),
            ItemKind::Flag | ItemKind::Decor | ItemKind::Command => false,
        }
    }
}
