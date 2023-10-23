use crate::polling::{PollingId, advance_deployment_step};
use crate::server::responses::Metadata;
use crate::emitter::send_deployment_failure;
use crate::logging::*;

pub async fn send_failure_message(meta: &Metadata, message: &str) {
    match send_deployment_failure(meta, format!("Failed to deploy {}: {} Error", meta.chall_name(), message)).await {
        Ok(_) => info!("Successfully sent deployment failure message for {} ({})", meta.chall_name(), meta.poll_id()),
        Err(e) => error!("Failed to send deployment failure message for {} ({}): {e:?}", meta.chall_name(), meta.poll_id()),
    };
}

/// Convenience function that calls `advance_deployment_step` on an ongoing deployment and logs the result.
pub fn advance_with_fail_log(polling_id: PollingId) -> bool {
    match advance_deployment_step(polling_id, None) {
        Ok(new_step) => {
            info!("Deployment step advanced to `{}` for {polling_id}", new_step.get_str());
            true
        }
        Err(e) => {
            error!("Failed to advance deployment step for {polling_id} (KILLED): {e:?}");
            false
        }
    }
}
