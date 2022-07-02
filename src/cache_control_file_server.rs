use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use httpdate::HttpDate;

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
#[derive(Debug, Clone)]
pub struct CCFileServer {
    root: PathBuf,
    cc_options: Arc<CCOptions>,
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
    pub fn new<P: AsRef<Path>>(path: P, cc_options: Arc<CCOptions>, options: Options) -> Self {
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

        CCFileServer { root: path.into(), cc_options, options, rank: Self::DEFAULT_RANK }
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
            }
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
            },
            Some(p) => Outcome::from_or_forward(req, data, CCNamedFileWrapper(NamedFile::open(p).await.ok(), self.cc_options.clone())),
            None => Outcome::forward(data),
        }
    }
}

struct CCNamedFileWrapper(Option<NamedFile>, Arc<CCOptions>);

impl<'r> Responder<'r, 'static> for CCNamedFileWrapper {
    fn respond_to(self, req: &'r Request<'_>) -> rocket::response::Result<'static> {
        let mut response = self.0.respond_to(req)?;

        let config = self.1.deref();

        if let Some(e) = config.expires {
            println!("{}", e);
            response.set_header(Header::new("Expires", e.to_string()));
        }

        //let mut header_map = HeaderMap::new();

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

/// A Struct representing multiple optional CacheControl-headers for the `CCFileServer`
///
/// If an Option is set to None, that header will not be set. If it is set to Some, a
/// header will be set and will be populated with a value if available.
///
#[derive(Debug, Clone)]
pub struct CCOptions {
    pub expires: Option<HttpDate>, //Todo let user provide function to calculate expires date
    pub max_age: Option<u32>,
    pub is_public: Option<bool>,
    pub no_cache: Option<()>,
    pub no_store: Option<()>,
}