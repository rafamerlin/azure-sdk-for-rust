use crate::{
    headers::from_headers::*,
    prelude::*,
    resources::collection::{IndexingPolicy, PartitionKey},
};
use azure_core::{
    headers::{etag_from_headers_optional, session_token_from_headers},
    Response as HttpResponse,
};
use time::OffsetDateTime;

operation! {
    CreateCollection,
    client: DatabaseClient,
    collection_name: String,
    partition_key: PartitionKey,
    ?consistency_level: ConsistencyLevel,
    ?indexing_policy: IndexingPolicy,
    ?offer: Offer
}

impl CreateCollectionBuilder {
    pub fn into_future(self) -> CreateCollection {
        Box::pin(async move {
            let mut request = self.client.collections_request(azure_core::Method::Post);
            request.insert_headers(&self.offer);
            if let Some(cl) = &self.consistency_level {
                request.insert_headers(cl);
            }

            /// Body for the create collection request
            #[derive(Serialize)]
            struct CreateCollectionBody<'a> {
                pub id: &'a str,
                #[serde(rename = "indexingPolicy", skip_serializing_if = "Option::is_none")]
                pub indexing_policy: &'a Option<IndexingPolicy>,
                #[serde(rename = "partitionKey")]
                pub partition_key: &'a PartitionKey,
            }

            let collection = CreateCollectionBody {
                id: &self.collection_name,
                indexing_policy: &self.indexing_policy,
                partition_key: &self.partition_key,
            };

            request.set_json(&collection)?;

            let response = self
                .client
                .cosmos_client()
                .send(request, self.context.clone(), ResourceType::Collections)
                .await?;

            CreateCollectionResponse::try_from(response).await
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CreateCollectionResponse {
    pub collection: Collection,
    pub charge: f64,
    pub activity_id: uuid::Uuid,
    pub etag: String,
    pub session_token: String,
    pub last_state_change: OffsetDateTime,
    pub schema_version: String,
    pub service_version: String,
    pub gateway_version: String,
    pub alt_content_path: String,
    pub quorum_acked_lsn: u64,
    pub current_write_quorum: u64,
    pub current_replica_set_size: u64,
}

impl CreateCollectionResponse {
    pub async fn try_from(response: HttpResponse) -> azure_core::Result<Self> {
        let (_status_code, headers, body) = response.deconstruct();
        Ok(Self {
            collection: body.json().await?,
            charge: request_charge_from_headers(&headers)?,
            activity_id: activity_id_from_headers(&headers)?,
            etag: etag_from_headers_optional(&headers)?.unwrap_or_default(),
            session_token: session_token_from_headers(&headers)?,
            last_state_change: last_state_change_from_headers(&headers)?,
            schema_version: schema_version_from_headers_optional(&headers)?.unwrap_or_default(),
            service_version: service_version_from_headers_optional(&headers)?.unwrap_or_default(),
            gateway_version: gateway_version_from_headers_optional(&headers)?.unwrap_or_default(),
            alt_content_path: alt_content_path_from_headers_optional(&headers)?.unwrap_or_default(),
            quorum_acked_lsn: quorum_acked_lsn_from_headers_optional(&headers)?.unwrap_or_default(),
            current_write_quorum: current_write_quorum_from_headers_optional(&headers)?.unwrap_or_default(),
            current_replica_set_size: current_replica_set_size_from_headers_optional(&headers)?.unwrap_or_default(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use azure_core::{
        BytesStream, Response, StatusCode,
        headers::{self, Headers},
    };
    use bytes::Bytes;

    #[tokio::test]
    async fn create_collection_response_allows_sparse_emulator_headers() -> azure_core::Result<()> {
        let mut headers = Headers::new();
        headers.insert(headers::DATE, "Sun, 26 Apr 2026 10:30:00 GMT");
        headers.insert(headers::SESSION_TOKEN, "0:0#0");
        headers.insert(headers::ACTIVITY_ID, "11111111-2222-3333-4444-555555555555");
        headers.insert(crate::headers::HEADER_REQUEST_CHARGE, "1");
        headers.insert(crate::headers::HEADER_GATEWAY_VERSION, "2.0.0-unknown");
        headers.insert(headers::CONTENT_TYPE, "application/json");

        let response = Response::new(
            StatusCode::Created,
            headers,
            Box::pin(BytesStream::new(Bytes::from_static(
                br#"{"id":"TestSdk","indexingPolicy":{"automatic":true,"indexingMode":"consistent","includedPaths":[],"excludedPaths":[]},"partitionKey":{"paths":["/id"],"kind":"Hash"},"_rid":"AQAAAAUAAAAAAAAAAAAAAA==","_ts":1777199400,"_self":"dbs/TML/colls/TestSdk/","_etag":"00000000-0000-0000-0000-000000000000","_docs":"docs/","_sprocs":"sprocs/","_triggers":"triggers/","_udfs":"udfs/","_conflicts":"conflicts/"}"#,
            ))),
        );

        let parsed = CreateCollectionResponse::try_from(response).await?;

        assert_eq!(parsed.collection.id, "TestSdk");
        assert_eq!(parsed.etag, "");
        assert_eq!(parsed.schema_version, "");
        assert_eq!(parsed.service_version, "");
        assert_eq!(parsed.gateway_version, "2.0.0-unknown");
        assert_eq!(parsed.alt_content_path, "");
        assert_eq!(parsed.quorum_acked_lsn, 0);
        assert_eq!(parsed.current_write_quorum, 0);
        assert_eq!(parsed.current_replica_set_size, 0);

        Ok(())
    }
}
