// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

pub use crate::generated::clients::{QueueServiceClient, QueueServiceClientOptions};

use crate::{
    clients::QueueClient,
    generated::models::{QueueClientCreateOptions, QueueClientDeleteOptions},
    logging::apply_storage_logging_defaults,
};
use azure_core::{
    http::{NoFormat, Pipeline, Response, Url},
    tracing, Result,
};
use azure_storage_common::{SharedKeyAuthorizationPolicy, StorageCredential};
use std::sync::Arc;

fn create_auth_policy(
    credential: &StorageCredential,
) -> Arc<dyn azure_core::http::policies::Policy> {
    match credential {
        StorageCredential::TokenCredential(token_credential) => Arc::new(
            azure_core::http::policies::auth::BearerTokenAuthorizationPolicy::new(
                token_credential.clone(),
                vec!["https://storage.azure.com/.default"],
            ),
        ),
        StorageCredential::SharedKey(shared_key_credential) => Arc::new(
            SharedKeyAuthorizationPolicy::new(shared_key_credential.clone()),
        ),
    }
}

impl QueueServiceClient {
    /// Creates a new QueueServiceClient.
    ///
    /// Supports both Entra ID token-based and Shared Key authentication.
    ///
    /// # Arguments
    ///
    /// * `endpoint` - The full URL of the Azure storage account, for example `https://myaccount.queue.core.windows.net/`
    /// * `credential` - An optional [`StorageCredential`] for authentication.
    ///   If `None`, the URL must contain authentication information (e.g., SAS token).
    /// * `options` - Optional configuration for the client.
    #[tracing::new("Storage.Queues.Service")]
    pub fn new(
        endpoint: &str,
        credential: Option<impl Into<StorageCredential>>,
        options: Option<QueueServiceClientOptions>,
    ) -> Result<Self> {
        let endpoint = Url::parse(endpoint)?;
        let mut options = options.unwrap_or_default();
        apply_storage_logging_defaults(&mut options.client_options);

        if let Some(credential) = credential {
            let credential = credential.into();
            if matches!(credential, StorageCredential::TokenCredential(_))
                && !endpoint.scheme().starts_with("https")
            {
                return Err(azure_core::Error::with_message(
                    azure_core::error::ErrorKind::Other,
                    format!("{endpoint} must use https"),
                ));
            }
            let auth_policy = create_auth_policy(&credential);
            options.client_options.per_try_policies.push(auth_policy);
        }

        let pipeline = Pipeline::new(
            option_env!("CARGO_PKG_NAME"),
            option_env!("CARGO_PKG_VERSION"),
            options.client_options.clone(),
            Vec::default(),
            Vec::default(),
            None,
        );

        Ok(Self {
            endpoint,
            version: options.version,
            pipeline,
        })
    }

    /// Returns a new instance of QueueClient.
    ///
    /// # Arguments
    ///
    /// * `queue_name` - The name of the queue.
    pub fn queue_client(&self, queue_name: &str) -> Result<QueueClient> {
        let mut queue_url = self.endpoint().clone();
        queue_url
            .path_segments_mut()
            .map_err(|_| {
                azure_core::Error::with_message(
                    azure_core::error::ErrorKind::Other,
                    "Invalid endpoint URL: Failed to parse out path segments from provided endpoint URL.",
                )
            })?
            .push(queue_name);
        Ok(QueueClient {
            endpoint: queue_url,
            pipeline: self.pipeline.clone(),
            version: self.version.clone(),
            tracer: self.tracer.clone(),
        })
    }

    /// Creates a new queue under the given account.
    ///
    /// # Arguments
    ///
    /// * `queue_name` - The name of the queue to create.
    /// * `options` - Optional configuration for the request.
    pub async fn create_queue(
        &self,
        queue_name: &str,
        options: Option<QueueClientCreateOptions<'_>>,
    ) -> Result<Response<(), NoFormat>> {
        self.queue_client(queue_name)?.create(options).await
    }

    /// Permanently deletes the specified queue.
    ///
    /// # Arguments
    ///
    /// * `queue_name` - The name of the queue to delete.
    /// * `options` - Optional configuration for the request.
    pub async fn delete_queue(
        &self,
        queue_name: &str,
        options: Option<QueueClientDeleteOptions<'_>>,
    ) -> Result<Response<(), NoFormat>> {
        self.queue_client(queue_name)?.delete(options).await
    }
}
