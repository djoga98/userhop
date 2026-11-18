use crate::errors::{ErrorCode, SwitcherError, SwitcherResult};
use crate::models::SwitchRequest;

use super::{AccountKey, SessionKey};

pub const MAX_ID_LEN: usize = 128;

fn is_valid_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_ID_LEN
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b':' | b'-' | b'_' | b'.'))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedRequest {
    pub account: AccountKey,
    pub session: Option<SessionKey>,
}

pub fn validate_request(request: &SwitchRequest) -> SwitcherResult<ValidatedRequest> {
    if !is_valid_id(&request.account_id) {
        return Err(SwitcherError::new(ErrorCode::InvalidRequest));
    }
    let session = match &request.session_id {
        Some(id) if is_valid_id(id) => Some(SessionKey(id.clone())),
        Some(_) => return Err(SwitcherError::new(ErrorCode::InvalidRequest)),
        None => None,
    };
    Ok(ValidatedRequest {
        account: AccountKey(request.account_id.clone()),
        session,
    })
}

/// Sanitizes OS-provided display strings before they reach the UI.
pub fn bounded_label(value: &str, max_chars: usize) -> String {
    value
        .chars()
        .filter(|c| !c.is_control())
        .take(max_chars)
        .collect::<String>()
        .trim()
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(account: &str, session: Option<&str>) -> SwitchRequest {
        SwitchRequest {
            account_id: account.to_owned(),
            session_id: session.map(str::to_owned),
        }
    }

    #[test]
    fn accepts_provider_qualified_ids() {
        let validated =
            validate_request(&request("win:S-1-5-21-1-2-3-1001", Some("win:session:2")))
                .expect("valid request");
        assert_eq!(validated.account.0, "win:S-1-5-21-1-2-3-1001");
        assert_eq!(validated.session, Some(SessionKey("win:session:2".into())));
    }

    #[test]
    fn rejects_empty_oversized_and_unexpected_characters() {
        let oversized = "a".repeat(MAX_ID_LEN + 1);
        for bad in [
            "",
            oversized.as_str(),
            "linux:uid:1000; rm -rf /",
            "a/b",
            "名前",
            "a b",
        ] {
            assert_eq!(
                validate_request(&request(bad, None)).unwrap_err().code,
                ErrorCode::InvalidRequest,
                "{bad:?} must be rejected"
            );
        }
        assert_eq!(
            validate_request(&request("linux:uid:1000", Some("../1")))
                .unwrap_err()
                .code,
            ErrorCode::InvalidRequest
        );
    }

    #[test]
    fn labels_drop_control_characters_and_are_bounded() {
        assert_eq!(bounded_label("  Ana\u{0007}\nMarić ", 64), "AnaMarić");
        assert_eq!(bounded_label("abcdef", 3), "abc");
    }
}
