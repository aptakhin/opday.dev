use serde_derive::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize)]
pub struct OpdayError {
    pub error: String,
    pub error_loc: String,
}

#[derive(Serialize, Deserialize)]
pub struct IdResponse {
    pub success: bool,
    pub id: Option<Uuid>,
    pub error: Option<OpdayError>,
}

#[derive(Serialize, Deserialize)]
pub struct HealthCheckModel {
    pub name: String,
}

#[derive(Serialize, Deserialize)]
pub struct HealthCheckModelGetResponse {
    pub success: bool,

    pub model: Option<HealthCheckModel>,
    pub error: Option<OpdayError>,
}

#[derive(Serialize, Deserialize)]
pub struct InsertHealthCheckModelRequest {
    pub name: String,
}

// #[derive(Serialize, Deserialize)]
pub struct InsertedHealthCheckModel {
    pub id: Uuid,
    pub model: HealthCheckModel,
}
