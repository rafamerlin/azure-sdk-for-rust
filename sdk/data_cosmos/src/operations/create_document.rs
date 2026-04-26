use crate::cosmos_entity::{add_as_partition_key_header_serialized, serialize_partition_key};
use crate::headers::from_headers::*;
use crate::prelude::*;
use crate::ResourceQuota;
use azure_core::headers::{etag_from_headers, session_token_from_headers};
use azure_core::{prelude::*, StatusCode};
use serde::Serialize;
use time::OffsetDateTime;

use azure_core::Response as HttpResponse;

operation! {
    CreateDocument<D: Serialize + CosmosEntity + Send + 'static>,
    client: CollectionClient,
    document: D,
    ?is_upsert: bool,
    ?indexing_directive: IndexingDirective,
    ?if_match_condition: IfMatchCondition,
    ?if_modified_since: IfModifiedSince,
    ?consistency_level: ConsistencyLevel,
    ?allow_tentative_writes: TentativeWritesAllowance,
    #[skip]
    partition_key: String
}

impl<D: Serialize + CosmosEntity + Send + 'static> CreateDocumentBuilder<D> {
    pub fn partition_key<PK: Serialize>(mut self, partition_key: &PK) -> azure_core::Result<Self> {
        self.partition_key = Some(serialize_partition_key(partition_key)?);
        Ok(self)
    }

    pub fn into_future(self) -> CreateDocument {
        Box::pin(async move {
            let document = self.document;
            let partition_key = match self.partition_key {
                Some(s) => s,
                None => serialize_partition_key(&document.partition_key())?,
            };
            let mut request = self.client.docs_request(azure_core::Method::Post);

            add_as_partition_key_header_serialized(&partition_key, &mut request);
            request.insert_headers(&self.if_match_condition);
            request.insert_headers(&self.if_modified_since);
            if let Some(cl) = &self.consistency_level {
                request.insert_headers(cl);
            }
            let is_upsert = if self.is_upsert.unwrap_or_default() {
                IsUpsert::Yes
            } else {
                IsUpsert::No
            };
            request.insert_headers(&is_upsert);
            request.insert_headers(
                &self
                    .indexing_directive
                    .unwrap_or(IndexingDirective::Default),
            );
            request.insert_headers(
                &self
                    .allow_tentative_writes
                    .unwrap_or(TentativeWritesAllowance::Deny),
            );

            request.set_json(&document)?;
            let response = self
                .client
                .pipeline()
                .send(
                    self.context.clone().insert(ResourceType::Documents),
                    &mut request,
                )
                .await?;

            CreateDocumentResponse::try_from(response).await
        })
    }
}

#[derive(Debug, Clone)]
pub struct CreateDocumentResponse {
    pub document_attributes: DocumentAttributes,
    pub is_update: bool,
    pub last_state_change: OffsetDateTime,
    pub etag: String,
    pub resource_quota: Vec<ResourceQuota>,
    pub resource_usage: Vec<ResourceQuota>,
    pub lsn: u64,
    pub schema_version: String,
    pub alt_content_path: String,
    pub content_path: String,
    pub quorum_acked_lsn: u64,
    pub current_write_quorum: u64,
    pub current_replica_set_size: u64,
    pub role: u32,
    pub global_committed_lsn: u64,
    pub number_of_read_regions: u32,
    pub transport_request_id: u64,
    pub cosmos_llsn: u64,
    pub cosmos_quorum_acked_llsn: u64,
    pub session_token: String,
    pub charge: f64,
    pub service_version: String,
    pub activity_id: uuid::Uuid,
    pub gateway_version: String,
    pub date: OffsetDateTime,
}

