use crate::headers::from_headers::{activity_id_from_headers, request_charge_from_headers};
use crate::prelude::*;
use crate::resources::User;
use azure_core::{
    collect_pinned_stream,
    headers::{continuation_token_from_headers_optional, session_token_from_headers},
    prelude::MaxItemCount,
    Response as HttpResponse, SessionToken,
};
use azure_core::{headers, Context, Continuable, Pageable};

#[derive(Debug, Clone)]
pub struct ListUsersBuilder {
    client: DatabaseClient,
    consistency_level: Option<ConsistencyLevel>,
    max_item_count: MaxItemCount,
    context: Context,
}

impl ListUsersBuilder {
    pub(crate) fn new(client: DatabaseClient) -> Self {
        Self {
            client,
            consistency_level: None,
            max_item_count: MaxItemCount::new(-1),
            context: Context::new(),
        }
    }

    setters! {
        consistency_level: ConsistencyLevel => Some(consistency_level),
        max_item_count: i32 => MaxItemCount::new(max_item_count),
        context: Context => context,
    }

    pub fn into_stream(self) -> ListUsers {
        let make_request = move |continuation: Option<String>| {
            let this = self.clone();
            let ctx = self.context.clone();
            async move {
                let mut request = this.client.cosmos_client().prepare_request_pipeline(
                    &format!("dbs/{}/users", this.client.database_name()),
                    http::Method::GET,
                );

                azure_core::headers::add_optional_header2(&this.consistency_level, &mut request)?;
                azure_core::headers::add_mandatory_header2(&this.max_item_count, &mut request)?;

                if let Some(c) = continuation {
                    let h = http::HeaderValue::from_str(c.as_str())
                        .map_err(azure_core::HttpHeaderError::InvalidHeaderValue)?;
                    request.headers_mut().append(headers::CONTINUATION, h);
                }

                let response = this
                    .client
                    .pipeline()
                    .send(ctx.clone().insert(ResourceType::Users), &mut request)
                    .await?;
                ListUsersResponse::try_from(response).await
            }
        };

        Pageable::new(make_request)
    }
}

pub type ListUsers = Pageable<ListUsersResponse, crate::Error>;

#[derive(Debug, Clone, PartialEq)]
pub struct ListUsersResponse {
    pub users: Vec<User>,
    pub rid: String,
    pub count: u32,
    pub charge: f64,
    pub activity_id: uuid::Uuid,
    pub session_token: SessionToken,
    pub continuation_token: Option<String>,
}

impl ListUsersResponse {
    pub async fn try_from(response: HttpResponse) -> crate::Result<Self> {
        let (_status_code, headers, pinned_stream) = response.deconstruct();
        let body = collect_pinned_stream(pinned_stream).await?;

        #[derive(Deserialize, Debug)]
        pub struct Response {
            #[serde(rename = "_rid")]
            rid: String,
            #[serde(rename = "Users")]
            pub users: Vec<User>,
            #[serde(rename = "_count")]
            pub count: u32,
        }

        let response: Response = serde_json::from_slice(&body)?;

        Ok(Self {
            users: response.users,
            rid: response.rid,
            count: response.count,
            charge: request_charge_from_headers(&headers)?,
            activity_id: activity_id_from_headers(&headers)?,
            session_token: session_token_from_headers(&headers)?,
            continuation_token: continuation_token_from_headers_optional(&headers)?,
        })
    }
}

impl IntoIterator for ListUsersResponse {
    type Item = User;

    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.users.into_iter()
    }
}

impl Continuable for ListUsersResponse {
    fn continuation(&self) -> Option<String> {
        self.continuation_token.clone()
    }
}