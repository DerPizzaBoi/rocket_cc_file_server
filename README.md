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

Right now it is just a prototype. There are still some things to do:

- [ ] Port the rocket Options
- [ ] Implement all caching options