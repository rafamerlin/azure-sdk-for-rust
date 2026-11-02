// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

//! SharedKey authorization for Azure Storage.
//!
//! The generated Storage clients accept only Entra ID (`TokenCredential`)
//! credentials. `SharedKeyPolicy` signs every request with the legacy
//! SharedKey scheme instead, which is useful for:
//!
//! - [Azurite], the emulator, which does not implement Entra ID.
//! - accounts that permit Shared Key authorization and have no Entra setup.
//!
//! The policy is a per-call [`Policy`] used with `client_options`
//! (for example `per_call_policies`), and must be inserted before any other
//! pipeline policy so the signature covers the final request state. See the
//! re-export in `azure_storage_blob::shared_key` (and
//! `azure_storage_queue::shared_key`) for a compiling example.
//!
//! Real accounts use host-style endpoints
//! (`https://{account}.blob.core.windows.net`); Azurite uses path-style
//! (`http://127.0.0.1:10000/{account}`), which the canonicalized resource
//! handles.
//!
//! The scheme signs the canonicalized resource with query parameters decoded
//! from the request URL, so it is compatible with the `QueryBuilder`
//! percent-encoding of spaces as `%20`.
//!
//! [Azurite]: https://github.com/Azure/Azurite

use std::{collections::BTreeMap, sync::Arc};

use async_trait::async_trait;
use azure_core::{
    credentials::Secret,
    hmac::hmac_sha256,
    http::{
        headers::{HeaderName, Headers},
        policies::{Policy, PolicyResult},
        Context, Request, Url,
    },
    time::{to_rfc7231, OffsetDateTime},
};

/// SharedKey authorization policy for path-style (Azurite) and host-style
/// (real account) endpoints.
///
/// Insert into `ClientOptions::per_call_policies` (as an
/// `Arc<dyn Policy>`), ahead of the transport so the signature covers the
/// final URL.
///
/// It signs every request with the account name and base64 account key
/// provided at construction. The key is held in a [`Secret`] and is never
/// printed or logged.
#[derive(Debug)]
pub struct SharedKeyPolicy {
    account: String,
    key: Secret,
}

impl SharedKeyPolicy {
    /// Creates the policy from an account name and a base64 account key.
    pub fn new(account: impl Into<String>, key: impl Into<String>) -> Self {
        Self {
            account: account.into(),
            key: Secret::new(key.into()),
        }
    }
}

#[async_trait]
impl Policy for SharedKeyPolicy {
    async fn send(
        &self,
        ctx: &Context,
        request: &mut Request,
        next: &[Arc<dyn Policy>],
    ) -> PolicyResult {
        assert!(
            !next.is_empty(),
            "authorization policies cannot be the last policy of a pipeline"
        );

        // The date header is part of the canonicalized x-ms-* headers.
        request.insert_header("x-ms-date", to_rfc7231(&OffsetDateTime::now_utc()));

        // The transport sets Content-Length when sending, but the signature
        // must match what the service receives, so set it here if known and
        // non-zero (per spec, Content-Length must be empty when zero).
        if let Some(len) = request.body().len().filter(|len| *len != 0) {
            request.insert_header("content-length", len.to_string());
        }

        let string_to_sign = string_to_sign(&self.account, request);
        let signature = hmac_sha256(&string_to_sign, &self.key)?;
        request.insert_header(
            "authorization",
            format!("SharedKey {}:{}", self.account, signature),
        );

        next[0].send(ctx, request, &next[1..]).await
    }
}

