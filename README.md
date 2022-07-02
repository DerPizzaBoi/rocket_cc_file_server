# Rocket cache control FileServer

A custom implementation of the Rocket FileServer, that supports headers for cache control.
CCFileServer can be used as a drop in replacement for Rocket's FileServer.

It is this easy to use:
```rust
#[launch]
fn rocket() -> Rocket<Build> {
    let options = Arc::new(CCOptions {
        expires: None,
        max_age: None,
        is_public: Some(true),
        no_cache: Some(()),
        no_store: None
    });
    rocket::build()
        .mount("/assets", CCFileServer::from("www/public/assets"))
        .mount("/js", CCFileServer::new("www/public/js", options))
}
```

## Usage

Mount the CCFileServer the same way you mount a normal rocket FileServer.

Additionally, you need to provide a CCOptions struct to configure the headers of the CCFileServer.
Every field that has *Some* value will be set as a header.

## TODO's for 1.0.0

Right now it is just a prototype. There are still some things to do:
pub max_age: Option<u32>,
pub is_public: Option<bool>,
pub no_cache: Option<()>,
pub no_store: Option<()>,
- [x] Port the rocket Options
- [ ] Implement all caching options
  - [x] max-age header
  - [x] public/private, no_cache and no_store
  - [ ] Provide optional function to calculate expires date
  - [ ] Implement E-tags