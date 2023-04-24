#![deny(clippy::all)]
#![warn(clippy::pedantic, clippy::nursery)]

use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::PathBuf;
use std::process::{exit, ExitCode};
use std::str::FromStr;
use clap::Parser;
use serde::Deserialize;
use serde_with::DeserializeFromStr;
use strum::EnumString;
use thiserror::Error;

#[derive(Deserialize)]
struct EntryList(Vec<Entry>);

#[derive(Deserialize)]
#[serde(tag = "type")]
enum Entry {
    #[serde(rename = "domain")]
    Domain {
        #[serde(rename = "match")]
        match_method: MatchMethod,
        domain: String,
    },
    #[serde(rename = "path")]
    Path {
        #[serde(rename = "match")]
        match_method: MatchMethod,
        path: String,
    }
}

#[derive(Parser)]
enum Args {
    Compile {
        #[clap(short = 't', long)]
        target: CompileTarget,
        #[clap(short = 'f', long = "feature", long)]
        feature_flag: Vec<GenerateTargetPlatform>,
        #[clap(short = 'i', long = "in", long = "input", long)]
        input_file: PathBuf,
        #[clap(short = 'o', long = "out", long = "output", long)]
        output_file: PathBuf,
        #[clap(short = 'h', long = "header", long)]
        /// Header attributes. Format: 'K=V'
        header_attributes: Vec<HeaderAttribute>,
        #[clap(short = 'v', long)]
        verbose: bool,
    },
    Check {
        input_file: PathBuf,
    },
}

#[derive(Clone, Eq, PartialEq)]
struct HeaderAttribute {
    key: String,
    value: String,
}

impl FromStr for HeaderAttribute {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (key, value) = s.split_once('=').ok_or(())?;
        Ok(Self {
            key: key.to_string(),
            value: value.to_string(),
        })
    }
}

impl From<&str> for HeaderAttribute {
    fn from(value: &str) -> Self {
        <Self as FromStr>::from_str(value).expect("!!")
    }
}

#[derive(EnumString, Copy, Clone, Eq, PartialEq)]
enum CompileTarget {
    #[strum(serialize = "uBlackList")]
    UBlackList,
    #[strum(serialize = "uBlockOrigin")]
    UBlockOrigin,
}

#[derive(EnumString, Copy, Clone, Eq, PartialEq)]
enum GenerateTargetPlatform {
    Base,
    /// Generates Google search block rule. Match if and only if the URL prefix matches in deny list entry.
    GoogleSearchPrefix,
    /// Also generates Google search block rule. Match if and only if the URL contains deny list entry.
    GoogleSearchFuzzy,
}

#[derive(EnumString, Copy, Clone, Eq, PartialEq, DeserializeFromStr)]
enum MatchMethod {
    #[strum(serialize = "literal")]
    Literal
}

