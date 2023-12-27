use poem::{RequestBody, Request, Result, http::StatusCode, Error, FromRequest};
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use nostr::event::Event;
use nostr::prelude::Value;
use nostr::prelude::core::str::FromStr;
use nostr::TagKind;
use nostr::Tag;

pub struct NostrAuth(Event);

impl NostrAuth {
    pub fn get_event(&self) -> &Event {
        &self.0
    }
}

#[poem::async_trait]
impl<'a> FromRequest<'a> for NostrAuth {
    async fn from_request(request: &'a Request, _: &mut RequestBody) -> Result<Self> {
        request
            .header("authorization")
            .ok_or_else(|| Error::from_string(
                    "Missing authorization header",
                    StatusCode::BAD_REQUEST
            ))
            .and_then(|auth|
                if auth[0.."Nostr ".len()] != String::from("Nostr ") {
                    Err(Error::from_string(
                        "The authorization scheme is not Nostr",
                        StatusCode::BAD_REQUEST
                    ))
                } else {
                    Ok(&auth["Nostr ".len()..])
                }
            )
            .and_then(|b64_event|
                STANDARD.decode(b64_event)
                    .map_err(|_| Error::from_string(
                        "Could not decode Base64 string into bytes",
                        StatusCode::BAD_REQUEST
                    ))
            )
            .and_then(|decoded|
                String::from_utf8(decoded)
                    .map_err(|_| Error::from_string(
                        "Could not decode Base64 bytes into a utf-8 string",
                        StatusCode::BAD_REQUEST
                    ))
            )
            .and_then(|string|
                Value::from_str(&string)
                    .map_err(|_| Error::from_string(
                        "Could not parse decoded Base64 string into JSON",
                        StatusCode::BAD_REQUEST
                    ))
            )
            .and_then(|value|
                Event::from_value(value)
                    .map_err(|_| Error::from_string(
                        "Could not parse JSON into nostr event",
                        StatusCode::BAD_REQUEST
                    ))
            )
            .map(|event|
                NostrAuth(event)
            )
    }
}

pub fn get_filename(event: &Event) -> Result<String> {
    let mut filename_tags = event.tags.iter().filter_map(|tag| match tag {
        Tag::Generic(TagKind::Custom(key), values) if key == "filename" => Some(values),
        _ => None,
    });


    if let Some(filenames) = filename_tags.nth(0) {
        if let Some(_) = filename_tags.nth(0) {
            Err(Error::from_string(
                "There are multiple filename tags.",
                StatusCode::BAD_REQUEST,
            ))
        } else if filenames.len() != 1 {
            Err(Error::from_string(
                "The list of filenames does not have exactly one element.",
                StatusCode::BAD_REQUEST,
            ))
        } else {
            Ok(
                filenames
                .get(0)
                .expect("There should be one element because of the previous if-statement.")
                .to_string()
            )
        }
    } else {
        Err(Error::from_string(
            "Either the list of filenames is empty or has more than one element.",
            StatusCode::BAD_REQUEST,
        ))
    }
}
