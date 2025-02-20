use clap::{arg, ArgMatches, Command};

pub fn parse_args() -> ArgMatches {
    let matches = Command::new("gcs-rs")
        .about("A rust-based, barebones GCS client based on Google Cloud HTTP API")
        .arg(arg!(--op <VALUE>))
        .arg(arg!(--uri <VALUE>))
        .get_matches();

    println!("Value for uri: {:?}", matches.get_one::<String>("uri"));

    matches
}
