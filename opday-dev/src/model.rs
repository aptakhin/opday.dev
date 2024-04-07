use serde_derive::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug)]
pub struct OpdayError {
    pub error: String,
    pub error_loc: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct IdResponse {
    pub success: bool,
    pub id: Option<Uuid>,
    pub error: Option<OpdayError>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct HealthCheckModel {
    pub name: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct HealthCheckModelGetResponse {
    pub success: bool,

    pub model: Option<HealthCheckModel>,
    pub error: Option<OpdayError>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct InsertHealthCheckModelRequest {
    pub name: String,
    pub url: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct InsertedHealthCheckModel {
    pub id: Uuid,
    pub model: HealthCheckModel,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct HealthCheckModelUpdateRequest {
    pub name: String,
    pub url: String,
}