impl CreateDocumentResponse {
    pub async fn try_from(response: HttpResponse) -> azure_core::Result<Self> {
        let (status_code, headers, body) = response.deconstruct();
        let body = body.collect().await?;

        Ok(CreateDocumentResponse {
            is_update: status_code == StatusCode::Ok,

            last_state_change: last_state_change_from_headers(&headers)?,
            etag: etag_from_headers(&headers)?,
            resource_quota: resource_quota_from_headers_optional(&headers)?.unwrap_or_default(),
            resource_usage: resource_usage_from_headers_optional(&headers)?.unwrap_or_default(),
            lsn: lsn_from_headers_optional(&headers)?.unwrap_or_default(),
            schema_version: schema_version_from_headers_optional(&headers)?.unwrap_or_default(),
            alt_content_path: alt_content_path_from_headers_optional(&headers)?.unwrap_or_default(),
            content_path: content_path_from_headers_optional(&headers)?.unwrap_or_default(),
            quorum_acked_lsn: quorum_acked_lsn_from_headers_optional(&headers)?.unwrap_or_default(),
            current_write_quorum: current_write_quorum_from_headers_optional(&headers)?.unwrap_or_default(),
            current_replica_set_size: current_replica_set_size_from_headers_optional(&headers)?.unwrap_or_default(),
            role: role_from_headers_optional(&headers)?.unwrap_or_default(),
            global_committed_lsn: global_committed_lsn_from_headers_optional(&headers)?.unwrap_or_default(),
            number_of_read_regions: number_of_read_regions_from_headers_optional(&headers)?.unwrap_or_default(),
            transport_request_id: transport_request_id_from_headers_optional(&headers)?.unwrap_or_default(),
            cosmos_llsn: cosmos_llsn_from_headers_optional(&headers)?.unwrap_or_default(),
            cosmos_quorum_acked_llsn: cosmos_quorum_acked_llsn_from_headers_optional(&headers)?.unwrap_or_default(),
            session_token: session_token_from_headers(&headers)?,
            charge: request_charge_from_headers(&headers)?,
            service_version: service_version_from_headers_optional(&headers)?.unwrap_or_default(),
            activity_id: activity_id_from_headers(&headers)?,
            gateway_version: gateway_version_from_headers_optional(&headers)?.unwrap_or_default(),
            date: date_from_headers(&headers)?,

            document_attributes: DocumentAttributes::try_from(&body)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use azure_core::{headers::{self, Headers}, BytesStream, Response};
    use bytes::Bytes;

    #[tokio::test]
    async fn create_document_response_allows_sparse_emulator_headers() -> azure_core::Result<()> {
        let mut headers = Headers::new();
        headers.insert(headers::DATE, "Sun, 26 Apr 2026 08:12:49 GMT");
        headers.insert(headers::ETAG, "c13e24f2-0ea7-4111-bcc6-47a04129bf86");
        headers.insert(headers::SESSION_TOKEN, "0:0#0");
        headers.insert(headers::ACTIVITY_ID, "02949074-0dcd-ae46-52cc-396e269fcb15");
        headers.insert(crate::headers::HEADER_REQUEST_CHARGE, "1");
        headers.insert(crate::headers::HEADER_RESOURCE_QUOTA, "documentsCount=-1;");
        headers.insert(crate::headers::HEADER_RESOURCE_USAGE, "documentsCount=0;");
        headers.insert(crate::headers::HEADER_GATEWAY_VERSION, "2.0.0-unknown");
        headers.insert(headers::CONTENT_TYPE, "application/json");

        let response = Response::new(
            StatusCode::Created,
            headers,
            Box::pin(BytesStream::new(Bytes::from_static(
                br#"{"id":"vnext-raw-seed","text":"vnext smoke","resourceType":{"SearchResourceType":"vehicle","id":"vnext-raw-seed"},"partition":true,"_rid":"AQAAAAUAAAAAAAAZ3I2S6EAAAwY=","_self":"dbs/TML/colls/Search/docs/vnext-raw-seed/","_etag":"c13e24f2-0ea7-4111-bcc6-47a04129bf86","_attachments":"attachments/","_ts":1777191169}"#,
            ))),
        );

        let parsed = CreateDocumentResponse::try_from(response).await?;

        assert!(!parsed.is_update);
        assert_eq!(parsed.lsn, 0);
        assert_eq!(parsed.schema_version, "");
        assert_eq!(parsed.alt_content_path, "");
        assert_eq!(parsed.content_path, "");
        assert_eq!(parsed.quorum_acked_lsn, 0);
        assert_eq!(parsed.current_write_quorum, 0);
        assert_eq!(parsed.current_replica_set_size, 0);
        assert_eq!(parsed.role, 0);
        assert_eq!(parsed.global_committed_lsn, 0);
        assert_eq!(parsed.number_of_read_regions, 0);
        assert_eq!(parsed.transport_request_id, 0);
        assert_eq!(parsed.cosmos_llsn, 0);
        assert_eq!(parsed.cosmos_quorum_acked_llsn, 0);
        assert_eq!(parsed.service_version, "");
        assert_eq!(parsed.gateway_version, "2.0.0-unknown");
        assert_eq!(parsed.document_attributes.rid(), "AQAAAAUAAAAAAAAZ3I2S6EAAAwY=");

        Ok(())
    }
}
