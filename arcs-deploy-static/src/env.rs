use arcs_env_rs::*;

env_var_req!(S3_BEARER_TOKEN);
env_var_req!(S3_ACCESS_KEY);
env_var_req!(S3_BUCKET_URL);
env_var_req!(S3_REGION);
env_var_req!(S3_BUCKET_NAME);

env_var_req!(CHALL_FOLDER -> CHALL_FOLDER_DEFAULT);

assert_req_env!(check_env_vars: S3_BEARER_TOKEN, S3_ACCESS_KEY, S3_BUCKET_URL, S3_REGION, S3_BUCKET_NAME, CHALL_FOLDER_DEFAULT);