// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

//! Credential types for authenticating with Azure Storage.

use azure_core::credentials::{Secret, TokenCredential};
use std::{fmt, sync::Arc};

/// A credential for authenticating with Azure Storage using Shared Key authorization.
///
/// This holds the storage account name and access key, which are used to compute
/// an HMAC-SHA256 signature for each request.
///
/// You can find your account access keys in the Azure portal under your storage
/// account's "Access keys" section.
#[derive(Clone)]
pub struct StorageSharedKeyCredential {
    account_name: String,
    account_key: Secret,
}

impl StorageSharedKeyCredential {
    /// Creates a new `StorageSharedKeyCredential`.
    ///
    /// # Arguments
    ///
    /// * `account_name` - The Azure Storage account name.
    /// * `account_key` - The account's access key (base64-encoded).
    pub fn new(account_name: impl Into<String>, account_key: impl Into<Secret>) -> Self {
        Self {
            account_name: account_name.into(),
            account_key: account_key.into(),
        }
    }

    /// Gets the storage account name.
    pub fn account_name(&self) -> &str {
        &self.account_name
    }

    /// Gets the account key.
    pub(crate) fn account_key(&self) -> &Secret {
        &self.account_key
    }
}

impl fmt::Debug for StorageSharedKeyCredential {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("StorageSharedKeyCredential")
            .field("account_name", &self.account_name)
            .field("account_key", &"***")
            .finish()
    }
}

/// A credential for authenticating with Azure Storage.
///
/// Supports both Entra ID token-based authentication and Shared Key authentication.
#[derive(Clone)]
pub enum StorageCredential {
    /// Entra ID (Azure AD) token-based authentication.
    TokenCredential(Arc<dyn TokenCredential>),
    /// Shared Key authentication using the storage account name and access key.
    SharedKey(StorageSharedKeyCredential),
}

impl fmt::Debug for StorageCredential {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TokenCredential(_) => f.debug_tuple("TokenCredential").field(&"***").finish(),
            Self::SharedKey(cred) => f.debug_tuple("SharedKey").field(cred).finish(),
        }
    }
}

// Allow converting any concrete token credential wrapped in `Arc`.
// The generic impl covers all sized types (the usual case).
impl<T: TokenCredential + 'static> From<Arc<T>> for StorageCredential {
    fn from(credential: Arc<T>) -> Self {
        Self::TokenCredential(credential)
    }
}

// We also need a specific impl for `Arc<dyn TokenCredential>` because the
// blanket generic above requires `T: Sized`.  Many helpers such as
// `Recording::credential()` return the trait object type directly, so having
// an explicit implementation avoids compilation errors when passing those
// values to client constructors.
impl From<Arc<dyn TokenCredential>> for StorageCredential {
    fn from(credential: Arc<dyn TokenCredential>) -> Self {
        Self::TokenCredential(credential)
    }
}

impl From<StorageSharedKeyCredential> for StorageCredential {
    fn from(credential: StorageSharedKeyCredential) -> Self {
        Self::SharedKey(credential)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_does_not_print_key() {
        let cred = StorageSharedKeyCredential::new("myaccount", "mysecretkey");
        let debug_output = format!("{cred:?}");
        assert!(debug_output.contains("myaccount"));
        assert!(!debug_output.contains("mysecretkey"));
    }

    #[test]
    fn from_shared_key_into_storage_credential() {
        let shared_key = StorageSharedKeyCredential::new("myaccount", "mykey");
        let cred: StorageCredential = shared_key.into();
        assert!(matches!(cred, StorageCredential::SharedKey(_)));
    }
}
