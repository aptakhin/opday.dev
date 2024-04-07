use serde_derive::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct HealthCheckModel {
    pub name: String,
}
