use std::path::{Path};
use std::time::{Duration, UNIX_EPOCH};

use rocket::fs::{Options};
use rocket::{Build, Rocket, uri};

use rocket::launch;
use rocket::http::Status;
use rocket::local::blocking::Client;

use rocket_cc_file_server::{CCFileServer, CCOptionsBuilder};

#[test]
fn it_works() {
    assert_eq!(2 + 2, 4);
}

#[launch]
fn rocket() -> Rocket<Build> {
    let options = CCOptionsBuilder::builder()
        .max_age(Some(300))
        .is_public(Some(true))
        .no_cache(Some(()))
        .build();

    let max_age = CCOptionsBuilder::builder()
        .max_age(Some(3600))
        .build();

    let is_public_no_cache_no_store = CCOptionsBuilder::builder()
        .is_public(Some(true))
        .no_cache(Some(()))
        .no_store(Some(()))
        .build();

    let expires_closure = CCOptionsBuilder::builder()
        .expires(Some(
            |_path| -> String {
                httpdate::fmt_http_date(UNIX_EPOCH + Duration::from_millis(7_956_831_600_000))
            }
        ))
        .build();

    let expires_fun = CCOptionsBuilder::builder()
        .expires(Some(expires))
        .build();

    rocket::build()
        .mount("/files", CCFileServer::new("test_dir/", options, Options::default()))
        .mount("/max-age", CCFileServer::new("test_dir/", max_age, Options::default()))
        .mount("/is_public_no_cache_no_store", CCFileServer::new("test_dir/", is_public_no_cache_no_store, Options::default()))
        .mount("/expires_closure", CCFileServer::new("test_dir/", expires_closure, Options::default()))
        .mount("/expires_fun", CCFileServer::new("test_dir/", expires_fun, Options::default()))
}

fn expires(_path: &Path) -> String {
    httpdate::fmt_http_date(UNIX_EPOCH + Duration::from_millis(7_956_831_600_000))
}

#[test]
fn simple_example() {
    let client = Client::tracked(rocket()).expect("valid rocket instance");
    let response = client.get(uri!("/files/test_file")).dispatch();
    assert_eq!(response.status(), Status::Ok);
    assert_eq!(response.headers().get_one("Cache-Control"), Some("public, no-cache, max-age=300;"));
    assert_eq!(response.into_string().unwrap(), "1234asdf");
}

#[test]
fn max_age() {
    let client = Client::tracked(rocket()).expect("valid rocket instance");
    let response = client.get(uri!("/max-age/test_file")).dispatch();
    assert_eq!(response.status(), Status::Ok);
    assert_eq!(response.headers().get_one("Cache-Control"), Some("max-age=3600;"));
    assert_eq!(response.into_string().unwrap(), "1234asdf");
}

#[test]
fn is_public_no_cache_no_store() {
    let client = Client::tracked(rocket()).expect("valid rocket instance");
    let response = client.get(uri!("/is_public_no_cache_no_store/test_file")).dispatch();
    assert_eq!(response.status(), Status::Ok);
    assert_eq!(response.headers().get_one("Cache-Control"), Some("public, no-cache, no-store;"));
    assert_eq!(response.into_string().unwrap(), "1234asdf");
}

#[test]
fn expires_closure() {
    let client = Client::tracked(rocket()).expect("valid rocket instance");
    let response = client.get(uri!("/expires_closure/test_file")).dispatch();
    assert_eq!(response.status(), Status::Ok);
    assert_eq!(response.headers().get_one("Expires"), Some("Thu, 21 Feb 2222 23:00:00 GMT"));
    assert_eq!(response.into_string().unwrap(), "1234asdf");
}

#[test]
fn expires_fun() {
    let client = Client::tracked(rocket()).expect("valid rocket instance");
    let response = client.get(uri!("/expires_fun/test_file")).dispatch();
    assert_eq!(response.status(), Status::Ok);
    assert_eq!(response.headers().get_one("Expires"), Some("Thu, 21 Feb 2222 23:00:00 GMT"));
    assert_eq!(response.into_string().unwrap(), "1234asdf");
}

#[test]
fn expires_non_existing_file() {
    let client = Client::tracked(rocket()).expect("valid rocket instance");
    let mut response = client.get(uri!("/expires_fun/not_existing")).dispatch();
    assert_eq!(response.status(), Status::NotFound);
    assert_eq!(response.headers().get_one("Expires"), None);

    response = client.get(uri!("/expires_fun/not_existing_2")).dispatch();
    assert_eq!(response.status(), Status::NotFound);
    assert_eq!(response.headers().get_one("Expires"), None);

    response = client.get(uri!("/expires_closure/not_existing")).dispatch();
    assert_eq!(response.status(), Status::NotFound);
    assert_eq!(response.headers().get_one("Expires"), None);

    response = client.get(uri!("/expires_closure/not_existing")).dispatch();
    assert_eq!(response.status(), Status::NotFound);
    assert_ne!(response.headers().get_one("Expires"), Some("Thu, 21 Feb 2222 23:00:00 GMT"));
}