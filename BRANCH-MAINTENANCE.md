# Branch maintenance — `blob-shared-key-support`

This document describes every change this branch carries relative to
`Azure/azure-sdk-for-rust` main, so that any maintenance session (human or
assistant) can reapply the delta after pulling Microsoft's main into this
branch. Keep the file close to the actual diff.

Last verified: 2026-10-25, code base `6b155e22fc` (origin/main `40f556ac4d`).

## Carried commits

Base: `origin/main` (Azure/azure-sdk-for-rust, fetched 2026-10-24 at `40f556ac4d`).

| Commit | Content | Exact files |
|---|---|---|
| `5cff8fc8e1` C1 | `QueryBuilder::build()` rewrites literal `+` to `%20` after `form_urlencoded` serialization (Blob tag search `where` expressions with spaces must be `%20`; values containing `+` are already `%2B`). Tests: `test_query_builder_with_special_chars` now asserts `hello%20world`; new tests `test_query_builder_encodes_spaces_as_percent_20`, `test_query_builder_encodes_plus_in_values`. | `sdk/core/typespec_client_core/src/http/mod.rs`, `sdk/core/typespec_client_core/CHANGELOG.md`, `sdk/core/azure_core/CHANGELOG.md` |
| `096b058745` C2 | `SharedKeyPolicy`: per-call `Policy` implementing legacy SharedKey signing for Blob and Queue, incl. path-style Azurite endpoints. | `sdk/storage/azure_storage_common/{Cargo.toml, CHANGELOG.md, src/lib.rs, src/shared_key.rs}`, `sdk/storage/azure_storage_blob/src/clients/mod.rs`, `sdk/storage/azure_storage_queue/src/lib.rs` |
| (C3) | Delimited listing: `delimiter` option on `BlobContainerClientListBlobsOptions` + `delimiter` query param, `BlobPrefix` model, and `ListBlobsResponse::blob_prefixes`. Parsing is a hand-written `MapAccess` visitor (`Blob_itemsInventory` in `xml_helpers.rs`) because quick-xml only merges ADJACENT repeated siblings — the interleaved `<Blob>`/`<BlobPrefix>` form errors as "duplicate field" under generated-style deserializers. Tests live in `sdk/storage/azure_storage_blob/tests/list_blobs_parsing.rs` (includes interleaved, consecutive, recursive-with-marker). Verified live against a local Azurite: root page = 1 blob + 2 prefixes; delimiter + `maxresults=2` pages correctly through `NextMarker`.

Emulator-vs-service note (affects CONSUMER tag-search tests, not this code path): local Azurite 3.35.0 tag queries return ⌈maxresults/2⌉ items and an EMPTY `NextMarker` when `maxresults` is set (raw wire bytes show the fault pre-parse; `list_blobs` paging is unaffected). Any consumer paging test against tag search must degrade the marker branch on the emulator while keeping full-drain set assertions strict.| `sdk/storage/azure_storage_blob/src/generated/models/{method_options.rs, models.rs, xml_helpers.rs}`, `sdk/storage/azure_storage_blob/src/generated/clients/blob_container_client.rs`, `sdk/storage/azure_storage_blob/tests/list_blobs_parsing.rs`, `sdk/storage/azure_storage_blob/CHANGELOG.md` |

C2 contract details to preserve on reapply:

- `azure_storage_common/Cargo.toml`: `async-trait = "0.1"` (plain version —
  NOT workspace-inherited; the root `[workspace.dependencies]` has no
  `async_trait` entry), plus:
  ```toml
  [features]
  default = ["shared_key"]
  shared_key = ["azure_core/hmac_rust"]
  ```
- `azure_storage_common/src/lib.rs`: `#[cfg(feature = "shared_key")] pub mod shared_key;`
- `azure_storage_blob/src/clients/mod.rs`: `pub use azure_storage_common::shared_key;`
  with a compiled `no_run` doc example (the doctest lives in the BLOB crate —
  `azure_storage_common` does not depend on it; the dependency direction is
  reversed).
- `azure_storage_queue/src/lib.rs`: same re-export, no doc example.
- Changelog entry under the top Unreleased `Features Added`.

Deliberately NOT carried (all decided 2026-10-24):

- The `azurite_find_blobs_by_tags.rs` verification example and its
  `hmac_rust` dev-dependency (the consuming project has its own harness for
  which this file is not the record).
