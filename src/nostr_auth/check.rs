use nostr::event::kind::Kind;
use nostr::event::tag::HttpMethod;
use nostr::event::tag::Tag;

use nostr::event::Event;
use nostr::prelude::key::XOnlyPublicKey;
use nostr::secp256k1::ThirtyTwoByteHash;
use nostr::types::time::Timestamp;
use nostr::UncheckedUrl;
use poem::{http::StatusCode, Error, Result};
use std::ops::Add;
use std::str::FromStr;

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;

use crate::config::Config;
use crate::config::Ops;
use crate::nostr_auth::parse::get_tag;

pub struct Auth<'a> {
    pub config: &'a dyn Ops,
}

impl Auth<'_> {
    pub fn new() -> Self {
        Self { config: &Config {} }
    }

    pub fn file_auth(&self, event: &Event, data: &[u8]) -> Result<String> {
        self.auth(event, HttpMethod::POST, "/upload")?;

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
        let sha256_hash = ring::digest::digest(&ring::digest::SHA256, data);

        if actual_hash.into_32() != sha256_hash.as_ref() {
            return Err(Error::from_string(
                "The given SHA256 hash does not match with the given binary data",
                StatusCode::UNAUTHORIZED,
            ));
        }

        let b64 = URL_SAFE_NO_PAD.encode(sha256_hash);

        let filename = if let Some(ext) = get_tag(event, "ext")? {
            format!("{b64}.{ext}")
        } else {
            b64.to_string()
        };

        Ok(filename)
    }

    pub fn auth(&self, event: &Event, method: HttpMethod, path: &str) -> Result<()> {
        event.verify().map_err(|_| {
            Error::from_string(
                "EventId/Signature are not valid...",
                StatusCode::UNAUTHORIZED,
            )
        })?;

        let whitelisted_pubkeys = self
            .config
            .get_config_values("pubkeyWhitelist")?
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

        let base_url = self.config.get_config_value("baseUrl")?;

        if Some(UncheckedUrl::from(format!("{base_url}{path}"))) != urls.next().cloned()
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
}

#[cfg(test)]
mod test_auth {
    use std::ops::Sub;

    use super::*;

    use crate::config::MockOps;
    use nostr::event::builder::EventBuilder;
    use nostr::key::FromSkStr;
    use nostr::types::time::Timestamp;
    use nostr::{EventId, Keys};

    #[test]
    fn should_ok_on_good_auth() {
        let private = "cb35da74d8d37ad5a2059d58e780cd0e160600ef62e3fd6a0399ebaf5b28695b";
        let public = "4344e9cc253a873a005a04b9ac59a5cee30054bba9fc4841d15a95875fe116c0";
        let keys = Keys::from_sk_str(private).unwrap();
        let event = EventBuilder::new(
            Kind::HttpAuth,
            "",
            vec![
                Tag::Method(HttpMethod::POST),
                Tag::AbsoluteURL("https://domain.com/api".into()),
            ],
        )
        .to_event(&keys)
        .unwrap();

        let mut mock = MockOps::new();

        mock.expect_get_config_values()
            .withf(|s| s == "pubkeyWhitelist")
            .returning(|_| Ok(vec![public.into()]));

        mock.expect_get_config_value()
            .withf(|s| s == "baseUrl")
            .returning(|_| Ok("https://domain.com".to_string()));

        let a = Auth { config: &mock };

        a.auth(&event, HttpMethod::POST, "/api").unwrap();
    }

