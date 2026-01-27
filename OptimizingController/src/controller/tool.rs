use reqwest::Client;
use serde::{Serialize, Deserialize};
use serde_json::Value;
use crate::controller::OptimizingController;

#[derive(Serialize, Deserialize)]
pub struct CalculationRequest {
    pub mdiff: f64,
    pub measured: f64,
    pub workload: f64,
}

pub async fn call_tool(
    controller: &OptimizingController,
    measured_constraint: f64,
) -> Result<f64, String> {
    let measurement_difference =
        (controller.sched_xup * (1.0 / controller.kf.x_hat)) - measured_constraint;
    let x_hat = controller.kf.x_hat;
    let url = "http://127.0.0.1:8080/calculate";

    let request_body = CalculationRequest {
        mdiff: measurement_difference,
        measured: measured_constraint,
        workload: x_hat,
    };

    let client = Client::new();

    let response = client
        .post(url)
        .json(&request_body)
        .send()
        .await
        .map_err(|e| format!("Request error: {}", e))?;

    let json_response: Value = response
        .json()
        .await
        .map_err(|e| format!("JSON parsing error: {}", e))?;

    json_response
        .get("result")
        .and_then(|v| v.as_f64())
        .ok_or_else(|| "Missing or invalid 'result' field in response".to_string())
}
