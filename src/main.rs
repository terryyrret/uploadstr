use poem::{get, post, listener::TcpListener,
           Route, Server,
           endpoint::StaticFilesEndpoint};
use poem::EndpointExt;
use poem::middleware::Cors;

mod nostr_auth;
mod endpoints;
mod config;

use endpoints::upload_file;
use endpoints::list_files;
use endpoints::delete_file;
use config::get_config_value;

// Use Jemalloc only for musl-64 bits platforms
#[cfg(all(target_env = "musl", target_pointer_width = "64"))]
#[global_allocator]
static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;


#[tokio::main]
async fn main() {
    let files_dir = get_config_value("filesDir").unwrap();
    let bind = get_config_value("bind").unwrap();

    let app = Route::new()
        .at("/upload", post(upload_file))
        .at("/list", get(list_files))
        .at("/delete", post(delete_file))
        .nest(
            "/f",
            StaticFilesEndpoint::new(files_dir)
        )
        .with(Cors::new().allow_credentials(true));

    Server::new(TcpListener::bind(bind))
      .run(app)
      .await
      .expect("To just run...")
}