    #[test]
    fn should_err_on_multiple_method_tags() {
        let private = "cb35da74d8d37ad5a2059d58e780cd0e160600ef62e3fd6a0399ebaf5b28695b";
        let public = "4344e9cc253a873a005a04b9ac59a5cee30054bba9fc4841d15a95875fe116c0";
        let keys = Keys::from_sk_str(private).unwrap();
        let event = EventBuilder::new(
            Kind::HttpAuth,
            "",
            vec![
                Tag::Method(HttpMethod::POST),
                Tag::Method(HttpMethod::POST),
                Tag::AbsoluteURL("https://domain.com/api".into()),
                Tag::Amount {
                    millisats: 10,
                    bolt11: Some(String::new()),
                },
            ],
        )
        .to_event(&keys)
        .unwrap();

        let mut mock = MockOps::new();

        mock.expect_get_config_values()
            .withf(|s| s == "pubkeyWhitelist")
            .returning(|_| Ok(vec![public.into()]));

        mock.expect_get_config_value()
            .withf(|s| s == "baseUrl")
            .returning(|_| Ok("https://domain.com".to_string()));

        let a = Auth { config: &mock };

        a.auth(&event, HttpMethod::POST, "/api").unwrap_err();
    }

    #[test]
    fn should_err_on_multiple_u_tags() {
        let private = "cb35da74d8d37ad5a2059d58e780cd0e160600ef62e3fd6a0399ebaf5b28695b";
        let public = "4344e9cc253a873a005a04b9ac59a5cee30054bba9fc4841d15a95875fe116c0";
        let keys = Keys::from_sk_str(private).unwrap();
        let event = EventBuilder::new(
            Kind::HttpAuth,
            "",
            vec![
                Tag::Method(HttpMethod::POST),
                Tag::AbsoluteURL("https://domain.com/api".into()),
                Tag::AbsoluteURL("https://domain.com/api".into()),
                Tag::Amount {
                    millisats: 10,
                    bolt11: Some(String::new()),
                },
            ],
        )
        .to_event(&keys)
        .unwrap();

        let mut mock = MockOps::new();

        mock.expect_get_config_values()
            .withf(|s| s == "pubkeyWhitelist")
            .returning(|_| Ok(vec![public.into()]));

        mock.expect_get_config_value()
            .withf(|s| s == "baseUrl")
            .returning(|_| Ok("https://domain.com".to_string()));

        let a = Auth { config: &mock };

        a.auth(&event, HttpMethod::POST, "/api").unwrap_err();
    }

    #[test]
    fn should_ok_on_good_auth_with_irrelevent_tags() {
        let private = "cb35da74d8d37ad5a2059d58e780cd0e160600ef62e3fd6a0399ebaf5b28695b";
        let public = "4344e9cc253a873a005a04b9ac59a5cee30054bba9fc4841d15a95875fe116c0";
        let keys = Keys::from_sk_str(private).unwrap();
        let event = EventBuilder::new(
            Kind::HttpAuth,
            "",
            vec![
                Tag::Method(HttpMethod::POST),
                Tag::AbsoluteURL("https://domain.com/api".into()),
                Tag::Amount {
                    millisats: 10,
                    bolt11: Some(String::new()),
                },
            ],
        )
        .to_event(&keys)
        .unwrap();

        let mut mock = MockOps::new();

        mock.expect_get_config_values()
            .withf(|s| s == "pubkeyWhitelist")
            .returning(|_| Ok(vec![public.into()]));

        mock.expect_get_config_value()
            .withf(|s| s == "baseUrl")
            .returning(|_| Ok("https://domain.com".to_string()));

        let a = Auth { config: &mock };

        a.auth(&event, HttpMethod::POST, "/api").unwrap();
    }

    #[test]
    fn should_err_on_old_timestamp() {
        let private = "cb35da74d8d37ad5a2059d58e780cd0e160600ef62e3fd6a0399ebaf5b28695b";
        let public = "4344e9cc253a873a005a04b9ac59a5cee30054bba9fc4841d15a95875fe116c0";
        let keys = Keys::from_sk_str(private).unwrap();
        let event = EventBuilder::new(
            Kind::HttpAuth,
            "",
            vec![
                Tag::Method(HttpMethod::POST),
                Tag::AbsoluteURL("https://domain.com/api".into()),
            ],
        )
        .to_event(&keys)
        .unwrap();

        let event = Event {
            created_at: Timestamp::now().sub(61_i64),
            ..event
        };

        let mut mock = MockOps::new();

        mock.expect_get_config_values()
            .withf(|s| s == "pubkeyWhitelist")
            .returning(|_| Ok(vec![public.into()]));

        mock.expect_get_config_value()
            .withf(|s| s == "baseUrl")
            .returning(|_| Ok("https://domain.com".to_string()));

        let a = Auth { config: &mock };

        let err = a.auth(&event, HttpMethod::POST, "/api").unwrap_err();

        assert_eq!(err.status(), StatusCode::UNAUTHORIZED);
    }

