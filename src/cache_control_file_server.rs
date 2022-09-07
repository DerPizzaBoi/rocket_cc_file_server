use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use rocket::{Data, error, Request, warn_};
use rocket::fs::{NamedFile, Options};
use rocket::http::{Header, Method};
use rocket::http::ext::IntoOwned;
use rocket::http::uri::Segments;
use rocket::response::{Redirect, Responder};
use rocket::route::{Handler, Outcome, Route};

/// Custom handler for serving static files.
///
/// This handler servers the same purpose as the normal rocket `FileSever`, but it
/// additionally provides Options for cache-control headers.
///
/// If you want to reuse a `CCOptions` instance for multiple `CCFileServers`, you can
/// simply clone it. Cloning `CCOptions` is cheap as it is simply a reference-counted
/// handle to the inner `CCOptions` state.
#[derive(Clone)]
pub struct CCFileServer {
    root: PathBuf,
    cc_options: Arc<CCOptionsInner>,
    options: Options,
    rank: isize,
}

impl CCFileServer {
    /// The default rank. Same as rocket `FileServer`
    const DEFAULT_RANK: isize = 10;

    /// Constructs a new `FileServer` that serves files from the file system
    /// `path` with `cc_options` and `options` enabled.
    ///
    // Copyright 2016 Sergio Benitez
    // Adapted from the `Rocket`-framework's FileServer implementation.
    // src: https://github.com/SergioBenitez/Rocket/blob/b6448fc01629c02196a439075db4d09d5c7b2091/core/lib/src/fs/server.rs line 143-163
    #[track_caller]
    pub fn new<P: AsRef<Path>>(path: P, cc_options: CCOptions, options: Options) -> Self {
        use rocket::yansi::Paint;

        let path = path.as_ref();
        if !options.contains(Options::Missing) {
            if !options.contains(Options::IndexFile) && !path.is_dir() {
                let path = path.display();
                error!("FileServer path '{}' is not a directory.", Paint::white(path));
                warn_!("Aborting early to prevent inevitable handler failure.");
                panic!("invalid directory: refusing to continue");
            } else if !path.exists() {
                let path = path.display();
                error!("FileServer path '{}' is not a file.", Paint::white(path));
                warn_!("Aborting early to prevent inevitable handler failure.");
                panic!("invalid file: refusing to continue");
            }
        }

        CCFileServer { root: path.into(), cc_options: cc_options.0, options, rank: Self::DEFAULT_RANK }
    }

    pub fn rank(mut self, rank: isize) -> Self {
        self.rank = rank;
        self
    }
}

// Copyright 2016 Sergio Benitez
// Adapted from the `Rocket`-framework's FileServer implementation.
// src: https://github.com/SergioBenitez/Rocket/blob/b6448fc01629c02196a439075db4d09d5c7b2091/core/lib/src/fs/server.rs line 143-163
#[rocket::async_trait]
impl Handler for CCFileServer {
    async fn handle<'r>(&self, req: &'r Request<'_>, data: Data<'r>) -> Outcome<'r> {
        use rocket::http::uri::fmt::Path;

        // TODO: Should we reject dotfiles for `self.root` if !DotFiles?
        let options = self.options;
        if options.contains(Options::IndexFile) && self.root.is_file() {
            let segments = match req.segments::<Segments<'_, Path>>(0..) {
                Ok(segments) => segments,
                Err(never) => match never {},
            };

            return if segments.is_empty() {
                let file = CCNamedFileWrapper(NamedFile::open(&self.root).await.ok(), self.cc_options.clone());
                Outcome::from_or_forward(req, data, file)
            } else {
                Outcome::forward(data)
            };
        }

        // Get the segments as a `PathBuf`, allowing dotfiles requested.
        let allow_dotfiles = options.contains(Options::DotFiles);
        let path = req.segments::<Segments<'_, Path>>(0..).ok()
            .and_then(|segments| segments.to_path_buf(allow_dotfiles).ok())
            .map(|path| self.root.join(path));

        match path {
            Some(p) if p.is_dir() => {
                // Normalize '/a/b/foo' to '/a/b/foo/'.
                if options.contains(Options::NormalizeDirs) && !req.uri().path().ends_with('/') {
                    let normal = req.uri().map_path(|p| format!("{}/", p))
                        .expect("adding a trailing slash to a known good path => valid path")
                        .into_owned();

                    return Outcome::from_or_forward(req, data, Redirect::permanent(normal));
                }

                if !options.contains(Options::Index) {
                    return Outcome::forward(data);
                }

                //TODO should index.html files for directories be cached?
                let index = CCNamedFileWrapper(NamedFile::open(p.join("index.html")).await.ok(), self.cc_options.clone());
                Outcome::from_or_forward(req, data, index)
            }
            Some(p) => Outcome::from_or_forward(req, data, CCNamedFileWrapper(NamedFile::open(p).await.ok(), self.cc_options.clone())),
            None => Outcome::forward(data),
        }
    }
}

struct CCNamedFileWrapper(Option<NamedFile>, Arc<CCOptionsInner>);

