use futures::{StreamExt, TryStreamExt};
use k8s_openapi::api::core::v1::Pod;
use kube::{
    Client,
    api::{Api, ListParams, PostParams, ResourceExt},
};
use std::error::Error;

pub async fn get_pods() -> Result<Vec<Pod>, Box<dyn Error>> {
    let client = Client::try_default().await?;
    let ns = "dcc";
    let pods: Api<Pod> = Api::namespaced(client, ns);
    let list = pods.list(&ListParams::default()).await?;
    Ok(list.items)
}