    #[test]
    fn should_err_on_future_timestamp() {
        let private = "cb35da74d8d37ad5a2059d58e780cd0e160600ef62e3fd6a0399ebaf5b28695b";
        let public = "4344e9cc253a873a005a04b9ac59a5cee30054bba9fc4841d15a95875fe116c0";
        let keys = Keys::from_sk_str(private).unwrap();
        let event = EventBuilder::new(
            Kind::HttpAuth,
            "",
            vec![
                Tag::Method(HttpMethod::POST),
                Tag::AbsoluteURL("https://domain.com/api".into()),
            ],
        )
        .to_event(&keys)
        .unwrap();

        let event = Event {
            created_at: Timestamp::now().add(9999_i64),
            ..event
        };

        let mut mock = MockOps::new();

        mock.expect_get_config_values()
            .withf(|s| s == "pubkeyWhitelist")
            .returning(|_| Ok(vec![public.into()]));

        mock.expect_get_config_value()
            .withf(|s| s == "baseUrl")
            .returning(|_| Ok("https://domain.com".to_string()));

        let a = Auth { config: &mock };

        let err = a.auth(&event, HttpMethod::POST, "/api").unwrap_err();

        assert_eq!(err.status(), StatusCode::UNAUTHORIZED);
    }

    #[test]
    fn should_err_on_bad_method_1() {
        let private = "cb35da74d8d37ad5a2059d58e780cd0e160600ef62e3fd6a0399ebaf5b28695b";
        let public = "4344e9cc253a873a005a04b9ac59a5cee30054bba9fc4841d15a95875fe116c0";
        let keys = Keys::from_sk_str(private).unwrap();
        let event = EventBuilder::new(
            Kind::HttpAuth,
            "",
            vec![
                Tag::Method(HttpMethod::POST),
                Tag::AbsoluteURL("https://domain.com/api".into()),
            ],
        )
        .to_event(&keys)
        .unwrap();

        let mut mock = MockOps::new();

        mock.expect_get_config_values()
            .withf(|s| s == "pubkeyWhitelist")
            .returning(|_| Ok(vec![public.into()]));

        mock.expect_get_config_value()
            .withf(|s| s == "baseUrl")
            .returning(|_| Ok("https://domain.com".to_string()));

        let a = Auth { config: &mock };

        let err = a.auth(&event, HttpMethod::GET, "/api").unwrap_err();

        assert_eq!(err.status(), StatusCode::UNAUTHORIZED);
    }

    #[test]
    fn should_err_on_bad_method_2() {
        let private = "cb35da74d8d37ad5a2059d58e780cd0e160600ef62e3fd6a0399ebaf5b28695b";
        let public = "4344e9cc253a873a005a04b9ac59a5cee30054bba9fc4841d15a95875fe116c0";
        let keys = Keys::from_sk_str(private).unwrap();
        let event = EventBuilder::new(
            Kind::HttpAuth,
            "",
            vec![
                Tag::Method(HttpMethod::PUT),
                Tag::AbsoluteURL("https://domain.com/api".into()),
            ],
        )
        .to_event(&keys)
        .unwrap();

        let mut mock = MockOps::new();

        mock.expect_get_config_values()
            .withf(|s| s == "pubkeyWhitelist")
            .returning(|_| Ok(vec![public.into()]));

        mock.expect_get_config_value()
            .withf(|s| s == "baseUrl")
            .returning(|_| Ok("https://domain.com".to_string()));

        let a = Auth { config: &mock };

        let err = a.auth(&event, HttpMethod::POST, "/api").unwrap_err();

        assert_eq!(err.status(), StatusCode::UNAUTHORIZED);
    }

