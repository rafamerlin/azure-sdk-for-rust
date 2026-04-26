use crate::{headers::from_headers::*, prelude::*, ResourceQuota};
use azure_core::{
    from_json, headers,
    headers::{
        continuation_token_from_headers_optional, item_count_from_headers,
        session_token_from_headers, HeaderValue,
    },
    prelude::*,
    Method, Pageable, Response as HttpResponse, SessionToken,
};
use serde::de::DeserializeOwned;
use serde_json::Value;
use time::OffsetDateTime;
use tracing::warn;

operation! {
    #[stream]
    QueryDocuments,
    client: CollectionClient,
    query: Query,
    ?if_match_condition: IfMatchCondition,
    ?if_modified_since: IfModifiedSince,
    ?max_item_count: MaxItemCount,
    ?consistency_level: ConsistencyLevel,
    ?parallelize_cross_partition_query: ParallelizeCrossPartition,
    ?query_cross_partition: QueryCrossPartition,
    ?partition_range_id: PartitionRangeId,
    ?continuation: Continuation,
    #[skip]
    partition_key_serialized: String
}

impl QueryDocumentsBuilder {
    pub fn partition_key<PK: serde::Serialize>(self, pk: &PK) -> azure_core::Result<Self> {
        Ok(Self {
            partition_key_serialized: Some(crate::cosmos_entity::serialize_partition_key(pk)?),
            ..self
        })
    }

    pub fn into_stream<T>(self) -> QueryDocuments<T>
    where
        T: DeserializeOwned + Send + Sync,
    {
        let make_request = move |continuation: Option<Continuation>| {
            let this = self.clone();
            let ctx = self.context.clone();
            async move {
                let mut request = this.client.cosmos_client().request(
                    &format!(
                        "dbs/{}/colls/{}/docs",
                        this.client.database_client().database_name(),
                        this.client.collection_name()
                    ),
                    Method::Post,
                );

                // signal that this is a query
                request.insert_header(
                    crate::headers::HEADER_DOCUMENTDB_ISQUERY,
                    HeaderValue::from_static("true"),
                );
                request.insert_header(
                    headers::CONTENT_TYPE,
                    HeaderValue::from_static("application/query+json"),
                );

                request.insert_headers(&this.if_match_condition);
                request.insert_headers(&this.if_modified_since);
                if let Some(cl) = &this.consistency_level {
                    request.insert_headers(cl);
                }
                request.insert_headers(&this.max_item_count.unwrap_or_default());
                request.insert_headers(&this.query_cross_partition.unwrap_or_default());
                request.insert_headers(&this.partition_range_id);

                request.set_json(&this.query)?;
                if let Some(partition_key_serialized) = this.partition_key_serialized.as_ref() {
                    crate::cosmos_entity::add_as_partition_key_header_serialized(
                        partition_key_serialized,
                        &mut request,
                    );
                }

                let continuation = continuation.or(this.continuation);
                request.insert_headers(&continuation);

                let response = this
                    .client
                    .pipeline()
                    .send(ctx.clone().insert(ResourceType::Documents), &mut request)
                    .await?;
                QueryDocumentsResponse::try_from(response).await
            }
        };

        Pageable::new(make_request)
    }
}

pub type QueryDocuments<T> = Pageable<QueryDocumentsResponse<T>, azure_core::error::Error>;

/// A response from querying for documents.
#[derive(Debug, Clone)]
pub struct QueryDocumentsResponse<T> {
    pub query_response_meta: QueryResponseMeta,
    pub results: Vec<(T, Option<DocumentAttributes>)>,
    pub last_state_change: OffsetDateTime,
    pub resource_quota: Vec<ResourceQuota>,
    pub resource_usage: Vec<ResourceQuota>,
    pub lsn: u64,
    pub item_count: u32,
    pub schema_version: String,
    pub alt_content_path: String,
    pub content_path: String,
    pub quorum_acked_lsn: Option<u64>,
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
    pub continuation_token: Option<Continuation>,
}

impl<T> QueryDocumentsResponse<T> {
    /// An iterator over the documents included in the response
    pub fn documents(&self) -> impl Iterator<Item = &T> {
        self.results.iter().map(|(d, _)| d)
    }
}

