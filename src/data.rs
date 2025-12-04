pub struct Data {
    pub name: String,
    pub status: String,
    pub age: String,
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