    #[test]
    fn should_err_on_bad_u_tag() {
        let private = "cb35da74d8d37ad5a2059d58e780cd0e160600ef62e3fd6a0399ebaf5b28695b";
        let public = "4344e9cc253a873a005a04b9ac59a5cee30054bba9fc4841d15a95875fe116c0";
        let keys = Keys::from_sk_str(private).unwrap();
        let event = EventBuilder::new(
            Kind::HttpAuth,
            "",
            vec![
                Tag::Method(HttpMethod::POST),
                Tag::AbsoluteURL("domain.com/api".into()),
            ],
        )
        .to_event(&keys)
        .unwrap();

        let mut mock = MockOps::new();

        mock.expect_get_config_values()
            .withf(|s| s == "pubkeyWhitelist")
            .returning(|_| Ok(vec![public.into()]));

        mock.expect_get_config_value()
            .withf(|s| s == "baseUrl")
            .returning(|_| Ok("https://domain.com".to_string()));

        let a = Auth { config: &mock };

        let err = a.auth(&event, HttpMethod::POST, "/api").unwrap_err();

        assert_eq!(err.status(), StatusCode::UNAUTHORIZED);
    }

    #[test]
    fn should_err_on_bad_kind() {
        let private = "cb35da74d8d37ad5a2059d58e780cd0e160600ef62e3fd6a0399ebaf5b28695b";
        let public = "4344e9cc253a873a005a04b9ac59a5cee30054bba9fc4841d15a95875fe116c0";
        let keys = Keys::from_sk_str(private).unwrap();
        let event = EventBuilder::new(
            Kind::BadgeAward,
            "",
            vec![
                Tag::Method(HttpMethod::POST),
                Tag::AbsoluteURL("https://domain.com/api".into()),
            ],
        )
        .to_event(&keys)
        .unwrap();

        let mut mock = MockOps::new();

        mock.expect_get_config_values()
            .withf(|s| s == "pubkeyWhitelist")
            .returning(|_| Ok(vec![public.into()]));

        mock.expect_get_config_value()
            .withf(|s| s == "baseUrl")
            .returning(|_| Ok("https://domain.com".to_string()));

        let a = Auth { config: &mock };

        let err = a.auth(&event, HttpMethod::POST, "/api").unwrap_err();

        assert_eq!(err.status(), StatusCode::UNAUTHORIZED);
    }

    #[test]
    fn should_err_when_not_on_whitelist_1() {
        let private = "cb35da74d8d37ad5a2059d58e780cd0e160600ef62e3fd6a0399ebaf5b28695b";
        let keys = Keys::from_sk_str(private).unwrap();
        let event = EventBuilder::new(
            Kind::HttpAuth,
            "",
            vec![
                Tag::Method(HttpMethod::POST),
                Tag::AbsoluteURL("https://domain.com/api".into()),
            ],
        )
        .to_event(&keys)
        .unwrap();

        let mut mock = MockOps::new();

        mock.expect_get_config_values()
            .withf(|s| s == "pubkeyWhitelist")
            .returning(|_| {
                Ok(vec![
                    "0000000000000000000000000000000000000000000000000000000000000000".into(),
                    "1111111111111111111111111111111111111111111111111111111111111111".into(),
                    "2222222222222222222222222222222222222222222222222222222222222222".into(),
                    "3333333333333333333333333333333333333333333333333333333333333333".into(),
                    "4444444444444444444444444444444444444444444444444444444444444444".into(),
                ])
            });

        mock.expect_get_config_value()
            .withf(|s| s == "baseUrl")
            .returning(|_| Ok("https://domain.com".to_string()));

        let a = Auth { config: &mock };

        let err = a.auth(&event, HttpMethod::POST, "/api").unwrap_err();

        assert_eq!(err.status(), StatusCode::UNAUTHORIZED);
    }

