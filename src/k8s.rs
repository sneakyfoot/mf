use k8s_openapi::api::core::v1::Pod;
use kube::{
    Client,
    api::{Api, ListParams, LogParams},
};
use std::error::Error;

pub async fn get_pods() -> Result<Vec<Pod>, Box<dyn Error>> {
    let client = Client::try_default().await?;
    let ns = "dcc";
    let pods: Api<Pod> = Api::namespaced(client, ns);
    let list = pods.list(&ListParams::default()).await?;
    Ok(list.items)
}

pub async fn stream_logs(pod: &str) -> Result<impl futures::AsyncBufRead + Unpin, kube::Error> {
    let client = Client::try_default().await?;
    let ns = "dcc";
    let pods: Api<Pod> = Api::namespaced(client, ns);
    let lp = LogParams {
        follow: true,
        tail_lines: Some(200),
        ..LogParams::default()
    };
    pods.log_stream(pod, &lp).await
}
