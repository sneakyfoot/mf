use crate::k8s::get_pods;
use ::std::error::Error;
use k8s_openapi::{
    api::core::v1::Pod,
    chrono::{DateTime, Utc},
};
use kube::{Client, ResourceExt};
use std::cmp::Ordering;

pub struct Data {
    pub name: String,
    pub status: String,
    pub artist: String,
    pub node: String,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
    pub created_at: Option<DateTime<Utc>>,
}

/// Fetch data from Kubernetes pods and convert them into a sorted vector of Data structs.
pub async fn fetch_data(client: Client) -> Result<Vec<Data>, Box<dyn Error>> {
    let pods = get_pods(client).await?;
    Ok(pods_to_data(pods))
}

/// Convert a vector of Pod objects into a sorted vector of Data structs.
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

/// Convert a single Pod object into a Data struct.
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
    let artist = pod
        .metadata
        .labels
        .as_ref()
        .and_then(|labels| labels.get("oom/artist"))
        .cloned()
        .unwrap_or_else(|| "Unknown".into());
    let started_at = pod
        .status
        .as_ref()
        .and_then(|s| s.start_time.as_ref())
        .map(|s| s.0);
    let finished_at = pod_finished_at(&pod);
    let created_at = pod.metadata.creation_timestamp.as_ref().map(|t| t.0);
    Data {
        name: pod.name_any(),
        status,
        node,
        artist,
        started_at,
        finished_at,
        created_at,
    }
}

/// Get the latest finished_at time from the terminated container statuses of a Pod.
fn pod_finished_at(pod: &Pod) -> Option<DateTime<Utc>> {
    pod.status
        .as_ref()?
        .container_statuses
        .as_ref()?
        .iter()
        .filter_map(|cs| {
            let state = cs.state.as_ref()?;
            let term = state.terminated.clone()?;
            term.finished_at
        })
        .max()
        .map(|s| s.0)
}
