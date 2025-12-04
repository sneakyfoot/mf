use crate::k8s::get_pods;
use ::std::error::Error;
use k8s_openapi::{
    api::core::v1::Pod,
    chrono::{DateTime, Utc},
};
use kube::ResourceExt;

pub struct Data {
    pub name: String,
    pub status: String,
    pub created_at: Option<DateTime<Utc>>,
}

pub async fn fetch_data() -> Result<Vec<Data>, Box<dyn Error>> {
    let pods = get_pods().await?;
    Ok(pods_to_data(pods))
}

pub fn pods_to_data(pods: Vec<Pod>) -> Vec<Data> {
    pods.into_iter().map(pod_to_data).collect()
}

fn pod_to_data(pod: Pod) -> Data {
    let status = pod
        .status
        .as_ref()
        .and_then(|s| s.phase.clone())
        .unwrap_or_else(|| "Unknown".into());
    let created_at = pod.metadata.creation_timestamp.as_ref().map(|t| t.0);
    Data {
        name: pod.name_any(),
        status,
        created_at,
    }
}
