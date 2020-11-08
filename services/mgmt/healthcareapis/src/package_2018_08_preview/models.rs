#![doc = "generated by AutoRust 0.1.0"]
#![allow(non_camel_case_types)]
#![allow(unused_imports)]
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ServicesProperties {
    #[serde(rename = "provisioningState", skip_serializing)]
    pub provisioning_state: Option<services_properties::ProvisioningState>,
    #[serde(rename = "accessPolicies", skip_serializing_if = "Option::is_none")]
    pub access_policies: Option<ServiceAccessPoliciesInfo>,
    #[serde(rename = "cosmosDbConfiguration", skip_serializing_if = "Option::is_none")]
    pub cosmos_db_configuration: Option<ServiceCosmosDbConfigurationInfo>,
    #[serde(rename = "authenticationConfiguration", skip_serializing_if = "Option::is_none")]
    pub authentication_configuration: Option<ServiceAuthenticationConfigurationInfo>,
    #[serde(rename = "corsConfiguration", skip_serializing_if = "Option::is_none")]
    pub cors_configuration: Option<ServiceCorsConfigurationInfo>,
}
pub mod services_properties {
    use super::*;
    #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
    pub enum ProvisioningState {
        Deleting,
        Succeeded,
        Creating,
        Accepted,
        Verifying,
        Updating,
        Failed,
        Canceled,
        Deprovisioned,
    }
}
pub type ServiceAccessPoliciesInfo = Vec<ServiceAccessPolicyEntry>;
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ServiceAccessPolicyEntry {
    #[serde(rename = "objectId")]
    pub object_id: String,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ServiceCosmosDbConfigurationInfo {
    #[serde(rename = "offerThroughput", skip_serializing_if = "Option::is_none")]
    pub offer_throughput: Option<i64>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ServiceAuthenticationConfigurationInfo {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authority: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audience: Option<String>,
    #[serde(rename = "smartProxyEnabled", skip_serializing_if = "Option::is_none")]
    pub smart_proxy_enabled: Option<bool>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ServiceCorsConfigurationInfo {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub origins: Vec<ServiceCorsConfigurationOriginEntry>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub headers: Vec<ServiceCorsConfigurationHeaderEntry>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub methods: Vec<ServiceCorsConfigurationMethodEntry>,
    #[serde(rename = "maxAge", skip_serializing_if = "Option::is_none")]
    pub max_age: Option<i64>,
    #[serde(rename = "allowCredentials", skip_serializing_if = "Option::is_none")]
    pub allow_credentials: Option<bool>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ServiceCorsConfigurationOriginEntry {}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ServiceCorsConfigurationHeaderEntry {}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ServiceCorsConfigurationMethodEntry {}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ServicesDescription {
    #[serde(flatten)]
    pub resource: Resource,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub properties: Option<ServicesProperties>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ServicesPatchDescription {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<serde_json::Value>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Resource {
    #[serde(skip_serializing)]
    pub id: Option<String>,
    #[serde(skip_serializing)]
    pub name: Option<String>,
    #[serde(rename = "type", skip_serializing)]
    pub type_: Option<String>,
    pub kind: resource::Kind,
    pub location: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub etag: Option<String>,
}
pub mod resource {
    use super::*;
    #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
    pub enum Kind {
        #[serde(rename = "fhir")]
        Fhir,
        #[serde(rename = "fhir-Stu3")]
        FhirStu3,
        #[serde(rename = "fhir-R4")]
        FhirR4,
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ErrorDetails {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ErrorDetailsInternal>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ErrorDetailsInternal {
    #[serde(skip_serializing)]
    pub code: Option<String>,
    #[serde(skip_serializing)]
    pub message: Option<String>,
    #[serde(skip_serializing)]
    pub target: Option<String>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ServicesDescriptionListResult {
    #[serde(rename = "nextLink", skip_serializing_if = "Option::is_none")]
    pub next_link: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub value: Vec<ServicesDescription>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OperationListResult {
    #[serde(rename = "nextLink", skip_serializing_if = "Option::is_none")]
    pub next_link: Option<String>,
    #[serde(skip_serializing)]
    pub value: Vec<Operation>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Operation {
    #[serde(skip_serializing)]
    pub name: Option<String>,
    #[serde(skip_serializing)]
    pub origin: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display: Option<OperationDisplay>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OperationDisplay {
    #[serde(skip_serializing)]
    pub provider: Option<String>,
    #[serde(skip_serializing)]
    pub resource: Option<String>,
    #[serde(skip_serializing)]
    pub operation: Option<String>,
    #[serde(skip_serializing)]
    pub description: Option<String>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CheckNameAvailabilityParameters {
    pub name: String,
    #[serde(rename = "type")]
    pub type_: String,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ServicesNameAvailabilityInfo {
    #[serde(rename = "nameAvailable", skip_serializing)]
    pub name_available: Option<bool>,
    #[serde(skip_serializing)]
    pub reason: Option<services_name_availability_info::Reason>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}
pub mod services_name_availability_info {
    use super::*;
    #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
    pub enum Reason {
        Invalid,
        AlreadyExists,
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OperationResultsDescription {
    #[serde(skip_serializing)]
    pub id: Option<String>,
    #[serde(skip_serializing)]
    pub name: Option<String>,
    #[serde(skip_serializing)]
    pub status: Option<operation_results_description::Status>,
    #[serde(rename = "startTime", skip_serializing)]
    pub start_time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub properties: Option<serde_json::Value>,
}
pub mod operation_results_description {
    use super::*;
    #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
    pub enum Status {
        Canceled,
        Succeeded,
        Failed,
        Requested,
        Running,
    }
}