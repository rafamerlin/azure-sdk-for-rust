// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

//! Offline contract tests for the fork-only behaviour carried by this branch.
//!
//! These tests run with plain `cargo test`. The captured-page test uses the
//! service response committed under `tests/fixtures/`; when that file is
//! absent the test skips, and `eng/scripts/fork-verify.sh` fails instead so a
//! rebase cannot silently delete the fixture.

use std::{error::Error, fs, path::PathBuf};

use azure_storage_blob::{models::ListBlobsResponse, shared_key::SharedKeyPolicy};
use serde::Deserialize;

/// The fields the consuming application reads from one delimited page.
///
/// This is the contract, not the whole response. Add a field here only when
/// the consumer actually depends on it.
#[derive(Debug, Deserialize)]
struct GoldenPage {
    container: Option<String>,
    next_marker: Option<String>,
    blobs: Vec<String>,
    prefixes: Vec<String>,
}

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

/// The SharedKey policy must stay reachable with the crate's default
/// features, because the branch turns it on by default.
///
/// The key below is a placeholder. It is never sent anywhere.
#[test]
fn shared_key_policy_is_available_by_default() {
    let policy = SharedKeyPolicy::new("devstoreaccount1", "placeholder-not-a-key");
    assert!(format!("{policy:?}").contains("SharedKeyPolicy"));
}

/// Parses the captured service response and compares it with the golden
/// fields.
///
/// The XML file is the exact body the service sent. The JSON file is the
/// expectation: the values the application needs from that page.
#[test]
fn captured_delimited_page_matches_the_golden_fields() -> Result<(), Box<dyn Error>> {
    let xml_path = fixture("list_blobs_delimited.xml");
    let json_path = fixture("list_blobs_delimited.json");

    match (xml_path.exists(), json_path.exists()) {
        (false, false) => {
            eprintln!(
                "SKIP captured_delimited_page_matches_the_golden_fields: \
                 no capture under tests/fixtures/. See tests/fixtures/README.md."
            );
            return Ok(());
        }
        (true, false) | (false, true) => {
            panic!(
                "incomplete capture: {} and {} must exist together",
                xml_path.display(),
                json_path.display()
            );
        }
        (true, true) => {}
    }

    let xml = fs::read(&xml_path)?;
    let page: ListBlobsResponse = azure_core::xml::from_xml(&xml)?;
    let golden: GoldenPage = serde_json::from_slice(&fs::read(&json_path)?)?;

    let blobs: Vec<String> = page
        .blob_items
        .iter()
        .filter_map(|blob| blob.name.clone())
        .collect();
    let prefixes: Vec<String> = page
        .blob_prefixes
        .iter()
        .filter_map(|prefix| prefix.name.clone())
        .collect();
    // The consumer treats an empty marker as "no more pages".
    let next_marker = page.next_marker.filter(|marker| !marker.is_empty());

    assert_eq!(page.container_name, golden.container, "container name");
    assert_eq!(next_marker, golden.next_marker, "next marker");
    assert_eq!(blobs, golden.blobs, "blob names");
    assert_eq!(prefixes, golden.prefixes, "blob prefixes");

    Ok(())
}
