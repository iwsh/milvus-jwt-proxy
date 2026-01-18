use base64::{Engine as _, engine::general_purpose};
use tracing::warn;

pub struct AuthTransformer;

impl AuthTransformer {
    pub fn new() -> Self {
        AuthTransformer
    }

    pub fn transform_auth_header(&self, header_value: &str) -> String {
        // If it already starts with Bearer, pass it through
        if header_value.starts_with("Bearer ") {
            return header_value.to_string();
        }

        // Try to decode base64
        let decoded = match general_purpose::STANDARD.decode(header_value) {
            Ok(bytes) => match String::from_utf8(bytes) {
                Ok(s) => s,
                Err(e) => {
                    warn!("Failed to parse decoded base64 as UTF-8: {}", e);
                    return header_value.to_string();
                }
            },
            Err(e) => {
                warn!("Failed to decode base64 authorization header: {}", e);
                return header_value.to_string();
            }
        };

        // Expected format: x-jwt-token:${JWT}
        if let Some(jwt) = decoded.strip_prefix("x-jwt-token:") {
            format!("Bearer {}", jwt)
        } else {
            warn!("Decoded authorization header missing 'x-jwt-token:' prefix");
            header_value.to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transform_auth_header_valid() {
        let transformer = AuthTransformer::new();
        // echo -n "x-jwt-token:test_token_123" | base64
        let valid_base64 = "eC1qd3QtdG9rZW46dGVzdF90b2tlbl8xMjM=";
        let result = transformer.transform_auth_header(valid_base64);
        assert_eq!(result, "Bearer test_token_123");
    }

    #[test]
    fn test_transform_auth_header_bearer_passthrough() {
        let transformer = AuthTransformer::new();
        let bearer = "Bearer existing_token";
        let result = transformer.transform_auth_header(bearer);
        assert_eq!(result, bearer);
    }

    #[test]
    fn test_transform_auth_header_invalid_base64() {
        let transformer = AuthTransformer::new();
        let invalid = "not-base64!!!";
        let result = transformer.transform_auth_header(invalid);
        assert_eq!(result, invalid);
    }

    #[test]
    fn test_transform_auth_header_missing_prefix() {
        let transformer = AuthTransformer::new();
        // echo -n "no-prefix-token" | base64
        let missing_prefix = "bm8tcHJlZml4LXRva2Vu";
        let result = transformer.transform_auth_header(missing_prefix);
        assert_eq!(result, missing_prefix);
    }
}
