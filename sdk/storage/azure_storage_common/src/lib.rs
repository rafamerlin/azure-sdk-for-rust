// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

//! Common types shared across Azure Storage crates.

pub mod credentials;
pub mod shared_key_authorization_policy;

pub use credentials::{StorageCredential, StorageSharedKeyCredential};
pub use shared_key_authorization_policy::SharedKeyAuthorizationPolicy;
