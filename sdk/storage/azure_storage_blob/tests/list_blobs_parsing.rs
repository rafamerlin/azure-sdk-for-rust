use azure_storage_blob::models::ListBlobsResponse;

// Parses a delimited listing page where `<Blob>` and `<BlobPrefix>` children of
// `<Blobs>` interleave, as the List Blobs service emits them.
#[test]
fn parses_interleaved_blobs_and_prefixes() {
    let xml = br#"<?xml version="1.0" encoding="utf-8"?>
<EnumerationResults ContainerName="c">
  <Blobs>
    <Blob><Name>av-vehicle-0001.jpg</Name></Blob>
    <BlobPrefix><Name>avatars/</Name></BlobPrefix>
    <Blob><Name>avatars/2024/01.jpg</Name></Blob>
    <BlobPrefix><Name>thumbs/</Name></BlobPrefix>
  </Blobs>
  <NextMarker></NextMarker>
</EnumerationResults>"#;
    let r: ListBlobsResponse = azure_core::xml::from_xml(xml).unwrap();
    assert_eq!(r.blob_items.len(), 2);
    assert_eq!(r.blob_prefixes.len(), 2);
    assert_eq!(r.blob_prefixes[0].name.as_deref(), Some("avatars/"));
    assert_eq!(r.blob_prefixes[1].name.as_deref(), Some("thumbs/"));
    assert_eq!(r.blob_items[0].name.as_deref(), Some("av-vehicle-0001.jpg"));
}

// Consecutive blobs (the common recursive listing) continue to parse.
#[test]
fn parses_consecutive_blobs() {
    let xml = br#"<?xml version="1.0" encoding="utf-8"?>
<EnumerationResults ContainerName="c">
  <Blobs>
    <Blob><Name>a.jpg</Name></Blob>
    <Blob><Name>b.jpg</Name></Blob>
    <Blob><Name>c.jpg</Name></Blob>
  </Blobs>
</EnumerationResults>"#;
    let r: ListBlobsResponse = azure_core::xml::from_xml(xml).unwrap();
    assert_eq!(r.blob_items.len(), 3);
}

// A realistic recursive page with a continuation marker.
#[test]
fn parses_recursive_listing() {
    let xml = br#"<?xml version="1.0" encoding="utf-8"?>
<EnumerationResults ContainerName="c" MaxResults="5000">
  <Blobs>
    <Blob><Name>folder/0101</Name></Blob><Blob><Name>folder/0102</Name></Blob>
    <Blob><Name>folder/0103</Name></Blob><Blob><Name>folder/0104</Name></Blob>
    <Blob><Name>folder/0105</Name></Blob>
  </Blobs>
  <NextMarker>abc</NextMarker>
</EnumerationResults>"#;
    let r: ListBlobsResponse = azure_core::xml::from_xml(xml).unwrap();
    assert_eq!(r.blob_items.len(), 5);
    assert!(r.blob_prefixes.is_empty());
    assert_eq!(r.next_marker.as_deref(), Some("abc"));
}
