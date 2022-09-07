use rocket::{Build, Rocket, launch};
use rocket::fs::Options;
use rocket_cc_file_server::{CCFileServer, CCOptionsBuilder};

// Look at the tests for an executable example
#[launch]
fn rocket() -> Rocket<Build> {
    //create an option configuration, which will be used to set the headers of the served files
    let options = CCOptionsBuilder::builder()
        .max_age(Some(300))
        .is_public(Some(true))
        .no_cache(Some(()))
        .build();

    let js_options = CCOptionsBuilder::builder()
        .max_age(Some(6000))
        .is_public(Some(false))
        .build();

    rocket::build()
        //.attach(...)  //setup your rocket instance as usual
        //.mount(...)
        .mount("/assets", CCFileServer::new("assets/", options.clone(), Options::default())) //simply add this to create a FileServer with the options provided
        .mount("/img", CCFileServer::new("img/", options.clone(), Options::default())) //Options reusable for multiple FileServer
        .mount("/js", CCFileServer::new("js/", js_options, Options::default())) //You can also use different options for different filetypes/directories
}