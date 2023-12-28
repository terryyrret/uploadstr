use poem::{handler, http::StatusCode, Error, Result};

mod filesystem;
use filesystem::delete_file as del_file;
use filesystem::get_files;
use filesystem::save_file;

use crate::config::get_config_value;
use crate::nostr_auth::check::check_auth;
use crate::nostr_auth::check::check_file_auth;
use crate::nostr_auth::parse::get_tag;
use crate::nostr_auth::parse::NostrAuth;

use nostr::HttpMethod;
use serde_json::to_string;

fn print_result(f: impl FnOnce() -> Result<String>) -> Result<String> {
    let result = f();
    println!("{:#?}", result);
    return result;
}

#[handler]
pub fn delete_file(auth: NostrAuth) -> Result<String> {
    print_result(|| {
        check_auth(&auth.get_event(), HttpMethod::POST, "/delete")?;

        get_tag(&auth.get_event(), "filename")
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
                    del_file(&files_dir, &filename)
                        .map(|_| format!("Successfully deleted {}", filename))
                        .map_err(|_| {
                            Error::from_string(
                                "Could not delete file from server...",
                                StatusCode::INTERNAL_SERVER_ERROR,
                            )
                        })
                })
            })
    })
}

#[handler]
pub fn upload_file(auth: NostrAuth, data: Vec<u8>) -> Result<String> {
    print_result(|| {
        check_file_auth(&auth.get_event(), &data).and_then(|filename| {
            get_config_value("baseUrl").and_then(|base_url| {
                get_config_value("filesDir").and_then(|files_dir| {
                    save_file(&files_dir, &filename, &data)
                        .map(|_| format!("{}/f/{}", base_url, filename))
                        .map_err(|_| {
                            Error::from_string(
                                "Could not save file to server.",
                                StatusCode::INTERNAL_SERVER_ERROR,
                            )
                        })
                })
            })
        })
    })
}

#[handler]
pub fn list_files(auth: NostrAuth) -> Result<String> {
    print_result(|| {
        check_auth(&auth.get_event(), HttpMethod::GET, "/list")?;

        get_config_value("baseUrl").and_then(|base_url| {
            get_config_value("filesDir").and_then(|files_dir| {
                get_files(&files_dir)
                    .map_err(|_| {
                        Error::from_string(
                            "Failed to get list of files",
                            StatusCode::INTERNAL_SERVER_ERROR,
                        )
                    })
                    .map(|list| {
                        list.into_iter()
                            .map(|filename| format!("{}/f/{}", base_url, filename))
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
