use k8s_openapi::api::core::v1::Pod;
use kube::{
    Client,
    api::{Api, ListParams, LogParams},
};
use std::error::Error;

const NAMESPACE: &str = "dcc";
const FILTER_KEY: &str = "managed-by";
const FILTER_VALUE: &str = "oom-scheduler";

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
    let ns = "dcc";
    let pods: Api<Pod> = Api::namespaced(client, ns);
    let lp = LogParams {
        follow: true,
        tail_lines: Some(100),
        ..LogParams::default()
    };
    pods.log_stream(pod, &lp).await
}

pub fn get_checkout_status() {}
