use std::path::{Path, PathBuf};
use std::sync::Arc;
use rocket::fs::{NamedFile, Options};
use rocket::{Build, Rocket, routes, uri};
use rocket::get;
use rocket::launch;
use rocket::http::Status;
use rocket::local::blocking::Client;
use rocket_cc_file_server::{CCFileServer, CCOptions};

#[test]
fn it_works() {
    assert_eq!(2 + 2, 4);
}

#[launch]
fn rocket() -> Rocket<Build> {
    let options = Arc::new(CCOptions {
        expires: None,
        max_age: Some(300),
        is_public: Some(true),
        no_cache: Some(()),
        no_store: None
    });
    let max_age = Arc::new(CCOptions{
        expires: None,
        max_age: Some(3600),
        is_public: None,
        no_cache: None,
        no_store: None
    });
    let is_public_no_cache_no_store =  Arc::new(CCOptions {
        expires: None,
        max_age: None,
        is_public: Some(true),
        no_cache: Some(()),
        no_store: Some(())
    });
    rocket::build()
        .mount("/files", CCFileServer::new("test_dir/", options, Options::default()))
        .mount("/max-age", CCFileServer::new("test_dir/", max_age, Options::default()))
        .mount("/is_public_no_cache_no_store", CCFileServer::new("test_dir/", is_public_no_cache_no_store, Options::default()))
}

#[test]
fn simple_example() {
    let client = Client::tracked(rocket()).expect("valid rocket instance");
    let mut response = client.get(uri!("/files/test_file")).dispatch();
    assert_eq!(response.status(), Status::Ok);
    assert_eq!(response.headers().get_one("Cache-Control"), Some("public, no-cache, max-age=300;"));
    assert_eq!(response.into_string().unwrap(), "1234asdf");
}

#[test]
fn max_age() {
    let client = Client::tracked(rocket()).expect("valid rocket instance");
    let mut response = client.get(uri!("/max-age/test_file")).dispatch();
    assert_eq!(response.status(), Status::Ok);
    assert_eq!(response.headers().get_one("Cache-Control"), Some("max-age=3600;"));
    assert_eq!(response.into_string().unwrap(), "1234asdf");
}

#[test]
fn is_public_no_cache_no_store() {
    let client = Client::tracked(rocket()).expect("valid rocket instance");
    let mut response = client.get(uri!("/is_public_no_cache_no_store/test_file")).dispatch();
    assert_eq!(response.status(), Status::Ok);
    assert_eq!(response.headers().get_one("Cache-Control"), Some("public, no-cache, no-store;"));
    assert_eq!(response.into_string().unwrap(), "1234asdf");
}