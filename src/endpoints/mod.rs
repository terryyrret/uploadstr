use poem::{handler, http::StatusCode, Error, Result};

mod filesystem;
use filesystem::FS;

use crate::config::Config;
use crate::config::Ops;
use crate::nostr_auth::check::Auth;
use crate::nostr_auth::parse::get_tag;
use crate::nostr_auth::parse::NostrAuth;

use nostr::HttpMethod;
use serde_json::to_string;

fn print_result(f: impl FnOnce() -> Result<String>) -> Result<String> {
    let result = f();
    println!("{result:#?}");
    result
}

#[handler]
pub fn delete_file(NostrAuth(event): NostrAuth) -> Result<String> {
    print_result(|| {
        Auth::new().auth(&event, HttpMethod::POST, "/delete")?;

        let filename = get_tag(&event, "filename")?.ok_or_else(|| {
            Error::from_string(
                "There is no filename tag specified.",
                StatusCode::BAD_REQUEST,
            )
        })?;
        let files_dir = Config::new().get_config_value("filesDir")?;

        FS::new()
            .delete_file(&files_dir, &filename)
            .map(|()| format!("Successfully deleted {filename}"))
    })
}

#[handler]
#[allow(clippy::needless_pass_by_value)]
pub fn upload_file(NostrAuth(event): NostrAuth, data: Vec<u8>) -> Result<String> {
    print_result(|| {
        let filename = Auth::new().file_auth(&event, &data)?;
        let config = Config::new();
        let base_url = config.get_config_value("baseUrl")?;
        let files_dir = config.get_config_value("filesDir")?;

        FS::new()
            .save_file(&files_dir, &filename, &data)
            .map(|()| format!("{base_url}/f/{filename}"))
    })
}

#[handler]
#[allow(clippy::needless_pass_by_value)]
pub fn list_files(NostrAuth(event): NostrAuth) -> Result<String> {
    print_result(|| {
        Auth::new().auth(&event, HttpMethod::GET, "/list")?;

        let config = Config::new();
        let base_url = config.get_config_value("baseUrl")?;
        let files_dir = config.get_config_value("filesDir")?;
        let filenames = FS::new().get_files(&files_dir)?;
        let urls = filenames
            .into_iter()
            .map(|filename| format!("{base_url}/f/{filename}"))
            .collect::<Vec<String>>();

        to_string(&urls).map_err(|_| {
            Error::from_string(
                "Failed to convert reply to JSON",
                StatusCode::INTERNAL_SERVER_ERROR,
            )
        })
    })
}
