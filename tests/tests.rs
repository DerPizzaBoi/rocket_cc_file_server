use std::path::{Path, PathBuf};
use std::sync::Arc;
use rocket::fs::{NamedFile, Options};
use rocket::{Build, Rocket, routes, uri};
use rocket::get;
use rocket::launch;
use rocket::http::Status;
use rocket::local::blocking::Client;
use rocket_cc_file_server::{CCFileServer, CCOptions};

#[launch]
fn rocket() -> Rocket<Build> {
    let options = Arc::new(CCOptions {
        expires: None,
        max_age: Some(300),
        is_public: Some(true),
        no_cache: Some(()),
        no_store: None
    });
    rocket::build()
        .mount("/files", CCFileServer::new("test_dir/", options, Options::default()))
}

#[test]
fn test_all() {
    let client = Client::tracked(rocket()).expect("valid rocket instance");
    let mut response = client.get(uri!("/files/test_file")).dispatch();
    assert_eq!(response.status(), Status::Ok);
    println!("{:?}", response.headers());
    assert_eq!(response.headers().get_one("Cache-Control"), Some("public, no-cache, max-age=300;"));
    assert_eq!(response.into_string().unwrap(), "1234asdf");
}