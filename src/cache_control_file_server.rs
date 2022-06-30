use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use httpdate::HttpDate;

use rocket::{Data, Request};
use rocket::fs::NamedFile;
use rocket::http::{ContentType, Header, HeaderMap, Method};
use rocket::http::uri::Segments;
use rocket::response::Responder;
use rocket::route::{Handler, Outcome, Route};
use rocket::tokio::fs::File;

#[derive(Debug, Clone)]
pub struct CCFileServer {
    root: PathBuf,
    options: Arc<CCOptions>,
    rank: isize,
}

impl CCFileServer {
    /// The default rank. Same as rocket `FileServer`
    const DEFAULT_RANK: isize = 10;

    #[track_caller]
    pub fn new<P: AsRef<Path>>(path: P, options: Arc<CCOptions>) -> Self {

        //TODO implement rocket FileServer Options

        CCFileServer { root: path.as_ref().into(), options, rank: Self::DEFAULT_RANK }
    }

    //TODO implement default options
    /*    pub fn from<P: AsRef<Path>>(path: P) -> Self {
            CCFileServer::new(path, CCOptions::default())
        }*/

    pub fn rank(mut self, rank: isize) -> Self {
        self.rank = rank;
        self
    }
}

#[rocket::async_trait]
impl Handler for CCFileServer {
    async fn handle<'r>(&self, req: &'r Request<'_>, data: Data<'r>) -> Outcome<'r> {
        use rocket::http::uri::fmt::Path;

        let allow_dotfiles = false;
        let path = req.segments::<Segments<'_, Path>>(0..).ok()
            .and_then(|segments| segments.to_path_buf(allow_dotfiles).ok())
            .map(|path| self.root.join(path));

        match path {
            Some(p) => Outcome::from_or_forward(req, data, CCNamedFileWrapper(NamedFile::open(p).await.ok(), self.options.clone())),
            None => Outcome::forward(data)
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

        let mut header_map = HeaderMap::new();

        let mut cache_control = String::new();
        if let Some(a) = config.is_public {
            if a { header_map.add(Header::new("Cache-Control", "public")); cache_control.push_str("public") } else { cache_control.push_str("private") }
        }
        if config.no_cache.is_some() {
            if !cache_control.is_empty() { cache_control.push_str(", ") }
            cache_control.push_str("no-cache");
        }
        if config.no_store.is_some() {
            if !cache_control.is_empty() { cache_control.push_str(", ") }
            cache_control.push_str("no-store");
        }
        if let Some(age) = config.max_age {
            if !cache_control.is_empty() { cache_control.push_str(", ") }
            cache_control.push_str("max-age=");
            cache_control.push_str(&age.to_string());
        }

        cache_control.push(';');
        response.set_header(Header::new("Cache-Control", cache_control));
        Ok(response)
    }
}

impl From<CCFileServer> for Vec<Route> {
    fn from(server: CCFileServer) -> Self {
        let source = rocket::figment::Source::File(server.root.clone());
        let mut route = Route::ranked(server.rank, Method::Get, "/<path..>", server);
        route.name = Some(format!("FileServer: {}", source).into());
        vec![route]
    }
}

#[derive(Debug, Clone)]
pub struct CCOptions {
    pub expires: Option<HttpDate>,
    pub max_age: Option<u32>,
    pub is_public: Option<bool>,
    pub no_cache: Option<()>,
    pub no_store: Option<()>,
}