impl<T> QueryDocumentsResponse<T>
where
    T: DeserializeOwned,
{
    pub async fn try_from(response: HttpResponse) -> azure_core::Result<Self> {
        let (_status_code, headers, body) = response.deconstruct();
        let body = body.collect().await?;

        let inner: Value = from_json(&body)?;
        let results = if let Value::Array(documents) = &inner["Documents"] {
            documents
                .iter()
                .map(|doc| {
                    let document: T = serde_json::from_value(doc.to_owned())?;
                    // If we have all the necessary fields to construct a `DocumentAttributes`
                    // we do, otherwise we just add a raw struct.
                    let attributes = serde_json::from_value(doc.to_owned());
                    if let Err(error) = &attributes {
                        warn!("error deserializing document attributes: {:#?}", error);
                    }
                    Ok((document, attributes.ok()))
                })
                .collect::<azure_core::Result<Vec<_>>>()?
        } else {
            Vec::new()
        };

        Ok(QueryDocumentsResponse {
            results,
            last_state_change: last_state_change_from_headers(&headers)?,
            resource_quota: resource_quota_from_headers_optional(&headers)?.unwrap_or_default(),
            resource_usage: resource_usage_from_headers_optional(&headers)?.unwrap_or_default(),
            lsn: lsn_from_headers_optional(&headers)?.unwrap_or_default(),
            item_count: item_count_from_headers(&headers)?,
            schema_version: schema_version_from_headers_optional(&headers)?.unwrap_or_default(),
            alt_content_path: alt_content_path_from_headers_optional(&headers)?.unwrap_or_default(),
            content_path: content_path_from_headers_optional(&headers)?.unwrap_or_default(),
            quorum_acked_lsn: quorum_acked_lsn_from_headers_optional(&headers)?,
            current_write_quorum: current_write_quorum_from_headers_optional(&headers)?,
            current_replica_set_size: current_replica_set_size_from_headers_optional(&headers)?,
            role: role_from_headers_optional(&headers)?.unwrap_or_default(),
            global_committed_lsn: global_committed_lsn_from_headers_optional(&headers)?
                .unwrap_or_default(),
            number_of_read_regions: number_of_read_regions_from_headers_optional(&headers)?
                .unwrap_or_default(),
            transport_request_id: transport_request_id_from_headers_optional(&headers)?
                .unwrap_or_default(),
            cosmos_llsn: cosmos_llsn_from_headers_optional(&headers)?.unwrap_or_default(),
            cosmos_quorum_acked_llsn: cosmos_quorum_acked_llsn_from_headers_optional(&headers)?,
            session_token: session_token_from_headers(&headers)?,
            charge: request_charge_from_headers(&headers)?,
            service_version: service_version_from_headers_optional(&headers)?.unwrap_or_default(),
            activity_id: activity_id_from_headers(&headers)?,
            gateway_version: gateway_version_from_headers_optional(&headers)?.unwrap_or_default(),
            continuation_token: continuation_token_from_headers_optional(&headers)?,
            date: date_from_headers(&headers)?,
            query_response_meta: from_json(&body)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use azure_core::{headers::Headers, BytesStream, Response, StatusCode};
    use bytes::Bytes;

    #[tokio::test]
    async fn query_documents_response_allows_sparse_emulator_headers() -> azure_core::Result<()> {
        let mut headers = Headers::new();
        headers.insert(headers::DATE, "Sun, 26 Apr 2026 06:28:00 GMT");
        headers.insert(headers::CONTENT_TYPE, "application/json");
        headers.insert(headers::ACTIVITY_ID, "11111111-1111-1111-1111-111111111111");
        headers.insert(headers::SESSION_TOKEN, "0:-1#1");
        headers.insert(headers::ITEM_COUNT, "1");
        headers.insert(headers::CONTINUATION, "next-page");
        headers.insert(crate::headers::HEADER_REQUEST_CHARGE, "1.0");

        let response = Response::new(
            StatusCode::Ok,
            headers,
            Box::pin(BytesStream::new(Bytes::from_static(
                br#"{"_rid":"RID","_count":1,"Documents":[{"id":"doc-1"}]}"#,
            ))),
        );

        let parsed = QueryDocumentsResponse::<serde_json::Value>::try_from(response).await?;

        assert_eq!(parsed.results.len(), 1);
        assert_eq!(parsed.item_count, 1);
        assert_eq!(parsed.lsn, 0);
        assert!(parsed.resource_quota.is_empty());
        assert!(parsed.resource_usage.is_empty());
        assert_eq!(parsed.schema_version, "");
        assert_eq!(parsed.alt_content_path, "");
        assert_eq!(parsed.content_path, "");
        assert_eq!(parsed.role, 0);
        assert_eq!(parsed.global_committed_lsn, 0);
        assert_eq!(parsed.number_of_read_regions, 0);
        assert_eq!(parsed.transport_request_id, 0);
        assert_eq!(parsed.cosmos_llsn, 0);
        assert_eq!(parsed.service_version, "");
        assert_eq!(parsed.gateway_version, "");
        assert_eq!(parsed.continuation_token.unwrap().value().as_str(), "next-page");

        Ok(())
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct QueryResponseMeta {
    #[serde(rename = "_rid")]
    pub rid: String,
    #[serde(rename = "_count")]
    pub count: u64,
}

impl<T> Continuable for QueryDocumentsResponse<T> {
    type Continuation = Continuation;
    fn continuation(&self) -> Option<Self::Continuation> {
        self.continuation_token.clone()
    }
}