    #[test]
    fn should_err_when_not_on_whitelist_2() {
        let private = "cb35da74d8d37ad5a2059d58e780cd0e160600ef62e3fd6a0399ebaf5b28695b";
        let keys = Keys::from_sk_str(private).unwrap();
        let event = EventBuilder::new(
            Kind::HttpAuth,
            "",
            vec![
                Tag::Method(HttpMethod::POST),
                Tag::AbsoluteURL("https://domain.com/api".into()),
            ],
        )
        .to_event(&keys)
        .unwrap();

        let mut mock = MockOps::new();

        mock.expect_get_config_values()
            .withf(|s| s == "pubkeyWhitelist")
            .returning(|_| Ok(vec![]));

        mock.expect_get_config_value()
            .withf(|s| s == "baseUrl")
            .returning(|_| Ok("https://domain.com".to_string()));

        let a = Auth { config: &mock };

        let err = a.auth(&event, HttpMethod::POST, "/api").unwrap_err();

        assert_eq!(err.status(), StatusCode::UNAUTHORIZED);
    }

    #[test]
    fn should_err_when_id_is_bad() {
        let private = "cb35da74d8d37ad5a2059d58e780cd0e160600ef62e3fd6a0399ebaf5b28695b";
        let public = "4344e9cc253a873a005a04b9ac59a5cee30054bba9fc4841d15a95875fe116c0";
        let keys = Keys::from_sk_str(private).unwrap();
        let event = EventBuilder::new(
            Kind::HttpAuth,
            "",
            vec![
                Tag::Method(HttpMethod::POST),
                Tag::AbsoluteURL("https://domain.com/api".into()),
            ],
        )
        .to_event(&keys)
        .unwrap();

        let event = Event {
            id: EventId::all_zeros(),
            ..event
        };

        let mut mock = MockOps::new();

        mock.expect_get_config_values()
            .withf(|s| s == "pubkeyWhitelist")
            .returning(|_| Ok(vec![public.into()]));

        mock.expect_get_config_value()
            .withf(|s| s == "baseUrl")
            .returning(|_| Ok("https://domain.com".to_string()));

        let a = Auth { config: &mock };

        let err = a.auth(&event, HttpMethod::POST, "/api").unwrap_err();

        assert_eq!(err.status(), StatusCode::UNAUTHORIZED);
    }

    #[test]
    fn should_err_when_sig_is_bad() {
        let private_a = "cb35da74d8d37ad5a2059d58e780cd0e160600ef62e3fd6a0399ebaf5b28695b";
        let public_a = "4344e9cc253a873a005a04b9ac59a5cee30054bba9fc4841d15a95875fe116c0";
        let private_b = "88b3d43e0c971877e01d289294b2e0a0d389a1e51b5cd11dd9e173682ab17962";
        let key_a = Keys::from_sk_str(private_a).unwrap();
        let key_b = Keys::from_sk_str(private_b).unwrap();
        let builder = EventBuilder::new(
            Kind::HttpAuth,
            "",
            vec![
                Tag::Method(HttpMethod::POST),
                Tag::AbsoluteURL("https://domain.com/api".into()),
            ],
        );

        let event_a = builder.clone().to_event(&key_a).unwrap();
        let event_b = builder.to_event(&key_b).unwrap();

        let event = Event {
            sig: event_b.sig,
            ..event_a
        };

        let mut mock = MockOps::new();

        mock.expect_get_config_values()
            .withf(|s| s == "pubkeyWhitelist")
            .returning(|_| Ok(vec![public_a.into()]));

        mock.expect_get_config_value()
            .withf(|s| s == "baseUrl")
            .returning(|_| Ok("https://domain.com".to_string()));

        let a = Auth { config: &mock };

        let err = a.auth(&event, HttpMethod::POST, "/api").unwrap_err();

        assert_eq!(err.status(), StatusCode::UNAUTHORIZED);
    }
}
