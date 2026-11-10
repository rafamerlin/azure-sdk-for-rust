# Captured contract fixtures

`fork_contract.rs` compares one real List Blobs page with the fields the
consuming application reads. The test needs two files here:

- `list_blobs_delimited.xml` — the exact response body the service sent.
- `list_blobs_delimited.json` — the values the application expects from it.

Neither file is a secret. Do not commit a SAS token or an account key.

## 1. Capture the body

Use a container that has both root-level blobs and at least one virtual
folder, then request a delimited listing.

With a container SAS URL (create it yourself, keep it out of the shell
history):

```bash
curl -sS "$BLOB_CONTAINER_SAS_URL&restype=container&comp=list&delimiter=%2F" \
  -o sdk/storage/azure_storage_blob/tests/fixtures/list_blobs_delimited.xml
```

The URL must not appear in a commit, a log, or this file.

## 2. Write the expectation

Copy the values the application needs from that page:

```json
{
  "container": "your-container",
  "next_marker": null,
  "blobs": ["root.txt"],
  "prefixes": ["avatars/", "thumbs/"]
}
```

Rules:

- `blobs` lists root-level blob names in service order.
- `prefixes` lists virtual folder names, each ending with `/`.
- `next_marker` is `null` when the page is the last one, otherwise the exact
  string from the response. The test treats an empty marker as `null`, the
  same way the consumer does.
- Add a field to `GoldenPage` in `fork_contract.rs` only when the consumer
  actually reads it.

If `prefixes` is empty, the capture did not include a virtual folder. Capture
again with a container that has one; an empty list would weaken the
`BlobPrefix` assertion.
