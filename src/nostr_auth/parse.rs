use poem::{RequestBody, Request, Result, http::StatusCode, Error, FromRequest};
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use nostr::event::Event;
use nostr::prelude::Value;
use nostr::prelude::core::str::FromStr;
use nostr::TagKind;
use nostr::Tag;

pub struct NostrAuth(pub Event);

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
                if auth[0.."Nostr ".len()] == *"Nostr " {
                    Ok(&auth["Nostr ".len()..])
                } else {
                    Err(Error::from_string(
                        "The authorization scheme is not Nostr",
                        StatusCode::BAD_REQUEST
                    ))
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
            .map(NostrAuth
            )
    }
}

pub fn get_tag(event: &Event, tag_name: &str) -> Result<Option<String>> {
    let mut tags = event.tags.iter().filter_map(|tag| match tag {
        Tag::Generic(TagKind::Custom(key), values) if key == tag_name => Some(values),
        _ => None,
    });


    if let Some(values) = tags.next() {
        if tags.next().is_some() {
            Err(Error::from_string(
                format!("There are multiple {tag_name} tags."),
                StatusCode::BAD_REQUEST,
            ))
        } else if values.len() != 1 {
            Err(Error::from_string(
                format!("The {tag_name} tag does not have exactly one element"),
                StatusCode::BAD_REQUEST,
            ))
        } else {
            Ok(Some(
                values.first()
                .expect("There should be one element because of the previous if-statement.")
                .to_string()
            ))
        }
    } else {
        Ok(None)
    }
}

#[cfg(test)]
mod test_get_tag {
    use super::*;

    use nostr::EventBuilder;
    use nostr::Kind;
    use nostr::HttpMethod;
    use nostr::Keys;
    use nostr::key::FromSkStr;

    #[test]
    fn should_correctly_get_tag_from_good_event_1() {
        let private = "cb35da74d8d37ad5a2059d58e780cd0e160600ef62e3fd6a0399ebaf5b28695b";
        let key = Keys::from_sk_str(private).unwrap();
        let event = EventBuilder::new(
            Kind::HttpAuth,
            "",
            vec![
                Tag::Method(HttpMethod::POST),
                Tag::AbsoluteURL("https://domain.com/upload".into()),
                Tag::Generic(TagKind::Custom("ext".into()), vec!["w".into()])
            ]
        ).to_event(&key).unwrap();

        let tag = get_tag(&event, "ext").unwrap().unwrap();

        assert_eq!(tag, "w");
    }

    #[test]
    fn should_correctly_get_tag_from_good_event_2() {
        let private = "cb35da74d8d37ad5a2059d58e780cd0e160600ef62e3fd6a0399ebaf5b28695b";
        let key = Keys::from_sk_str(private).unwrap();
        let event = EventBuilder::new(
            Kind::HttpAuth,
            "",
            vec![
                Tag::Method(HttpMethod::POST),
                Tag::AbsoluteURL("https://domain.com/upload".into()),
                Tag::Generic(TagKind::Custom("1".into()), vec!["a".into()]),
                Tag::Generic(TagKind::Custom("2".into()), vec!["b".into()]),
                Tag::Generic(TagKind::Custom("3".into()), vec!["c".into()]),
                Tag::Generic(TagKind::Custom("4".into()), vec!["d".into()]),
            ]
        ).to_event(&key).unwrap();

        let tag = get_tag(&event, "1").unwrap().unwrap();

        assert_eq!(tag, "a");
    }

    #[test]
    fn should_correctly_get_tag_from_good_event_3() {
        let private = "cb35da74d8d37ad5a2059d58e780cd0e160600ef62e3fd6a0399ebaf5b28695b";
        let key = Keys::from_sk_str(private).unwrap();
        let event = EventBuilder::new(
            Kind::HttpAuth,
            "",
            vec![
                Tag::Method(HttpMethod::POST),
                Tag::AbsoluteURL("https://domain.com/upload".into()),
                Tag::Generic(TagKind::Custom("1".into()), vec!["a".into()]),
                Tag::Generic(TagKind::Custom("2".into()), vec!["b".into()]),
                Tag::Generic(TagKind::Custom("3".into()), vec!["c".into()]),
                Tag::Generic(TagKind::Custom("4".into()), vec!["d".into()]),
            ]
        ).to_event(&key).unwrap();

        let tag = get_tag(&event, "4").unwrap().unwrap();

        assert_eq!(tag, "d");
    }

    #[test]
    fn should_ok_when_asked_for_missing_tag() {
        let private = "cb35da74d8d37ad5a2059d58e780cd0e160600ef62e3fd6a0399ebaf5b28695b";
        let key = Keys::from_sk_str(private).unwrap();
        let event = EventBuilder::new(
            Kind::HttpAuth,
            "",
            vec![
                Tag::Method(HttpMethod::POST),
                Tag::AbsoluteURL("https://domain.com/upload".into()),
                Tag::Generic(TagKind::Custom("ext".into()), vec!["w".into()])
            ]
        ).to_event(&key).unwrap();

        let option = get_tag(&event, "does_not_exist").unwrap();

        assert!(option.is_none());
    }

    #[test]
    fn should_err_on_multi_of_same_tag() {
        let private = "cb35da74d8d37ad5a2059d58e780cd0e160600ef62e3fd6a0399ebaf5b28695b";
        let key = Keys::from_sk_str(private).unwrap();
        let event = EventBuilder::new(
            Kind::HttpAuth,
            "",
            vec![
                Tag::Method(HttpMethod::POST),
                Tag::AbsoluteURL("https://domain.com/upload".into()),
                Tag::Generic(TagKind::Custom("1".into()), vec!["a".into()]),
                Tag::Generic(TagKind::Custom("1".into()), vec!["b".into()]),
            ]
        ).to_event(&key).unwrap();

        let err = get_tag(&event, "1").unwrap_err();

        assert_eq!(err.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn should_err_on_multi_values_of_tag() {
        let private = "cb35da74d8d37ad5a2059d58e780cd0e160600ef62e3fd6a0399ebaf5b28695b";
        let key = Keys::from_sk_str(private).unwrap();
        let event = EventBuilder::new(
            Kind::HttpAuth,
            "",
            vec![
                Tag::Method(HttpMethod::POST),
                Tag::AbsoluteURL("https://domain.com/upload".into()),
                Tag::Generic(TagKind::Custom("1".into()), vec!["a".into(), "b".into()]),
            ]
        ).to_event(&key).unwrap();

        let err = get_tag(&event, "1").unwrap_err();

        assert_eq!(err.status(), StatusCode::BAD_REQUEST);
    }
}
