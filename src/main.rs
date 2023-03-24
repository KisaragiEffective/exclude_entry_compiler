use clap::Parser;
use serde::Deserialize;
use serde_with::DeserializeFromStr;
use strum::EnumString;

#[derive(Deserialize)]
struct EntryList {
    entries: Vec<Entry>
}

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
        #[clap(short = 'f', long)]
        feature_flag: Vec<Include>,
    },
    Check {
        #[clap(short = 't', long)]
        target: CompileTarget,
        #[clap(short = 'f', long)]
        include: Vec<Include>,
    }
}

#[derive(EnumString, Copy, Clone)]
enum CompileTarget {
    #[strum(serialize = "uBlackList")]
    UBlackList,
    #[strum(serialize = "uBlockOrigin")]
    UBlockOrigin,
}

#[derive(EnumString, Copy, Clone)]
enum Include {
    Base,
    GoogleSearch
}

#[derive(EnumString, Copy, Clone, DeserializeFromStr)]
enum MatchMethod {
    Literal
}

fn main() {
    println!("Hello, world!");
}
