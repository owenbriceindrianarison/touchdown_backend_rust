use std::collections::BTreeMap;

use shared::AppError;
use validator::{Validate, ValidationErrors};

/// Converts `validator` errors to `AppError::Validation`,
/// whose serialization yields the `details` exposed to the client.
pub fn to_app_error(errors: ValidationErrors) -> AppError {
    let mut fields: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (field, errs) in errors.field_errors() {
        fields.insert(
            field.to_string(),
            errs.iter()
                .map(|e| {
                    e.message
                        .as_ref()
                        .map(|m| m.to_string())
                        .unwrap_or_else(|| e.code.to_string())
                })
                .collect(),
        );
    }
    AppError::Validation { fields }
}

// Call this at the beginning of each handler: `validate(&req)?;`
pub fn validate<T: Validate>(value: &T) -> Result<(), AppError> {
    value.validate().map_err(to_app_error)
}
