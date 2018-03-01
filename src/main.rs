extern crate futures;
extern crate hyper;
extern crate tokio_core;
extern crate pretty_env_logger;
extern crate serde;
extern crate serde_json;
extern crate semver;

#[macro_use]
extern crate serde_derive;

use std::env;

use futures::Future;
use futures::stream::Stream;

use hyper::Client;

use std::fs::File;
use std::io::prelude::*;

use serde_json::{Value, Map};

use semver::{Version, VersionReq};

#[derive(Serialize, Deserialize)]
struct Project {
    name: String,
    version: String,
    author: String,
    dependencies: Map<String, Value>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Response {
    name: String,
    versions: Map<String, Value>,
    disttags: Map<String, Value>,
}

fn check_version(name: String, v: String) {
    let version = &v.replace("\"", "");
    let full_url = "http://registry.npmjs.org/".to_owned() + &name;
    let url = full_url.parse::<hyper::Uri>().unwrap();

    let mut core = tokio_core::reactor::Core::new().unwrap();
    let handle = core.handle();
    let client = Client::new(&handle);

    let f = client.get(url).map_err(|_err| ()).and_then(|resp| {
        resp.body().concat2().map_err(|_err| ()).map(|chunk| {
            let v = chunk.to_vec();
            String::from_utf8_lossy(&v).to_string()
        })
    });

    let response = core.run(f).unwrap();
    // TODO: This is really bad
    let correct_response = &response.replace("dist-tags", "disttags");
    let package_info: Response = serde_json::from_str(&correct_response).unwrap();
    let latest_version = package_info.disttags["latest"].as_str().unwrap();

    let current_version = VersionReq::parse(&version).unwrap();
    let version_match = current_version.matches(&Version::parse(&latest_version).unwrap());
    let (_, vv) = version.split_at(1);
    match version_match {
        false => println!("{}: Hey, new version that is SemVer incompatible, our: {}, latest {}", name, version, latest_version),
        true => match vv == latest_version {
            false => println!("{}: Hey, new SemVer compatible version is out, our: {}, latest: {}", name, version, latest_version),
            true => return
        }
    }
}

fn main() {
    pretty_env_logger::init();

    let url = match env::args().nth(1) {
        Some(url) => url,
        None => {
            println!("Usage: npm-updater path-to-package-json");
            return;
        }
    };

    let mut file = File::open(url).unwrap();
    let mut content = String::new();
    file.read_to_string(&mut content).unwrap();
    let project: Project = serde_json::from_str(&content).unwrap();

    for dependency in project.dependencies {
        let (name, version) = dependency;
        check_version(name, version.to_string());
    }
}
