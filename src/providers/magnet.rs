//! Magnet URI parsing: extract a validated info hash from a `magnet:` link.

use crate::search::model::normalize_info_hash;

/// Returns the lowercased 40-char BTIH from a `magnet:?xt=urn:btih:...` URI, or
/// `None` if the URI has no valid `xt` parameter.
pub fn magnet_info_hash(magnet: &str) -> Option<String> {
    let params = magnet.strip_prefix("magnet:")?;
    let params = params.strip_prefix('?').unwrap_or(params);
    for pair in params.split('&') {
        if let Some(v) = pair.strip_prefix("xt=") {
            if let Some(hash) = v.strip_prefix("urn:btih:") {
                return normalize_info_hash(hash);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_and_lowercases_info_hash() {
        let m = "magnet:?xt=urn:btih:AE0CF422FA0BC9669AD605EA564999A8FBE5AA4B&dn=x&tr=y";
        assert_eq!(
            magnet_info_hash(m).as_deref(),
            Some("ae0cf422fa0bc9669ad605ea564999a8fbe5aa4b")
        );
    }

    #[test]
    fn rejects_missing_or_invalid_xt() {
        assert_eq!(magnet_info_hash("magnet:?dn=x"), None);
        assert_eq!(magnet_info_hash("magnet:?xt=urn:btih:short"), None);
        assert_eq!(magnet_info_hash("not-a-magnet"), None);
    }
}
