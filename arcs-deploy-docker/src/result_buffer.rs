use smallvec::{smallvec, SmallVec};
use bollard::models::{ ImageId, BuildInfo, ProgressDetail, ErrorDetail };
use std::{fmt::Display, io::stdout};
use std::io::Stdout;

use const_format::{concatcp, str_repeat};

#[derive(Debug)]
pub struct ResultBuffer<T: std::io::Write = Stdout> {
    stream_output: Option<String>,
    id_list: SmallVec<[String; 1]>,
    progress_string: Option<String>,
    progress_portion: Option<f64>,
    progress_target: Option<T>,

    error_list: SmallVec<[ResultBufferError; 1]>,
}

#[derive(Debug)]
pub struct ResultBufferError {
    pub error_text: String,
    pub code: Option<i64>,
    pub detail_message: Option<String>,
}

const BAR_WIDTH: usize = 20;
const PROGRESS_FILLED_STR: &str = str_repeat!("#", BAR_WIDTH);
const PROGRESS_UNFILL_STR: &str = str_repeat!("-", BAR_WIDTH);


impl<T: std::io::Write> ResultBuffer<T> {
    pub fn new() -> Self {
        Self {
            stream_output: None,
            id_list: smallvec![],
            progress_string: None,
            progress_portion: None,
            progress_target: None,
            error_list: smallvec![],
        }
    }

    pub fn with_progress_logging(self, target: T) -> Self {
        Self {
            progress_target: Some(target),
            ..self
        }
    }

    pub fn process_build_info(&mut self, build_info: BuildInfo) -> std::io::Result<()> {
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
            self.update_progress_portion(progress_detail)?;
        }

        Ok(())
    }

    pub fn stream_in(&mut self, new_data: &str) -> &mut Self {
        match self.stream_output.take() {
            Some(mut curr_stream) => {
                curr_stream.push_str(new_data);
                self.stream_output = Some(curr_stream);
            },
            None => {
                self.stream_output = Some(new_data.to_string());
            }
        } 
        print!("{}", new_data);
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

    // TODO: Make this actually write to the correct target.
    pub fn update_progress_portion(&mut self, detail: ProgressDetail) -> std::io::Result<()> {
        if let (Some(current), Some(total)) = (detail.current, detail.total) {
            let portion = current as f64 / total as f64;
            self.progress_portion = Some(portion);
            if let Some(ref mut target) = self.progress_target {
                let progress_idx = (portion * BAR_WIDTH as f64) as usize;
                use std::io::Write;
                write!(stdout(), "|{}{}|\r", &PROGRESS_FILLED_STR[0..progress_idx], &PROGRESS_UNFILL_STR[0..(20 - progress_idx)])?;
            }
        }
        Ok(()) 
    }
}

impl<T: std::io::Write> Display for ResultBuffer<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use std::borrow::Cow::*;
        if self.error_list.is_empty() {
            let id_text = if self.id_list.is_empty() {
                Borrowed("")
            } else {
                let mut output = String::from("");
                output.push_str(&self.id_list[0]);
                self.id_list.iter().skip(1).for_each(|string| {
                    output.push_str(", ");
                    output.push_str(string);
                });
                output.push(')');
                Owned(output)
            };

            writeln!(f, "Build completed successfully. {}", id_text)?;

            if let Some(stream) = &self.stream_output {

                writeln!(f, "\nStream output:")?;
                for line in stream.lines() {
                    writeln!(f, "\t{}", line)?;
                }
            }

        } else {

        }
        Ok(())
    }
}