fn string_to_sign(account: &str, request: &Request) -> String {
    let headers = request.headers();
    let content_length = headers
        .get_optional_str(&HeaderName::from_static("content-length"))
        .filter(|value| *value != "0")
        .unwrap_or_default();
    format!(
        "{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}{}",
        request.method(),
        add_if_exists(headers, "content-encoding"),
        add_if_exists(headers, "content-language"),
        content_length,
        add_if_exists(headers, "content-md5"),
        add_if_exists(headers, "content-type"),
        // The Date header is redundant with x-ms-date, which is included in
        // the canonicalized headers.
        add_if_exists(headers, "date"),
        add_if_exists(headers, "if-modified-since"),
        add_if_exists(headers, "if-match"),
        add_if_exists(headers, "if-none-match"),
        add_if_exists(headers, "if-unmodified-since"),
        add_if_exists(headers, "range"),
        canonicalize_headers(headers),
        canonicalized_resource(account, &request.url()),
    )
}

fn add_if_exists<'a>(headers: &'a Headers, name: &'static str) -> &'a str {
    headers
        .get_optional_str(&HeaderName::from_static(name))
        .unwrap_or_default()
}

/// Formats every `x-ms-*` header as lowercase `name:value` lines, sorted by name.
fn canonicalize_headers(headers: &Headers) -> String {
    let mut names: Vec<&HeaderName> = headers
        .iter()
        .map(|(name, _)| name)
        .filter(|name| name.as_str().starts_with("x-ms"))
        .collect();
    names.sort_unstable();

    let mut result = String::new();
    for name in names {
        let value = headers.get_optional_str(name).unwrap_or_default();
        result.push_str(&format!("{}:{}\n", name.as_str(), value));
    }
    result
}

/// Builds the canonicalized resource.
///
/// It is `/<account>` followed by the URL path exactly as sent. For the
/// real service's host-style URLs this yields `/<account>` for container
/// paths and `/<account>/` for the service root (the server's
/// canonicalization keeps the root path's trailing slash). For Azurite's
/// path-style URLs the account is part of the path, so it appears twice,
/// which matches what the emulator expects. Query parameters follow, sorted
/// by key, with lowercased keys and decoded values.
fn canonicalized_resource(account: &str, uri: &Url) -> String {
    let mut can_res = format!("/{account}{}", uri.path());
    can_res.push('\n');

    let mut params: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (key, value) in uri.query_pairs() {
        params
            .entry(key.to_lowercase())
            .or_default()
            .push(value.to_string());
    }
    for (key, mut values) in params {
        values.sort_unstable();
        can_res.push_str(&key);
        can_res.push(':');
        can_res.push_str(&values.join(","));
        can_res.push('\n');
    }
    can_res.truncate(can_res.len() - 1);
    can_res
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonicalized_resource_host_style_path() {
        // Real accounts: the root path is "/", and the server's
        // canonicalization keeps its trailing slash in the resource string.
        let url = Url::parse(
            "https://myaccount.blob.core.windows.net/?comp=blobs&where=%22foo%22%3D%27bar%27+AND+%22env%22%3D%27test%27",
        )
        .unwrap();
        let expected = concat!(
            "/myaccount/\n",
            "comp:blobs\n",
            "where:\"foo\"='bar' AND \"env\"='test'",
        );
        assert_eq!(canonicalized_resource("myaccount", &url), expected);
    }

    #[test]
    fn canonicalized_resource_host_style_container_path() {
        // Real accounts: container-scoped operations yield "/<account>/<path>".
        let url = Url::parse(
            "https://myaccount.blob.core.windows.net/mycontainer?restype=container&comp=blobs",
        )
        .unwrap();
        let expected = concat!(
            "/myaccount/mycontainer\n",
            "comp:blobs\n",
            "restype:container",
        );
        assert_eq!(canonicalized_resource("myaccount", &url), expected);
    }

    #[test]
    fn canonicalized_resource_path_style_path() {
        // Azurite: the account is part of the path, so it appears twice.
        let url =
            Url::parse("https://127.0.0.1:10000/devstoreaccount1?comp=blobs&where=a").unwrap();
        let expected = concat!(
            "/devstoreaccount1/devstoreaccount1\n",
            "comp:blobs\n",
            "where:a"
        );
        assert_eq!(canonicalized_resource("devstoreaccount1", &url), expected);
    }
}
