use crate::k8s::get_pods;
use ::std::error::Error;
use k8s_openapi::{
    api::core::v1::Pod,
    chrono::{DateTime, Utc},
};
use kube::ResourceExt;
use std::cmp::Ordering;

pub struct Data {
    pub name: String,
    pub status: String,
    pub node: String,
    pub created_at: Option<DateTime<Utc>>,
}

pub async fn fetch_data() -> Result<Vec<Data>, Box<dyn Error>> {
    let pods = get_pods().await?;
    Ok(pods_to_data(pods))
}

fn pods_to_data(pods: Vec<Pod>) -> Vec<Data> {
    let mut items: Vec<Data> = pods.into_iter().map(pod_to_data).collect();
    items.sort_by(|a, b| match (&a.created_at, &b.created_at) {
        (Some(a), Some(b)) => b.cmp(a),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    });
    items
}

fn pod_to_data(pod: Pod) -> Data {
    let status = pod
        .status
        .as_ref()
        .and_then(|s| s.phase.clone())
        .unwrap_or_else(|| "Unknown".into());
    let node = pod
        .spec
        .as_ref()
        .and_then(|n| n.node_name.clone())
        .unwrap_or_else(|| "N/A".into());
    let created_at = pod.metadata.creation_timestamp.as_ref().map(|t| t.0);
    Data {
        name: pod.name_any(),
        status,
        node,
        created_at,
    }
}
