// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

//! Helper for creating the appropriate auth policy from a [`StorageCredential`].

use azure_core::http::policies::{auth::BearerTokenAuthorizationPolicy, Policy};
use azure_storage_common::{SharedKeyAuthorizationPolicy, StorageCredential};
use std::sync::Arc;

/// The default scope for Azure Storage bearer token authentication.
const STORAGE_TOKEN_SCOPE: &str = "https://storage.azure.com/.default";

/// Creates the appropriate authorization [`Policy`] for the given credential.
pub(crate) fn create_auth_policy(credential: &StorageCredential) -> Arc<dyn Policy> {
    match credential {
        StorageCredential::TokenCredential(token_credential) => {
            Arc::new(BearerTokenAuthorizationPolicy::new(
                token_credential.clone(),
                vec![STORAGE_TOKEN_SCOPE],
            ))
        }
        StorageCredential::SharedKey(shared_key_credential) => Arc::new(
            SharedKeyAuthorizationPolicy::new(shared_key_credential.clone()),
        ),
    }
}
