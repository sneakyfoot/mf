use k8s_openapi::api::core::v1::Node;
use k8s_openapi::api::core::v1::Pod;
use kube::{
    Client,
    api::{Api, ListParams, LogParams},
};
use std::error::Error;

const NAMESPACE: &str = "dcc";
const FILTER_KEY: &str = "managed-by";
const FILTER_VALUE: &str = "oom-scheduler";
const CHECK_OUT_KEY: &str = "oom_farm";

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
