use crate::{proto::{ValidateFileNameRequest, ValidateFileNameReply}, file};

pub fn validate_file_name(request: ValidateFileNameRequest) -> Result<ValidateFileNameReply, Box<dyn std::error::Error>> {
  let valid = match file::get_page_type(&request.name) {
    Some(_) => true,
    None => false,
  };

  Ok(ValidateFileNameReply { valid })
}