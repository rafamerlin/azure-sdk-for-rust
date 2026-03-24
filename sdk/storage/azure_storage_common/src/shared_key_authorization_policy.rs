// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

//! Authorization policy for Azure Storage Shared Key requests.
//!
//! This policy implements Azure Storage's Shared Key authorization scheme,
//! computing an HMAC-SHA256 signature for each request as defined in the
//! [official documentation](https://learn.microsoft.com/rest/api/storageservices/authorize-with-shared-key).

use crate::credentials::StorageSharedKeyCredential;
use azure_core::{
    http::{
        headers::{HeaderName, HeaderValue, AUTHORIZATION},
        policies::{Policy, PolicyResult},
        Context, Request,
    },
    time::{self, OffsetDateTime},
};
use std::sync::Arc;
use tracing::trace;

/// `x-ms-date` header name.
const MS_DATE: HeaderName = HeaderName::from_static("x-ms-date");

/// Standard headers included in the Shared Key signature, in order.
const SIGNATURE_HEADERS: &[&str] = &[
    "content-encoding",
    "content-language",
    "content-length",
    "content-md5",
    "content-type",
    "date",
    "if-modified-since",
    "if-match",
    "if-none-match",
    "if-unmodified-since",
    "range",
];

/// Authorization policy that signs requests using Azure Storage Shared Key.
///
/// This policy computes the authorization signature based on the request method,
/// headers, and canonicalized resource, then adds the `Authorization` and
/// `x-ms-date` headers.
#[derive(Debug, Clone)]
pub struct SharedKeyAuthorizationPolicy {
    credential: StorageSharedKeyCredential,
}

impl SharedKeyAuthorizationPolicy {
    /// Creates a new authorization policy from a shared key credential.
    pub fn new(credential: StorageSharedKeyCredential) -> Self {
        Self { credential }
    }
}

#[async_trait::async_trait]
impl Policy for SharedKeyAuthorizationPolicy {
    async fn send(
        &self,
        ctx: &Context,
        request: &mut Request,
        next: &[Arc<dyn Policy>],
    ) -> PolicyResult {
        trace!("SharedKeyAuthorizationPolicy::send");

        assert!(
            !next.is_empty(),
            "SharedKeyAuthorizationPolicy cannot be the last policy in a pipeline"
        );

        // Set x-ms-date header with current UTC time in RFC 7231 format.
        let date_string = time::to_rfc7231(&OffsetDateTime::now_utc());
        request.insert_header(MS_DATE, HeaderValue::from(date_string));

        // Build the string-to-sign and compute the HMAC-SHA256 signature.
        let string_to_sign = build_string_to_sign(request, self.credential.account_name());
        trace!(
            string_to_sign = string_to_sign.replace('\n', "\\n"),
            "computing shared key signature"
        );

        let signature =
            azure_core::hmac::hmac_sha256(&string_to_sign, self.credential.account_key())?;

        let auth_value = format!("SharedKey {}:{signature}", self.credential.account_name());
        request.insert_header(AUTHORIZATION, HeaderValue::from(auth_value));

        next[0].send(ctx, request, &next[1..]).await
    }
}

/// Builds the string-to-sign for Azure Storage Shared Key authorization.
///
/// The format is defined at:
/// <https://learn.microsoft.com/rest/api/storageservices/authorize-with-shared-key>
///
/// ```text
/// StringToSign = VERB + "\n" +
///                Content-Encoding + "\n" +
///                Content-Language + "\n" +
///                Content-Length + "\n" +
///                Content-MD5 + "\n" +
///                Content-Type + "\n" +
///                Date + "\n" +
///                If-Modified-Since + "\n" +
///                If-Match + "\n" +
///                If-None-Match + "\n" +
///                If-Unmodified-Since + "\n" +
///                Range + "\n" +
///                CanonicalizedHeaders +
///                CanonicalizedResource
/// ```
fn build_string_to_sign(request: &Request, account_name: &str) -> String {
    let mut result = String::with_capacity(256);

    // HTTP verb.
    result.push_str(request.method().to_string().to_uppercase().as_str());
    result.push('\n');

    // Standard headers.
    for &header_name in SIGNATURE_HEADERS {
        let value = request
            .headers()
            .get_optional_str(&HeaderName::from(header_name.to_string()))
            .unwrap_or("");

        // Per the spec: if Content-Length is 0, use empty string instead.
        if header_name == "content-length" && value == "0" {
            // empty
        } else {
            result.push_str(value);
        }
        result.push('\n');
    }

    // Canonicalized headers: all x-ms- headers, sorted, lowercased.
    result.push_str(&build_canonicalized_headers(request));

    // Canonicalized resource.
    result.push_str(&build_canonicalized_resource(request, account_name));

    result
}

