use k8s_openapi::api::batch::v1::Job;
use k8s_openapi::api::core::v1::Node;
use k8s_openapi::api::core::v1::Pod;
use kube::{
    Client, ResourceExt,
    api::{Api, DeleteParams, ListParams, LogParams},
};
use std::error::Error;

const NAMESPACE: &str = "dcc";
const FILTER_KEY: &str = "managed-by";
const FILTER_VALUE: &str = "oom-scheduler";
const CHECK_OUT_KEY: &str = "oom/farm";

pub async fn get_pods(client: Client) -> Result<Vec<Pod>, Box<dyn Error>> {
    let ns = NAMESPACE;
    let pods: Api<Pod> = Api::namespaced(client, ns);
    let list = pods.list(&ListParams::default()).await?;
    let filtered: Vec<Pod> = list
        .into_iter()
        .filter(|list| {
            list.metadata
                .labels
                .as_ref()
                .and_then(|labels| labels.get(FILTER_KEY))
                .map(|val| val == FILTER_VALUE)
                .unwrap_or(false)
        })
        .collect();
    Ok(filtered)
}

pub async fn stream_logs(
    client: Client,
    pod: &str,
) -> Result<impl futures::AsyncBufRead + Unpin, kube::Error> {
    let ns = NAMESPACE;
    let pods: Api<Pod> = Api::namespaced(client, ns);
    let lp = LogParams {
        follow: true,
        tail_lines: Some(100),
        ..LogParams::default()
    };
    pods.log_stream(pod, &lp).await
}

/// Check if the node is schedulable based on the label (key).
/// If the label value is "true", the node is considered schedulable.
/// If the label is missing or has any other value, the node is not schedulable
pub async fn is_host_schedulable(
    client: Client,
    key: Option<&str>,
) -> Result<bool, Box<dyn Error>> {
    let key = key.unwrap_or(CHECK_OUT_KEY);
    let node_name = hostname::get()?.to_string_lossy().into_owned();
    let nodes: Api<Node> = Api::all(client);
    let node = nodes.get(&node_name).await?;
    let labels = node.metadata.labels.unwrap_or_default();
    let value = labels.get(key);
    Ok(match value {
        Some(v) if v == "true" => true,
        _ => false,
    })
}

pub async fn set_host_schedulable(
    client: Client,
    key: Option<&str>,
    schedulable: bool,
) -> Result<(), Box<dyn Error>> {
    let key = key.unwrap_or(CHECK_OUT_KEY);
    let node_name = hostname::get()?.to_string_lossy().into_owned();
    let nodes: Api<Node> = Api::all(client);
    let mut node = nodes.get(&node_name).await?;
    let labels = node
        .metadata
        .labels
        .get_or_insert_with(|| std::collections::BTreeMap::new());
    labels.insert(key.to_string(), schedulable.to_string());
    nodes
        .replace(&node_name, &Default::default(), &node)
        .await?;
    Ok(())
}

/// Cancel all jobs associated with the given controller id (final element provided by pdg).
pub async fn cancel_jobs(client: Client, controller: &str) -> Result<(), Box<dyn Error>> {
    let ns = NAMESPACE;
    let controller_suffix = controller.rsplit('-').next();
    let jobs: Api<Job> = Api::namespaced(client, ns);
    let list = jobs.list(&ListParams::default()).await?;
    for job in list.into_iter().filter(|j| {
        j.metadata
            .name
            .as_deref()
            .and_then(|n| n.rsplit('-').next())
            == controller_suffix
    }) {
        jobs.delete(&job.name_any(), &DeleteParams::foreground())
            .await?;
    }
    Ok(())
}
