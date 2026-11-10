// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

//! Live emulator checks for the fork-only request paths.
//!
//! These tests are opt-in. They run only when `AZURE_STORAGE_EMULATOR=1`;
//! otherwise they skip and `cargo test` stays offline.
//!
//! ```bash
//! AZURE_STORAGE_EMULATOR=1 \
//! AZURE_STORAGE_ACCESS_KEY="$AZURITE_ACCOUNT_KEY" \
//!   cargo test -p azure_storage_blob --test emulator
//! ```
//!
//! The account name and endpoint default to Azurite's path-style form:
//! `http://127.0.0.1:10000/devstoreaccount1`.
//!
//! What this proves:
//!
//! - `SharedKeyPolicy` signs a real request the emulator accepts.
//! - `delimiter` rolls virtual folders into `BlobPrefix` rows.
//! - a tag filter containing spaces is accepted end to end.
//!
//! What this does not prove: tag-search paging. Azurite 3.35 returns
//! `ceil(maxresults/2)` items and an empty `NextMarker` for that operation.
//! See `BRANCH-MAINTENANCE.md`.

use std::{
    env,
    error::Error,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use azure_core::http::{RequestContent, Url};
use azure_storage_blob::{
    models::BlobContainerClientListBlobsOptions, shared_key::SharedKeyPolicy, BlobServiceClient,
    BlobServiceClientOptions,
};
use futures::TryStreamExt;

struct Emulator {
    endpoint: String,
    account: String,
    key: String,
}

/// Returns the emulator settings, or `None` when the suite is not enabled.
fn emulator() -> Option<Emulator> {
    let enabled = env::var("AZURE_STORAGE_EMULATOR")
        .map(|value| value == "1" || value.eq_ignore_ascii_case("true"))
        .unwrap_or(false);
    if !enabled {
        eprintln!(
            "SKIP emulator checks: set AZURE_STORAGE_EMULATOR=1 to run them. \
             See tests/emulator.rs."
        );
        return None;
    }

    Some(Emulator {
        endpoint: env::var("AZURE_STORAGE_EMULATOR_ENDPOINT")
            .unwrap_or_else(|_| "http://127.0.0.1:10000/devstoreaccount1".to_owned()),
        account: env::var("AZURE_STORAGE_ACCOUNT")
            .unwrap_or_else(|_| "devstoreaccount1".to_owned()),
        key: env::var("AZURE_STORAGE_ACCESS_KEY").unwrap_or_else(|_| {
            panic!(
                "AZURE_STORAGE_ACCESS_KEY must be set when AZURE_STORAGE_EMULATOR=1. \
                 Use the emulator account key; do not commit it."
            )
        }),
    })
}

fn unique_suffix() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    format!("{}-{}", std::process::id(), nanos)
}

#[tokio::test]
async fn shared_key_auth_and_delimited_listing() -> Result<(), Box<dyn Error>> {
    let Some(emulator) = emulator() else {
        return Ok(());
    };

    let mut options = BlobServiceClientOptions::default();
    options.client_options.per_call_policies = vec![Arc::new(SharedKeyPolicy::new(
        emulator.account.clone(),
        emulator.key.clone(),
    ))];
    let service = BlobServiceClient::new(Url::parse(&emulator.endpoint)?, None, Some(options))?;

    // Container names must be lowercase and may contain hyphens.
    let container_name = format!("fork-contract-{}", unique_suffix());
    let container = service.blob_container_client(&container_name);
    container.create(None).await?;

    // One root blob and two virtual folders. The delimiter read must return
    // the root blob only, plus one prefix per folder.
    for name in ["root.txt", "avatars/2024/01.jpg", "thumbs/x.jpg"] {
        container
            .blob_client(name)
            .upload(RequestContent::from(name.as_bytes().to_vec()), None)
            .await?;
    }

    let options = Some(BlobContainerClientListBlobsOptions {
        delimiter: Some("/".to_owned()),
        ..Default::default()
    });
    let mut pages = container.list_blobs(options)?.into_pages();
    let page = pages
        .try_next()
        .await?
        .expect("a delimited listing returns one page")
        .into_model()?;

    let mut blobs: Vec<String> = page
        .blob_items
        .iter()
        .filter_map(|blob| blob.name.clone())
        .collect();
    let mut prefixes: Vec<String> = page
        .blob_prefixes
        .iter()
        .filter_map(|prefix| prefix.name.clone())
        .collect();
    blobs.sort();
    prefixes.sort();

    assert_eq!(blobs, vec!["root.txt".to_owned()], "root-level blobs");
    assert_eq!(
        prefixes,
        vec!["avatars/".to_owned(), "thumbs/".to_owned()],
        "virtual folder prefixes"
    );

    // A filter with spaces must survive %20 encoding and SharedKey signing.
    // No paging assertion here: the emulator's tag paging is known to differ.
    let _filtered: Vec<_> = service
        .find_blobs_by_tags("\"env\"='test' AND \"kind\"='smoke'", None)?
        .try_collect()
        .await?;

    container.delete(None).await.ok();
    Ok(())
}
