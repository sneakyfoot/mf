use crate::k8s::get_pods;
use ::std::error::Error;
use k8s_openapi::api::core::v1::Pod;
use kube::ResourceExt;

pub struct Data {
    pub name: String,
    pub status: String,
    pub age: String,
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
    let age = pod
        .metadata
        .creation_timestamp
        .as_ref()
        .map(|t| format!("{}", t.0))
        .unwrap_or_else(|| "n/a".into());
    Data {
        name: pod.name_any(),
        status,
        age,
    }
}

pub fn sample_data() -> Vec<Data> {
    vec![
        Data {
            name: "render-beauty-fkas45".into(),
            status: "Running".into(),
            age: "24m".into(),
        },
        Data {
            name: "sim-pyro-3daf4".into(),
            status: "Pending".into(),
            age: "95m".into(),
        },
    ]
}
