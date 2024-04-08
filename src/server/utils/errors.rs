use yaml::File;

/// Enum that represents the different errors that can occur during the deploy process
/// 
/// ## Variants
/// - `FileUpload` - Error uploading file(s) to CDN
/// - `Build` - Error building Docker image
/// - `Push` - Error pushing to remote Docker registry
/// - `Pull` - Error pulling from remote Docker registry
/// - `Fetch` - Error fetching local challenge folder
/// - `Deploy` - Error deploying to Kubernetes cluster
#[derive(Debug, Clone)]
pub enum DeployProcessErr {
    FileUpload(Vec<File>),
    Build(String),
    Push(String),
    Pull(String),
    Fetch(String),
    Deploy(String),
}
impl std::fmt::Display for DeployProcessErr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FileUpload(files) => write!(f, "Failed to upload: {files:?}"),
            Self::Build(e) => write!(f, "Failed to build: {e}"),
            Self::Push(e) => write!(f, "Failed to push: {e}"),
            Self::Pull(e) => write!(f, "Failed to pull: {e}"),
            Self::Fetch(e) => write!(f, "Failed to fetch: {e}"),
            Self::Deploy(e) => write!(f, "Failed to deploy: {e}"),
        }
    }
}

// impl From<(DeployProcessErr, Metadata)> for Response {
//     fn from((err, meta): (DeployProcessErr, Metadata)) -> Self {
//         use DeployProcessErr::*;
//         match err {
//             FileUpload(files) => Response::server_deploy_process_err(
//                 0,
//                 "Error uploading file(s) to CDN",
//                 Some(json!({ "files": files })),
//                 meta,
//             ),
//             Build(s) => Response::server_deploy_process_err(
//                 1,
//                 "Error building docker image",
//                 Some(json!({ "reason": s })),
//                 meta,
//             ),
//             Push(s) => Response::server_deploy_process_err(
//                 2,
//                 "Error pushing to registry",
//                 Some(json!({ "reason": s })),
//                 meta,
//             ),
//             Pull(s) => Response::server_deploy_process_err(
//                 3,
//                 "Error pulling from registry",
//                 Some(json!({ "reason": s })),
//                 meta,
//             ),
//             Fetch(s) => Response::server_deploy_process_err(
//                 4,
//                 "Error fetching challenge folder",
//                 Some(json!({ "reason": s })),
//                 meta,
//             ),
//             Deploy(s) => Response::server_deploy_process_err(
//                 5,
//                 "Error deploying to Kubernetes",
//                 Some(json!({ "reason": s })),
//                 meta,
//             ),
//         }
//     }
// }
