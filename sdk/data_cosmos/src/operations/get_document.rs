use std::marker::PhantomData;

use crate::headers::from_headers::*;
use crate::prelude::*;
use crate::ResourceQuota;
use azure_core::headers::{etag_from_headers, session_token_from_headers, Headers};
use azure_core::{from_json, prelude::*, Response as HttpResponse, SessionToken, StatusCode};
use serde::de::DeserializeOwned;
use time::OffsetDateTime;

#[derive(Debug, Clone)]
pub struct GetDocumentBuilder<T> {
    client: DocumentClient,
    if_match_condition: Option<IfMatchCondition>,
    if_modified_since: Option<IfModifiedSince>,
    consistency_level: Option<ConsistencyLevel>,
    context: Context,
    _document: PhantomData<T>,
}

impl<T> GetDocumentBuilder<T> {
    pub(crate) fn new(client: DocumentClient) -> Self {
        Self {
            client,
            if_match_condition: None,
            if_modified_since: None,
            consistency_level: None,
            context: Context::new(),
            _document: PhantomData,
        }
    }

    setters! {
        consistency_level: ConsistencyLevel => Some(consistency_level),
        if_match_condition: IfMatchCondition => Some(if_match_condition),
        if_modified_since: OffsetDateTime => Some(IfModifiedSince::new(if_modified_since)),
        context: Context => context,
    }
}

impl<T: DeserializeOwned + Send> GetDocumentBuilder<T> {
    /// Convert into a future
    ///
    /// We do not implement `std::future::IntoFuture` because it requires the ability for the
    /// output of the future to be generic which is not possible in Rust (as of 1.59). Once
    /// generic associated types (GATs) stabilize, this will become possible.
    pub fn into_future(self) -> GetDocument<T> {
        Box::pin(async move {
            let mut request = self.client.document_request(azure_core::Method::Get);

            request.insert_headers(&self.if_match_condition);
            request.insert_headers(&self.if_modified_since);
            if let Some(cl) = &self.consistency_level {
                request.insert_headers(cl);
            }
            crate::cosmos_entity::add_as_partition_key_header_serialized(
                self.client.partition_key_serialized(),
                &mut request,
            );

            request.set_body(azure_core::EMPTY_BODY);

            let response = self
                .client
                .cosmos_client()
                .pipeline()
                .send(
                    self.context.clone().insert(ResourceType::Documents),
                    &mut request,
                )
                .await?;

            GetDocumentResponse::try_from(response).await
        })
    }
}

azure_core::future!(GetDocument<T>);

impl<T: DeserializeOwned + Send> std::future::IntoFuture for GetDocumentBuilder<T> {
    type IntoFuture = GetDocument<T>;
    type Output = <GetDocument<T> as std::future::Future>::Output;
    fn into_future(self) -> Self::IntoFuture {
        Self::into_future(self)
    }
}

#[derive(Debug, Clone)]
// note(rylev): clippy seems to be falsely detecting that
// one of the variants is much larger than the other (which
// is not true)
#[allow(clippy::large_enum_variant)]
pub enum GetDocumentResponse<T> {
    Found(FoundDocumentResponse<T>),
    NotFound(NotFoundDocumentResponse),
}

impl<T> GetDocumentResponse<T>
where
    T: DeserializeOwned,
{
    pub async fn try_from(response: HttpResponse) -> azure_core::Result<Self> {
        let (status_code, headers, body) = response.deconstruct();
        let body = body.collect().await?;

        let has_been_found =
            status_code == StatusCode::Ok || status_code == StatusCode::NotModified;

        if has_been_found {
            Ok(GetDocumentResponse::Found(FoundDocumentResponse::try_from(
                &headers, body,
            )?))
        } else {
            Ok(GetDocumentResponse::NotFound(
                NotFoundDocumentResponse::try_from(&headers)?,
            ))
        }
    }
}

#[derive(Debug, Clone)]
pub struct FoundDocumentResponse<T> {
    pub document: Document<T>,
    pub content_location: Option<String>,
    pub last_state_change: OffsetDateTime,
    pub etag: String,
    pub resource_quota: Vec<ResourceQuota>,
    pub resource_usage: Vec<ResourceQuota>,
    pub lsn: u64,
    pub schema_version: String,
    pub alt_content_path: String,
    pub content_path: String,
    pub role: u32,
    pub global_committed_lsn: u64,
    pub number_of_read_regions: u32,
    pub item_lsn: u64,
    pub transport_request_id: u64,
    pub cosmos_llsn: u64,
    pub cosmos_item_llsn: u64,
    pub session_token: SessionToken,
    pub charge: f64,
    pub service_version: String,
    pub activity_id: uuid::Uuid,
    pub gateway_version: String,
    pub date: OffsetDateTime,
}

