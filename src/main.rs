extern crate clap;
extern crate csv;
extern crate rand;
extern crate hyper;
extern crate url;

mod storage;
mod util;

use serde::Deserialize;
use clap::{App, Arg};
use hyper::{Body, Response, Server};
use hyper::rt::Future;
use hyper::service::{service_fn_ok};
use std::fs::File;
use std::net::SocketAddr;
use storage::{Storage, InMemoryStorage};
use std::sync::Arc;
use url::Url;


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

    let mut initializable_banners = InMemoryStorage::new();
    for record_result in reader.deserialize() {
        let record: BannerRecord = record_result.expect("CSV deserialization error");
        let record_dup = record.clone();
        if let Err(e) = initializable_banners.add_banner(record.url, record.shows_amount, record.categories) {
            eprintln!("Banners {:?} isn't added: {}", record_dup, e);
        }
    }

    println!("{} loaded", &initializable_banners);

    let port = args.value_of("http_port").unwrap();
    let bind_addr: SocketAddr = ("0.0.0.0:".to_owned() + port).parse().expect("Illegal bind address");
    let banners = Arc::new(initializable_banners);

    let base_url = Arc::new(Url::parse("http://localhost").unwrap());

    let service = move || {
        let storage = banners.clone();
        let base = base_url.clone();

        service_fn_ok(move |req| {
            let uri = req.uri().to_string();
            let url = base.join(uri.as_str()).unwrap();
            let categories = url.query_pairs().filter_map(|(param, val)| {
                if param.eq("category") {
                    Some(val.to_string())
                } else {
                    None
                }
            }).collect::<Vec<String>>();

            storage.get_banner_html(categories)
                .map_or_else(|| Response::builder().status(204).body(Body::empty()).unwrap(),
                             |html| Response::new(Body::from(html)))
        })
    };

    let server = Server::bind(&bind_addr)
        .serve(service)
        .map_err(|e| eprintln!("Server error: {}", e));

    println!("Start listening on {}", &bind_addr);
    hyper::rt::run(server);

}

#[derive(Debug, Deserialize, Clone)]
struct BannerRecord {
    url: String,
    shows_amount: u32,
    categories: Vec<String>,
}
