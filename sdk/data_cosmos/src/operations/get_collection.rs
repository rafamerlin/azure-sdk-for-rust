use crate::{headers::from_headers::*, prelude::*};
use azure_core::{
    headers::{content_type_from_headers, etag_from_headers_optional, session_token_from_headers},
    Response as HttpResponse,
};
use time::OffsetDateTime;

operation! {
    GetCollection,
    client: CollectionClient,
    ?consistency_level: ConsistencyLevel
}

impl GetCollectionBuilder {
    pub fn into_future(self) -> GetCollection {
        Box::pin(async move {
            let mut request = self.client.collection_request(azure_core::Method::Get);

            if let Some(cl) = &self.consistency_level {
                request.insert_headers(cl);
            }

            let response = self
                .client
                .pipeline()
                .send(
                    self.context.clone().insert(ResourceType::Collections),
                    &mut request,
                )
                .await?;

            GetCollectionResponse::try_from(response).await
        })
    }
}

#[derive(Debug, Clone)]
pub struct GetCollectionResponse {
    pub collection: Collection,
    pub last_state_change: OffsetDateTime,
    pub etag: String,
    pub collection_partition_index: u64,
    pub collection_service_index: u64,
    pub lsn: u64,
    pub schema_version: String,
    pub alt_content_path: String,
    pub content_path: String,
    pub global_committed_lsn: u64,
    pub number_of_read_regions: u32,
    pub item_lsn: u64,
    pub transport_request_id: u64,
    pub cosmos_llsn: u64,
    pub cosmos_item_llsn: u64,
    pub charge: f64,
    pub service_version: String,
    pub activity_id: uuid::Uuid,
    pub session_token: String,
    pub gateway_version: String,
    pub server: String,
    pub xp_role: u32,
    pub content_type: String,
    pub content_location: Option<String>,
    pub date: OffsetDateTime,
}

impl GetCollectionResponse {
    pub async fn try_from(response: HttpResponse) -> azure_core::Result<Self> {
        let (_status_code, headers, body) = response.deconstruct();

        Ok(Self {
            collection: body.json().await?,
            last_state_change: last_state_change_from_headers(&headers)?,
            etag: etag_from_headers_optional(&headers)?.unwrap_or_default(),
            collection_partition_index: collection_partition_index_from_headers_optional(&headers)?.unwrap_or_default(),
            collection_service_index: collection_service_index_from_headers_optional(&headers)?.unwrap_or_default(),
            lsn: lsn_from_headers_optional(&headers)?.unwrap_or_default(),
            schema_version: schema_version_from_headers_optional(&headers)?.unwrap_or_default(),
            alt_content_path: alt_content_path_from_headers_optional(&headers)?.unwrap_or_default(),
            content_path: content_path_from_headers_optional(&headers)?.unwrap_or_default(),
            global_committed_lsn: global_committed_lsn_from_headers_optional(&headers)?.unwrap_or_default(),
            number_of_read_regions: number_of_read_regions_from_headers_optional(&headers)?.unwrap_or_default(),
            item_lsn: item_lsn_from_headers_optional(&headers)?.unwrap_or_default(),
            transport_request_id: transport_request_id_from_headers_optional(&headers)?.unwrap_or_default(),
            cosmos_llsn: cosmos_llsn_from_headers_optional(&headers)?.unwrap_or_default(),
            cosmos_item_llsn: cosmos_item_llsn_from_headers_optional(&headers)?.unwrap_or_default(),
            charge: request_charge_from_headers(&headers)?,
            service_version: service_version_from_headers_optional(&headers)?.unwrap_or_default(),
            activity_id: activity_id_from_headers(&headers)?,
            session_token: session_token_from_headers(&headers)?,
            gateway_version: gateway_version_from_headers_optional(&headers)?.unwrap_or_default(),
            server: server_from_headers_optional(&headers)?.unwrap_or_default(),
            xp_role: role_from_headers_optional(&headers)?.unwrap_or_default(),
            content_type: content_type_from_headers(&headers)?,
            content_location: content_location_from_headers(&headers)?,
            date: date_from_headers(&headers)?,
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
    async fn get_collection_response_allows_sparse_emulator_headers() -> azure_core::Result<()> {
        let mut headers = Headers::new();
        headers.insert(headers::DATE, "Sun, 26 Apr 2026 10:31:00 GMT");
        headers.insert(headers::SESSION_TOKEN, "0:0#0");
        headers.insert(headers::ACTIVITY_ID, "66666666-7777-8888-9999-000000000000");
        headers.insert(crate::headers::HEADER_REQUEST_CHARGE, "1");
        headers.insert(crate::headers::HEADER_GATEWAY_VERSION, "2.0.0-unknown");
        headers.insert(headers::CONTENT_TYPE, "application/json");

        let response = Response::new(
            StatusCode::Ok,
            headers,
            Box::pin(BytesStream::new(Bytes::from_static(
                br#"{"id":"TestSdk","indexingPolicy":{"automatic":true,"indexingMode":"consistent","includedPaths":[],"excludedPaths":[]},"partitionKey":{"paths":["/id"],"kind":"Hash"},"_rid":"AQAAAAUAAAAAAAAAAAAAAA==","_ts":1777199460,"_self":"dbs/TML/colls/TestSdk/","_etag":"00000000-0000-0000-0000-000000000000","_docs":"docs/","_sprocs":"sprocs/","_triggers":"triggers/","_udfs":"udfs/","_conflicts":"conflicts/"}"#,
            ))),
        );

        let parsed = GetCollectionResponse::try_from(response).await?;

        assert_eq!(parsed.collection.id, "TestSdk");
        assert_eq!(parsed.etag, "");
        assert_eq!(parsed.collection_partition_index, 0);
        assert_eq!(parsed.collection_service_index, 0);
        assert_eq!(parsed.lsn, 0);
        assert_eq!(parsed.schema_version, "");
        assert_eq!(parsed.alt_content_path, "");
        assert_eq!(parsed.content_path, "");
        assert_eq!(parsed.global_committed_lsn, 0);
        assert_eq!(parsed.number_of_read_regions, 0);
        assert_eq!(parsed.item_lsn, 0);
        assert_eq!(parsed.transport_request_id, 0);
        assert_eq!(parsed.cosmos_llsn, 0);
        assert_eq!(parsed.cosmos_item_llsn, 0);
        assert_eq!(parsed.service_version, "");
        assert_eq!(parsed.gateway_version, "2.0.0-unknown");
        assert_eq!(parsed.server, "");
        assert_eq!(parsed.xp_role, 0);

        Ok(())
    }
}