impl<T> FoundDocumentResponse<T>
where
    T: DeserializeOwned,
{
    fn try_from(headers: &Headers, body: bytes::Bytes) -> azure_core::Result<Self> {
        Ok(Self {
            document: from_json(&body)?,
            content_location: content_location_from_headers(headers)?,
            last_state_change: last_state_change_from_headers(headers)?,
            etag: etag_from_headers(headers)?,
            resource_quota: resource_quota_from_headers_optional(headers)?.unwrap_or_default(),
            resource_usage: resource_usage_from_headers_optional(headers)?.unwrap_or_default(),
            lsn: lsn_from_headers_optional(headers)?.unwrap_or_default(),
            schema_version: schema_version_from_headers_optional(headers)?.unwrap_or_default(),
            alt_content_path: alt_content_path_from_headers_optional(headers)?.unwrap_or_default(),
            content_path: content_path_from_headers_optional(headers)?.unwrap_or_default(),
            role: role_from_headers_optional(headers)?.unwrap_or_default(),
            global_committed_lsn: global_committed_lsn_from_headers_optional(headers)?.unwrap_or_default(),
            number_of_read_regions: number_of_read_regions_from_headers_optional(headers)?.unwrap_or_default(),
            item_lsn: item_lsn_from_headers_optional(headers)?.unwrap_or_default(),
            transport_request_id: transport_request_id_from_headers_optional(headers)?.unwrap_or_default(),
            cosmos_llsn: cosmos_llsn_from_headers_optional(headers)?.unwrap_or_default(),
            cosmos_item_llsn: cosmos_item_llsn_from_headers_optional(headers)?.unwrap_or_default(),
            session_token: session_token_from_headers(headers)?,
            charge: request_charge_from_headers(headers)?,
            service_version: service_version_from_headers_optional(headers)?.unwrap_or_default(),
            activity_id: activity_id_from_headers(headers)?,
            gateway_version: gateway_version_from_headers_optional(headers)?.unwrap_or_default(),
            date: date_from_headers(headers)?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct NotFoundDocumentResponse {
    pub content_location: Option<String>,
    pub last_state_change: OffsetDateTime,
    pub lsn: u64,
    pub schema_version: String,
    pub current_write_quorum: Option<u64>,
    pub current_replica_set_size: Option<u64>,
    pub role: u32,
    pub global_committed_lsn: u64,
    pub number_of_read_regions: u32,
    pub transport_request_id: u64,
    pub cosmos_llsn: u64,
    pub cosmos_quorum_acked_llsn: Option<u64>,
    pub session_token: SessionToken,
    pub charge: f64,
    pub service_version: String,
    pub activity_id: uuid::Uuid,
    pub gateway_version: String,
    pub date: OffsetDateTime,
}

impl NotFoundDocumentResponse {
    fn try_from(headers: &Headers) -> azure_core::Result<Self> {
        Ok(Self {
            content_location: content_location_from_headers(headers)?,
            last_state_change: last_state_change_from_headers(headers)?,
            lsn: lsn_from_headers_optional(headers)?.unwrap_or_default(),
            schema_version: schema_version_from_headers_optional(headers)?.unwrap_or_default(),
            current_write_quorum: current_write_quorum_from_headers_optional(headers)?,
            current_replica_set_size: current_replica_set_size_from_headers_optional(headers)?,
            role: role_from_headers_optional(headers)?.unwrap_or_default(),
            global_committed_lsn: global_committed_lsn_from_headers_optional(headers)?.unwrap_or_default(),
            number_of_read_regions: number_of_read_regions_from_headers_optional(headers)?.unwrap_or_default(),
            transport_request_id: transport_request_id_from_headers_optional(headers)?.unwrap_or_default(),
            cosmos_llsn: cosmos_llsn_from_headers_optional(headers)?.unwrap_or_default(),
            cosmos_quorum_acked_llsn: cosmos_quorum_acked_llsn_from_headers_optional(headers)?,
            session_token: session_token_from_headers(headers)?,
            charge: request_charge_from_headers(headers)?,
            service_version: service_version_from_headers_optional(headers)?.unwrap_or_default(),
            activity_id: activity_id_from_headers(headers)?,
            gateway_version: gateway_version_from_headers_optional(headers)?.unwrap_or_default(),
            date: date_from_headers(headers)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use azure_core::{headers::{self, Headers}, BytesStream, Response};
    use bytes::Bytes;

    #[tokio::test]
    async fn get_document_found_response_allows_sparse_emulator_headers() -> azure_core::Result<()> {
        let mut headers = Headers::new();
        headers.insert(headers::DATE, "Sun, 26 Apr 2026 08:12:49 GMT");
        headers.insert(headers::ETAG, "c13e24f2-0ea7-4111-bcc6-47a04129bf86");
        headers.insert(headers::SESSION_TOKEN, "0:0#0");
        headers.insert(headers::ACTIVITY_ID, "782704bd-d942-d5a2-21d9-2f01980905dc");
        headers.insert(crate::headers::HEADER_REQUEST_CHARGE, "1");
        headers.insert(crate::headers::HEADER_RESOURCE_QUOTA, "documentsCount=-1;");
        headers.insert(crate::headers::HEADER_RESOURCE_USAGE, "documentsCount=0;");
        headers.insert(crate::headers::HEADER_GATEWAY_VERSION, "2.0.0-unknown");
        headers.insert(headers::CONTENT_TYPE, "application/json");

        let response = Response::new(
            StatusCode::Ok,
            headers,
            Box::pin(BytesStream::new(Bytes::from_static(
                br#"{"id":"vnext-raw-seed","text":"vnext smoke","resourceType":{"SearchResourceType":"vehicle","id":"vnext-raw-seed"},"partition":true,"_rid":"AQAAAAUAAAAAAAAZ3I2S6EAAAwY=","_self":"dbs/TML/colls/Search/docs/vnext-raw-seed/","_etag":"c13e24f2-0ea7-4111-bcc6-47a04129bf86","_attachments":"attachments/","_ts":1777191169}"#,
            ))),
        );

        let parsed = GetDocumentResponse::<serde_json::Value>::try_from(response).await?;

        match parsed {
            GetDocumentResponse::Found(found) => {
                assert_eq!(found.lsn, 0);
                assert_eq!(found.schema_version, "");
                assert_eq!(found.alt_content_path, "");
                assert_eq!(found.content_path, "");
                assert_eq!(found.role, 0);
                assert_eq!(found.global_committed_lsn, 0);
                assert_eq!(found.number_of_read_regions, 0);
                assert_eq!(found.item_lsn, 0);
                assert_eq!(found.transport_request_id, 0);
                assert_eq!(found.cosmos_llsn, 0);
                assert_eq!(found.cosmos_item_llsn, 0);
                assert_eq!(found.service_version, "");
                assert_eq!(found.gateway_version, "2.0.0-unknown");
                assert_eq!(found.document.document["id"], "vnext-raw-seed");
            }
            GetDocumentResponse::NotFound(_) => panic!("expected found response"),
        }

        Ok(())
    }

    #[tokio::test]
    async fn get_document_not_found_response_allows_sparse_emulator_headers() -> azure_core::Result<()> {
        let mut headers = Headers::new();
        headers.insert(headers::DATE, "Sun, 26 Apr 2026 08:13:22 GMT");
        headers.insert(headers::SESSION_TOKEN, "0:0#0");
        headers.insert(headers::ACTIVITY_ID, "b7497eb1-68a8-d404-986f-53512be8ed0a");
        headers.insert(crate::headers::HEADER_REQUEST_CHARGE, "1");
        headers.insert(crate::headers::HEADER_RESOURCE_QUOTA, "documentsCount=-1;");
        headers.insert(crate::headers::HEADER_RESOURCE_USAGE, "documentsCount=0;");
        headers.insert(crate::headers::HEADER_GATEWAY_VERSION, "2.0.0-unknown");
        headers.insert(headers::CONTENT_TYPE, "application/json");

        let response = Response::new(
            StatusCode::NotFound,
            headers,
            Box::pin(BytesStream::new(Bytes::from_static(
                br#"{"message":"Resource Not Found. Learn more: https://aka.ms/cosmosdb-tsg-not-found"}"#,
            ))),
        );

        let parsed = GetDocumentResponse::<serde_json::Value>::try_from(response).await?;

        match parsed {
            GetDocumentResponse::Found(_) => panic!("expected not found response"),
            GetDocumentResponse::NotFound(not_found) => {
                assert_eq!(not_found.lsn, 0);
                assert_eq!(not_found.schema_version, "");
                assert_eq!(not_found.role, 0);
                assert_eq!(not_found.global_committed_lsn, 0);
                assert_eq!(not_found.number_of_read_regions, 0);
                assert_eq!(not_found.transport_request_id, 0);
                assert_eq!(not_found.cosmos_llsn, 0);
                assert_eq!(not_found.service_version, "");
                assert_eq!(not_found.gateway_version, "2.0.0-unknown");
            }
        }

        Ok(())
    }
}
