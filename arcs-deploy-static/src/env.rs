use arcs_deploy_env::*;

env_var_req!(S3_BEARER_TOKEN);
env_var_req!(S3_BUCKET_URL);
env_var_req!(CHALL_FOLDER -> CHALL_FOLDER_DEFAULT);

assert_req_env!(check_env_vars: S3_BEARER_TOKEN, S3_BUCKET_URL, CHALL_FOLDER_DEFAULT);