- The previous-generation fork fixes `16347585` / `07410b6`
  (find-by-tags `next_marker` exposure and `marker` / `maxresults` builder
  options) — the new generation ships `FindBlobsByTagsOptions` with both.
- The Cosmos emulator-relax parsing fix `77ba6239` — no consumer imports
  `azure_data_cosmos` yet; port it only when that changes.

## Reapply procedure after pulling Microsoft main

1. In a worktree on this branch: `git fetch origin main`.
2. `git rebase origin/main` (or merge; rebase keeps the commits legible).
3. Expected conflict hotspots:
   - `sdk/core/typespec_client_core/src/http/mod.rs` — the `QueryBuilder` test
     region. Reassert `%20` semantics: `hello+world` -> `hello%20world`, and
     keep both new test functions.
   - The two core `CHANGELOG.md` files — move the entries into the current
     top Unreleased sections; never merge text blindly.
   - `azure_storage_common/Cargo.toml` — the `[features]` block and the
     `async-trait` dependency line.
   - C3 (delimited listing, once it exists on this branch) — generated files;
     see the section below.
4. Do NOT reapply anything in `generated/` when Microsoft regenerates the
   same area — check their commits for the same change first. If their
   regeneration drops the option again, reapply on top and note it here.
5. SDK validation battery (all commands from the worktree):

   ```bash
   cargo test -q -p typespec_client_core --lib
   cargo test -q -p azure_core --lib
   cargo test -q -p azure_storage_common
   cargo test -q -p azure_storage_blob --doc
   cargo fmt -p typespec_client_core -p azure_core -p azure_storage_common -p azure_storage_blob -p azure_storage_queue -- --check
   cargo clippy -p typespec_client_core -p azure_core -p azure_storage_common -p azure_storage_blob -p azure_storage_queue --all-targets
   ```

6. Then re-verify the consuming project against this branch: its unit suite
   and its Azurite live checks. If the emulator is fresh, run the project's
   provisioning test once first. Report only counts; never store project
   details here.

## DONE — delimited listing (`delimiter` + `BlobPrefix`) → commit C3

Status: implemented, tested (unit + live Azurite), committed as C3. See the
commit table above; keep this section for reapply context. Key reapply rule:
if Microsoft regenerates `ListBlobsResponse` or the list_blobs options, expect
their generated deserializer to Drop `BlobPrefix` again — reassert:
1. the `delimiter` option field + `delimiter` URL pair in `list_blobs`,
2. the `BlobPrefix` model,
3. `ListBlobsResponse::blob_prefixes` with the manual `Deserialize` impl
   (`ListBlobsResponseShadow` → `Blob_itemsInventory`: a single
   `MapAccess` visitor that appends per key; do NOT try two serde-renamed
   "Blobs" fields — quick-xml rejects non-adjacent same-key siblings).

Why: Microsoft's new generated SDK dropped the `delimiter` request parameter
and the `BlobPrefix` model from List Blobs (the previous generation carried
both). Hierarchical (virtual-folder) listings were therefore impossible:
prefix filtering was recursive, root-level views mixed folder descendants into
page rows, and page arithmetic changed.

Targets (fork-side edits to generated code paths is an accepted cost of this
fork; upstream would need a TypeSpec-level request instead) — all apply to
the C3 commit when reapplying:

- `sdk/storage/azure_storage_blob/src/generated/models/method_options.rs`
  (`BlobContainerClientListBlobsOptions`): the
  `pub delimiter: Option<String>` field (typically `/`), and the
  `delimiter` query parameter in the URL building of `list_blobs` in
  `generated/clients/blob_container_client.rs`.
- `sdk/storage/azure_storage_blob/src/generated/models/models.rs`
  (`ListBlobsResponse`): `blob_prefixes: Vec<BlobPrefix>` parsed from the
  repeated `<BlobPrefix><Name>…</Name></BlobPrefix>` children of `<Blobs>`
  via `Blob_itemsInventory` in `generated/models/xml_helpers.rs`.
- `BlobPrefix` is exported through `pub use models::*`.
- Consumer side (in the consuming project, not here): switch root-level
  reads to delimiter mode where one-level semantics are wanted; keep
  recursive prefix reads for opened folders; add one nested-tree live check
  (populate root blobs + `a/b/c`-style levels, assert the delimited root
  page returns exactly the root blobs and one row per first-level prefix).

Commit message draft: title "Add delimited listing support to List Blobs"
(under 50 characters), body explaining the `delimiter` parameter, the
`BlobPrefix` parsing, the upstream omission, and pointing to this file.
