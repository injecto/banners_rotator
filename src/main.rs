extern crate clap;
extern crate csv;
extern crate rand;

mod storage;

use serde::Deserialize;
use clap::{App, Arg};
use std::fs::File;

#[derive(Debug, Deserialize)]
struct BannerRecord {
    url: String,
    shows_amount: u64,
    categories: Vec<String>,
}

fn main() {
    let args = App::new("Banners rotator")
        .arg(Arg::with_name("FILE")
            .help("Banners config as CSV")
            .index(1)
            .required(true))
        .arg(Arg::with_name("http_port")
            .short("p")
            .long("port")
            .help("Listening HTTP port")
            .default_value("8080"))
        .get_matches();

    let config_file = File::open(args.value_of("FILE").unwrap()).expect("Can't open config file");
    let mut reader = csv::ReaderBuilder::new()
        .delimiter(b';')
        .flexible(true)
        .has_headers(false)
        .from_reader(config_file);

    for record_result in reader.deserialize() {
        let record: BannerRecord = record_result.expect("CSV deserialization error");
        println!("{:?}", record);
    }
}
