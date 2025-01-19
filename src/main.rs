#![allow(dead_code)]
#![allow(unused)]

pub mod http {

    use clap::{arg, ArgMatches, Command};

    pub fn parse_args() -> ArgMatches {
        let matches = Command::new("gcs-rs")
            .about("A rust-based, barebones GCS client based on Google Cloud HTTP API")
            .arg(arg!(--op <VALUE>).required(true))
            .arg(arg!(--uri <VALUE>).required(true))
            .get_matches();

        println!(
            "Value for uri: {:?}",
            matches.get_one::<String>("uri").expect("required")
        );

        println!(
            "Value for op: {:?}",
            matches.get_one::<String>("op").expect("required")
        );

        matches
    }
}

#[tokio::main]
async fn main() -> Result<(), reqwest::Error> {
    use http::parse_args;

    // TODO: validate URI and GCS operation
    let matches = parse_args();
    let uri = matches.get_one::<String>("uri").expect("required");
    let op = matches.get_one::<String>("op").expect("required");

    /// # Authorization
    ///
    /// Cloud Storage requests made through the XML API support both RSA keys and HMAC keys
    /// as credentials.
    ///
    /// JSON API only supports a bearer token. I think this is only obtained through `gcloud
    /// auth`, e.g.:
    ///
    /// curl -X GET \
    ///   -H "Authorization: Bearer $(gcloud auth print-access-token)" \
    ///   -H "x-goog-user-project: PROJECT_ID" \
    ///   "https://iam.googleapis.com/v1/projects/PROJECT_ID/serviceAccounts"
    ///
    ///   
    // Basic request
    let res = reqwest::get(uri).await?;
    println!("Status: {}", res.status());
    println!("Headers:\n{:#?}", res.headers());
    let body = res.text().await?;
    println!("Body:\n{}", body);

    Ok(())
}
