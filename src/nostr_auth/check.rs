use nostr::event::kind::Kind;
use nostr::event::tag::HttpMethod;
use nostr::event::tag::Tag;

use nostr::event::Event;
use nostr::prelude::key::XOnlyPublicKey;
use nostr::types::time::Timestamp;
use nostr::UncheckedUrl;
use poem::{http::StatusCode, Error, Result};
use std::ops::Add;
use std::str::FromStr;

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use crypto::digest::Digest;
use crypto::sha2::Sha256;

use crate::config::get_config_value;
use crate::config::get_config_values;
use crate::nostr_auth::parse::get_tag;

pub fn check_file_auth(event: &Event, data: &[u8]) -> Result<String> {
    check_auth(event, HttpMethod::POST, "/upload")?;

    let mut payloads = event.tags.iter().filter_map(|tag| match tag {
        Tag::Payload(hash) => Some(hash),
        _ => None,
    });

    let payload = payloads.next();

    if payload.is_some() && payloads.next().is_some() {
        return Err(Error::from_string(
            "There are two payload hashes.",
            StatusCode::UNAUTHORIZED,
        ));
    }

    if payload.is_none() {
        return Err(Error::from_string(
            "The payload hash is missing.",
            StatusCode::UNAUTHORIZED,
        ));
    }

    let actual_hash = payload.unwrap();

    let mut hasher = Sha256::new();
    let mut sha256_hash: [u8; 32] = [0; 32];

    hasher.input(data);
    hasher.result(&mut sha256_hash);

    if actual_hash.as_ref() != sha256_hash {
        return Err(Error::from_string(
            "The given SHA256 hash does not match with the given binary data",
            StatusCode::UNAUTHORIZED,
        ));
    }

    let b64 = URL_SAFE_NO_PAD.encode(sha256_hash);

    let filename = if let Some(ext) = get_tag(event, "ext")? {
        format!("{}.{}", b64, ext)
    } else {
        b64.to_string()
    };

    Ok(filename)
}

pub fn check_auth(event: &Event, method: HttpMethod, path: &str) -> Result<()> {
    event.verify().map_err(|_| {
        Error::from_string(
            "EventId/Signature are not valid...",
            StatusCode::UNAUTHORIZED,
        )
    })?;

    let whitelisted_pubkeys = get_config_values("pubkeyWhitelist")?
        .into_iter()
        .flat_map(|pubkey| XOnlyPublicKey::from_str(&pubkey))
        .collect::<Vec<XOnlyPublicKey>>();

    if !whitelisted_pubkeys.contains(&event.pubkey) {
        return Err(Error::from_string(
            "Given pubkey is not in list of whitelisted pubkeys.",
            StatusCode::UNAUTHORIZED,
        ));
    }

    if event.kind != Kind::HttpAuth {
        return Err(Error::from_string(
            "Incorrect kind. Must be 27235.",
            StatusCode::UNAUTHORIZED,
        ));
    }

    if event.created_at.add(60_i64) < Timestamp::now() {
        return Err(Error::from_string(
            "Event was not created within the past 60 seconds.",
            StatusCode::UNAUTHORIZED,
        ));
    }

    if event.created_at > Timestamp::now() {
        return Err(Error::from_string(
            "This event is from the future...",
            StatusCode::UNAUTHORIZED,
        ));
    }

    let mut urls = event.tags.iter().filter_map(|tag| match tag {
        Tag::AbsoluteURL(url) => Some(url),
        _ => None,
    });

    let base_url = get_config_value("baseUrl")?;

    if Some(UncheckedUrl::from(format!("{}{}", base_url, path))) != urls.next().cloned()
        || urls.next().is_some()
    // Checks for multiple u tags...
    {
        return Err(Error::from_string(
            "The u tag is not the correct url or there are multiple u tags.",
            StatusCode::UNAUTHORIZED,
        ));
    }

    let mut methods = event.tags.iter().filter_map(|tag| match tag {
        Tag::Method(method) => Some(method),
        _ => None,
    });

    if Some(method) != methods.next().cloned() || methods.next().is_some()
    // Checks for multiple method tags...
    {
        return Err(Error::from_string(
            "The method tag is not what was used or there are multiple method tags",
            StatusCode::UNAUTHORIZED,
        ));
    }

    Ok(())
}