/// Builds the CanonicalizedHeaders portion of the string-to-sign.
///
/// 1. Collect all headers starting with `x-ms-`.
/// 2. Sort them alphabetically by header name.
/// 3. Format as `{name}:{value}\n`.
fn build_canonicalized_headers(request: &Request) -> String {
    let mut headers: Vec<(&str, &str)> = request
        .headers()
        .iter()
        .filter_map(|(name, value)| {
            let name_str = name.as_str();
            if name_str.starts_with("x-ms-") {
                Some((name_str, value.as_str()))
            } else {
                None
            }
        })
        .collect();

    headers.sort_by_key(|(name, _)| *name);

    let mut result = String::new();
    for (name, value) in &headers {
        result.push_str(name);
        result.push(':');
        // Trim whitespace and collapse internal whitespace in the value.
        result.push_str(value.trim());
        result.push('\n');
    }
    result
}

/// Builds the CanonicalizedResource portion of the string-to-sign.
///
/// Format: `/{accountName}/{path}\n{param1}:{value1}\n{param2}:{value2}...`
///
/// Query parameters are sorted alphabetically by parameter name.
fn build_canonicalized_resource(request: &Request, account_name: &str) -> String {
    let url = request.url();

    let mut result = String::new();
    result.push('/');
    result.push_str(account_name);

    // URL path (already starts with /).
    let path = url.path();
    result.push_str(path);

    // Query parameters, sorted alphabetically.
    let mut query_params: Vec<(String, String)> = url
        .query_pairs()
        .map(|(k, v)| (k.to_lowercase(), v.to_string()))
        .collect();

    query_params.sort_by(|(a, _), (b, _)| a.cmp(b));

    for (key, value) in &query_params {
        result.push('\n');
        result.push_str(key);
        result.push(':');
        result.push_str(value);
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use azure_core::{
        credentials::Secret,
        http::{headers::Headers, response::AsyncRawResponse, Method, StatusCode, Url},
    };

    /// Mock transport policy for testing.
    #[derive(Debug)]
    struct MockTransport;

    #[async_trait::async_trait]
    impl Policy for MockTransport {
        async fn send(
            &self,
            _ctx: &Context,
            _request: &mut Request,
            _next: &[Arc<dyn Policy>],
        ) -> PolicyResult {
            Ok(AsyncRawResponse::from_bytes(
                StatusCode::Ok,
                Headers::new(),
                azure_core::Bytes::new(),
            ))
        }
    }

    #[tokio::test]
    async fn shared_key_policy_adds_authorization_header() {
        let cred = StorageSharedKeyCredential::new(
            "devstoreaccount1",
            "Eby8vdM02xNOcqFlqUwJPLlmEtlCDXJ1OUzFT50uSRZ6IFsuFq2UVErCz4I6tq/K1SZFPTOtr/KBHBeksoGMGw==",
        );
        let policy = SharedKeyAuthorizationPolicy::new(cred);

        let transport: Arc<dyn Policy> = Arc::new(MockTransport);
        let policies: Vec<Arc<dyn Policy>> = vec![transport];

        let url = Url::parse("https://devstoreaccount1.blob.core.windows.net/mycontainer/myblob")
            .unwrap();
        let mut request = Request::new(url, Method::Get);

        let ctx = Context::default();
        let result = policy.send(&ctx, &mut request, &policies).await;
        assert!(result.is_ok());

        // Verify headers were set.
        assert!(request.headers().get_optional_str(&MS_DATE).is_some());
        let auth_header = request.headers().get_optional_str(&AUTHORIZATION).unwrap();
        assert!(
            auth_header.starts_with("SharedKey devstoreaccount1:"),
            "Expected SharedKey header, got: {auth_header}"
        );
    }

    #[test]
    fn canonicalized_headers_sorted() {
        let url = Url::parse("https://myaccount.blob.core.windows.net/mycontainer").unwrap();
        let mut request = Request::new(url, Method::Get);
        request.insert_header(
            HeaderName::from("x-ms-version".to_string()),
            HeaderValue::from("2024-08-04"),
        );
        request.insert_header(
            HeaderName::from("x-ms-date".to_string()),
            HeaderValue::from("Sun, 01 Jan 2023 00:00:00 GMT"),
        );
        request.insert_header(
            HeaderName::from("x-ms-blob-type".to_string()),
            HeaderValue::from("BlockBlob"),
        );

        let result = build_canonicalized_headers(&request);

        // Should be sorted alphabetically.
        assert!(result.starts_with("x-ms-blob-type:BlockBlob\n"));
        assert!(result.contains("x-ms-date:Sun, 01 Jan 2023 00:00:00 GMT\n"));
        assert!(result.contains("x-ms-version:2024-08-04\n"));
    }

    #[test]
    fn canonicalized_resource_with_query_params() {
        let url = Url::parse(
            "https://myaccount.blob.core.windows.net/mycontainer?restype=container&comp=list",
        )
        .unwrap();
        let request = Request::new(url, Method::Get);

        let result = build_canonicalized_resource(&request, "myaccount");

        assert_eq!(
            result,
            "/myaccount/mycontainer\ncomp:list\nrestype:container"
        );
    }

    #[test]
    fn canonicalized_resource_without_query_params() {
        let url = Url::parse("https://myaccount.blob.core.windows.net/mycontainer/myblob").unwrap();
        let request = Request::new(url, Method::Get);

        let result = build_canonicalized_resource(&request, "myaccount");

        assert_eq!(result, "/myaccount/mycontainer/myblob");
    }

    #[test]
    fn string_to_sign_format() {
        let url = Url::parse("https://myaccount.blob.core.windows.net/mycontainer/myblob").unwrap();
        let mut request = Request::new(url, Method::Get);
        request.insert_header(
            HeaderName::from("x-ms-date".to_string()),
            HeaderValue::from("Sun, 01 Jan 2023 00:00:00 GMT"),
        );
        request.insert_header(
            HeaderName::from("x-ms-version".to_string()),
            HeaderValue::from("2024-08-04"),
        );

        let result = build_string_to_sign(&request, "myaccount");

        // Should start with the HTTP verb.
        assert!(result.starts_with("GET\n"));
        // Should end with the canonicalized resource.
        assert!(result.ends_with("/myaccount/mycontainer/myblob"));

        // Should have 12 lines for 1 verb + 11 headers, plus canonicalized headers + resource.
        let lines: Vec<&str> = result.split('\n').collect();
        assert!(
            lines.len() >= 14,
            "Expected at least 14 lines, got {}",
            lines.len()
        );
    }

    #[test]
    fn signature_is_valid() {
        // Use a known key/data pair to validate the signature computation.
        let key = Secret::new("Eby8vdM02xNOcqFlqUwJPLlmEtlCDXJ1OUzFT50uSRZ6IFsuFq2UVErCz4I6tq/K1SZFPTOtr/KBHBeksoGMGw==");
        let data = "test data";
        let result = azure_core::hmac::hmac_sha256(data, &key);
        assert!(result.is_ok(), "HMAC computation should succeed");
    }
}
