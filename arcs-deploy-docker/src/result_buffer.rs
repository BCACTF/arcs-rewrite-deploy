use smallvec::{smallvec, SmallVec};
use bollard::models::{ ImageId, BuildInfo, ProgressDetail, ErrorDetail };

pub struct ResultBuffer {
    stream_output: Option<String>,
    id_list: SmallVec<[String; 1]>,
    progress_string: Option<String>,
    progress_portion: Option<f64>,
    error_list: SmallVec<[ResultBufferError; 1]>,
}

pub struct ResultBufferError {
    pub error_text: String,
    pub code: Option<i64>,
    pub detail_message: Option<String>,
}

impl ResultBuffer {
    pub fn new() -> Self {
        Self {
            stream_output: None,
            id_list: smallvec![],
            progress_string: None,
            progress_portion: None,
            error_list: smallvec![],
        }
    }

    pub fn process_build_info(&mut self, build_info: BuildInfo) -> &mut Self {
        if let Some(id) = build_info.id {
            todo!("implement id handling: {:?}", id);
        }

        if let Some(new_data) = build_info.stream {
            self.stream_in(&new_data);
        }

        if let Some(error) = build_info.error {
            self.add_error(error, build_info.error_detail);
        }

        if let Some(status) = build_info.status {
            todo!("implement status handling: {:?}", status);
        }

        if let Some(curr_progress) = build_info.progress {
            self.update_progress_string(curr_progress);
        }

        if let Some(progress_detail) = build_info.progress_detail {
            self.update_progress_portion(progress_detail);
        }

        self
    }

    pub fn stream_in(&mut self, new_data: &str) -> &mut Self {
        match self.stream_output.take() {
            Some(mut curr_stream) => {
                curr_stream.push_str(new_data);
            },
            None => {
                self.stream_output = Some(new_data.to_string());
            }
        } 
        self
    }

    pub fn image_id(&mut self, image_id_struct: ImageId) -> &mut Self {
        if let Some(image_id) = image_id_struct.id {
            self.id_list.push(image_id);
        }
        self
    }

    pub fn add_error(&mut self, error: String, opt_detail: Option<ErrorDetail>) -> &mut Self {
        match opt_detail {
            Some(detail) => self.error_list.push(ResultBufferError {
                error_text: error,
                code: detail.code,
                detail_message: detail.message,
            }),
            None => self.error_list.push(ResultBufferError {
                error_text: error,
                code: None,
                detail_message: None,
            })
        }
        self
    }

    pub fn update_progress_string(&mut self, progress: String) -> &mut Self {
        self.progress_string = Some(progress);
        self
    }

    pub fn update_progress_portion(&mut self, detail: ProgressDetail) -> &mut Self {
        if let (Some(current), Some(total)) = (detail.current, detail.total) {
            self.progress_portion = Some(current as f64 / total as f64);
        }
        self
    }
}