#[derive(Error, Debug)]
enum CompileError {
    #[error("JSON Deserialize error: {0}")]
    Deserialize(#[from] serde_json::Error),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Unsupported feature combination")]
    UnsupportedFeatureSet,
    #[error("Syntax error: {0}")]
    Syntax(#[from] SyntaxCheckError),
}

#[derive(Error, Debug)]
enum SyntaxCheckError {
    #[error("JSON Deserialize error: {0}")]
    Deserialize(#[from] serde_json::Error),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error)
}

#[derive(Error, Debug)]
enum ExecutionError {
    #[error("Failed to compile: {0}")]
    Compile(#[from] CompileError),
    #[error("Failed to syntax check: {0}")]
    Check(#[from] SyntaxCheckError),
}

fn main() -> ExitCode {
    let x = imp::main();

    if let Err(e) = x {
        eprintln!("{e}");
        ExitCode::from(1)
    } else {
        ExitCode::from(0)
    }
}

mod imp {
    use clap::Parser;
    use crate::{Args, compile, ExecutionError, syntax_check};

    #[allow(clippy::redundant_pub_crate)]
    // ExecutionError must be pub if this vis is also pub
    pub(crate) fn main() -> Result<(), ExecutionError> {
        let args = Args::parse();
        match args {
            Args::Compile { target, feature_flag, input_file, output_file, header_attributes, verbose } => {
                compile(input_file, target, &feature_flag, output_file, &header_attributes, verbose)?;
            }
            Args::Check { input_file } => {
                syntax_check(input_file)?;
            }
        };

        Ok(())
    }
}

#[allow(clippy::too_many_lines)]
fn compile(
    input_file: PathBuf,
    target: CompileTarget,
    feature_flags: &[GenerateTargetPlatform],
    output_file: PathBuf,
    header_attributes: &[HeaderAttribute],
    verbose: bool,
) -> Result<(), CompileError> {
    if feature_flags.is_empty() {
        return Ok(())
    }

    if target != CompileTarget::UBlockOrigin && feature_flags.contains(&GenerateTargetPlatform::GoogleSearchPrefix) {
        return Err(CompileError::UnsupportedFeatureSet)
    }

    let google_search_prefix = feature_flags.contains(&GenerateTargetPlatform::GoogleSearchPrefix);
    let google_search_fuzzy = feature_flags.contains(&GenerateTargetPlatform::GoogleSearchFuzzy);

    if google_search_prefix && google_search_fuzzy {
        eprintln!("Both --include=GoogleSearchPrefix and --include=GoogleSearchFuzzy must not be used in same time.");
        eprintln!("Please separate call.");
        exit(1);
    }

    let google = google_search_prefix || google_search_fuzzy;

    let list = syntax_check(input_file)?;
    if verbose {
        println!("loaded {} entries", list.0.len());
    }

    let mut writer = BufWriter::new(
        File::options().write(true).truncate(true).create(true).open(output_file)?
    );

    let comment = match target {
        CompileTarget::UBlackList => "#",
        CompileTarget::UBlockOrigin => "!",
    };

    let mut outputs = vec![];
    let header = header_attributes.iter().map(|x| {
        let mut buf = String::with_capacity(determine_header_attribute_length(x));
        buf.push_str(comment);
        buf.push(' ');
        buf.push_str(&x.key);
        buf.push_str(": ");
        buf.push_str(&x.value);
        buf.push('\n');

        buf
    }).collect::<String>();
    outputs.push(header);
    if verbose {
        println!("loaded {} headers", header_attributes.len());
    }

    if feature_flags.contains(&GenerateTargetPlatform::Base) {
        let entry_serialize: String = match target {
            CompileTarget::UBlackList => {
                /*
                jq -r '.[] | select(.type == "domain") | .domain | ("*://" + . + "/*")' < "$data" >> "$dist"
                jq -r '.[] | select(.type == "path") | .path | ("*://" + .)' < "$data" >> "$dist"

                */ */

                list.0.iter().map(|x| match x {
                    Entry::Domain { match_method, domain } => {
                        match *match_method {
                            MatchMethod::Literal => format!("*://{domain}/*\n"),
                        }
                    }
                    Entry::Path { match_method, path } => {
                        match *match_method {
                            MatchMethod::Literal => format!("*://{path}\n"),
                        }
                    }
                }).collect()
            }
            CompileTarget::UBlockOrigin => {
                list.0.iter().map(|x| match x {
                    Entry::Domain { match_method, domain: out }
                    | Entry::Path { match_method, path: out } => {
                        match *match_method {
                            MatchMethod::Literal => format!("||{out}^\n"),
                        }
                    }
                }).collect()
            }
        };

        if verbose {
            println!("pushed General block rules");
        }

        outputs.push(entry_serialize);
    }

    if google {
        let href_operator = if google_search_prefix {
            "^="
        } else {
            "*="
        };

        let cp = list.0.iter().filter_map(|x| {
            match x {
                Entry::Domain { match_method, domain } => {
                    (*match_method == MatchMethod::Literal).then_some(domain)
                }
                Entry::Path { match_method, path } => {
                    (*match_method == MatchMethod::Literal).then_some(path)
                }
            }
        }).flat_map(|href_spec| {
            [
                format!(r#"www.google.*##.g:has(a[href{href_operator}"{href_spec}")"#),
                format!(r#"www.google.*##.a[href{href_operator}"{href_spec}"]:upward(1)"#),
            ]
        }).collect::<Vec<_>>().join("\n");

        if verbose {
            println!("pushed Google block rules");
        }
        outputs.push(cp);
    }

    if verbose {
        println!("writing file");
    }

    writer.write_all(outputs.join("").as_bytes())?;

    Ok(())
}

fn determine_header_attribute_length(attr: &HeaderAttribute) -> usize {
    2 + attr.key.len() + 2 + attr.value.len() + 1
}

fn syntax_check(input: PathBuf) -> Result<EntryList, SyntaxCheckError> {
    let mut json = String::new();
    BufReader::new(File::open(input)?).read_to_string(&mut json)?;
    let x = serde_json::from_str(&json)?;
    Ok(x)
}
