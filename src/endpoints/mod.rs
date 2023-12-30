use poem::{handler, http::StatusCode, Error, Result};

mod filesystem;
use filesystem::FS;

use crate::config::get_config_value;
use crate::nostr_auth::check::auth;
use crate::nostr_auth::check::file_auth;
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
        auth(&event, HttpMethod::POST, "/delete")?;

        get_tag(&event, "filename")
            .and_then(|maybe_tag| {
                maybe_tag.ok_or_else(|| {
                    Error::from_string(
                        "There is no filename tag specified.",
                        StatusCode::BAD_REQUEST,
                    )
                })
            })
            .and_then(|filename| {
                get_config_value("filesDir").and_then(|files_dir| {
                    FS::new()
                        .delete_file(&files_dir, &filename)
                        .map(|()| format!("Successfully deleted {filename}"))
                })
            })
    })
}

#[handler]
#[allow(clippy::needless_pass_by_value)]
pub fn upload_file(NostrAuth(event): NostrAuth, data: Vec<u8>) -> Result<String> {
    print_result(|| {
        let filename = file_auth(&event, &data)?;
        let base_url = get_config_value("baseUrl")?;
        let files_dir = get_config_value("filesDir")?;

        FS::new()
            .save_file(&files_dir, &filename, &data)
            .map(|()| format!("{base_url}/f/{filename}"))
    })
}

#[handler]
#[allow(clippy::needless_pass_by_value)]
pub fn list_files(NostrAuth(event): NostrAuth) -> Result<String> {
    print_result(|| {
        auth(&event, HttpMethod::GET, "/list")?;

        get_config_value("baseUrl").and_then(|base_url| {
            get_config_value("filesDir").and_then(|files_dir| {
                FS::new()
                    .get_files(&files_dir)
                    .map(|list| {
                        list.into_iter()
                            .map(|filename| format!("{base_url}/f/{filename}"))
                            .collect::<Vec<String>>()
                    })
                    .and_then(|v| {
                        to_string(&v).map_err(|_| {
                            Error::from_string(
                                "Failed to convert reply to JSON",
                                StatusCode::INTERNAL_SERVER_ERROR,
                            )
                        })
                    })
            })
        })
    })
}