impl<'r> Responder<'r, 'static> for CCNamedFileWrapper {
    fn respond_to(self, req: &'r Request<'_>) -> rocket::response::Result<'static> {
        let file_path = match self.0 {
            Some(ref r) => r.path().to_path_buf(),
            //Creating non-existing path, because the file is none. When calling the NamedFiles'
            // *respond_to*, this function will return with an Error(because the file does not exist)
            // and the path won't be use. The *expires_non_existing_file* test proves this.
            None => PathBuf::new(),
        };

        //After this, NamedFile should be Some
        let mut response = self.0.respond_to(req)?;

        let config = self.1.deref();

        //let mut header_map = HeaderMap::new();

        if let Some(fun) = config.expires {
            let expires = (fun)(file_path.as_path());
            response.set_header(Header::new("Expires", expires));
        }

        let mut cache_control = String::new();
        if let Some(a) = config.is_public {
            if a {
                //header_map.add(Header::new("Cache-Control", "public"));
                cache_control.push_str("public")
            } else {
                //header_map.add(Header::new("Cache-Control", "private"));
                cache_control.push_str("private")
            }
        }
        if config.no_cache.is_some() {
            if !cache_control.is_empty() { cache_control.push_str(", ") }
            //header_map.add(Header::new("Cache-Control", "no-cache"));
            cache_control.push_str("no-cache");
        }
        if config.no_store.is_some() {
            if !cache_control.is_empty() { cache_control.push_str(", ") }
            //header_map.add(Header::new("Cache-Control", "no-store"));
            cache_control.push_str("no-store");
        }
        if let Some(age) = config.max_age {
            if !cache_control.is_empty() { cache_control.push_str(", ") }
            //header_map.add(Header::new("Cache-Control", format!("max-age={}", &age)));
            cache_control.push_str("max-age=");
            cache_control.push_str(&age.to_string());
        }

        //todo replace string with headermap?

        cache_control.push(';');
        response.set_header(Header::new("Cache-Control", cache_control));
        Ok(response)
    }
}

// Copyright 2016 Sergio Benitez
// Copied from the `Rocket`-framework's FileServer implementation.
// src: https://github.com/SergioBenitez/Rocket/blob/b6448fc01629c02196a439075db4d09d5c7b2091/core/lib/src/fs/server.rs line 184-191
impl From<CCFileServer> for Vec<Route> {
    fn from(server: CCFileServer) -> Self {
        let source = rocket::figment::Source::File(server.root.clone());
        let mut route = Route::ranked(server.rank, Method::Get, "/<path..>", server);
        route.name = Some(format!("FileServer: {}", source).into());
        vec![route]
    }
}

pub struct CCOptionsBuilder(CCOptionsInner);

pub struct CCOptions(pub(crate) Arc<CCOptionsInner>);

impl CCOptionsBuilder {
    /// Creates a new CCOptions struct directly. It is better to use the build patter, as this function makes code hard to read and understand
    pub fn new(expires: Option<fn(&Path) -> String>, max_age: Option<u32>, is_public: Option<bool>, no_cache: Option<()>, no_store: Option<()>) -> CCOptionsBuilder {
        let inner = CCOptionsInner {
            expires,
            max_age,
            is_public,
            no_cache,
            no_store,
        };
        CCOptionsBuilder(inner)
    }

    pub fn builder() -> CCOptionsBuilder {
        CCOptionsBuilder(CCOptionsInner::default())
    }

    pub fn expires(mut self, expires: Option<fn(&Path) -> String>) -> CCOptionsBuilder {
        self.0.expires = expires;
        self
    }

    pub fn max_age(mut self, max_age: Option<u32>) -> CCOptionsBuilder {
        self.0.max_age = max_age;
        self
    }

    pub fn is_public(mut self, is_public: Option<bool>) -> CCOptionsBuilder {
        self.0.is_public = is_public;
        self
    }

    pub fn no_cache(mut self, no_cache: Option<()>) -> CCOptionsBuilder {
        self.0.no_cache = no_cache;
        self
    }

    pub fn no_store(mut self, no_store: Option<()>) -> CCOptionsBuilder {
        self.0.no_store = no_store;
        self
    }

    pub fn build(self) -> CCOptions {
        CCOptions(Arc::new(self.0))
    }

    pub fn clear(mut self) -> CCOptionsBuilder {
        self.0 = CCOptionsInner::default();
        self
    }
}

impl Clone for CCOptions {
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}

/// A Struct representing multiple optional CacheControl-headers for the `CCFileServer`
///
/// If an Option is set to None, that header will not be set. If it is set to Some, a
/// header will be set and will be populated with a value if available.
#[derive(Clone, Default)]
pub(crate) struct CCOptionsInner {
    /// The function that is used to calculate the expiry date. Its current parameter is the
    /// requested files' path to set file/directory specific dates. Create an issue if you have other parameter ideas
    pub expires: Option<fn(&Path) -> String>,
    pub max_age: Option<u32>,
    pub is_public: Option<bool>,
    pub no_cache: Option<()>,
    pub no_store: Option<()>,
}