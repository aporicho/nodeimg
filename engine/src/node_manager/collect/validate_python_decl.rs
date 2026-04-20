#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PythonPinValidationSpec {
    pub name: String,
    pub required: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PythonParamValidationSpec {
    pub name: String,
    pub data_type: String,
    pub default_expr: String,
    pub expose_values: Vec<String>,
}

pub fn validate_python_node_decl(
    inputs: &[PythonPinValidationSpec],
    outputs: &[PythonPinValidationSpec],
    params: &[PythonParamValidationSpec],
) -> Result<(), String> {
    validate_unique_names(inputs, outputs, params)?;
    validate_param_rules(params)?;
    validate_output_rules(outputs)?;
    Ok(())
}

fn validate_output_rules(outputs: &[PythonPinValidationSpec]) -> Result<(), String> {
    for output in outputs {
        if output.required {
            return Err(format!(
                "output Pin '{}' must not declare required",
                output.name
            ));
        }
    }
    Ok(())
}

fn validate_unique_names(
    inputs: &[PythonPinValidationSpec],
    outputs: &[PythonPinValidationSpec],
    params: &[PythonParamValidationSpec],
) -> Result<(), String> {
    use std::collections::HashSet;

    let mut input_names = HashSet::new();
    let mut output_names = HashSet::new();

    for name in inputs.iter().map(|p| &p.name) {
        if !input_names.insert(name.clone()) {
            return Err(format!("duplicate input interface name '{}'", name));
        }
    }
    for name in outputs.iter().map(|p| &p.name) {
        if !output_names.insert(name.clone()) {
            return Err(format!("duplicate output interface name '{}'", name));
        }
    }

    for param in params {
        if param.expose_values.iter().any(|value| value == "input")
            && !input_names.insert(param.name.clone())
        {
            return Err(format!(
                "duplicate input-side interface name '{}'",
                param.name
            ));
        }

        if param.expose_values.iter().any(|value| value == "output")
            && !output_names.insert(param.name.clone())
        {
            return Err(format!(
                "duplicate output-side interface name '{}'",
                param.name
            ));
        }
    }

    Ok(())
}

fn validate_param_rules(params: &[PythonParamValidationSpec]) -> Result<(), String> {
    for param in params {
        if param.expose_values.is_empty() {
            return Err(format!(
                "Param '{}' must declare non-empty expose=[...]",
                param.name
            ));
        }

        for value in &param.expose_values {
            match value.as_str() {
                "control" | "input" | "output" => {}
                other => {
                    return Err(format!(
                        "Param '{}' has unsupported expose value '{}'",
                        param.name, other
                    ));
                }
            }
        }

        if is_complex_python_type(&param.data_type) && param.default_expr.trim() != "None" {
            return Err(format!(
                "Param '{}' with complex type '{}' must use default=None",
                param.name, param.data_type
            ));
        }
    }

    Ok(())
}

fn is_complex_python_type(data_type: &str) -> bool {
    matches!(
        data_type,
        "IMAGE" | "MASK" | "MODEL" | "CLIP" | "VAE" | "LATENT" | "CONDITIONING"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(name: &str) -> PythonPinValidationSpec {
        PythonPinValidationSpec {
            name: name.into(),
            required: true,
        }
    }

    fn output(name: &str) -> PythonPinValidationSpec {
        PythonPinValidationSpec {
            name: name.into(),
            required: false,
        }
    }

    fn param(
        name: &str,
        data_type: &str,
        default_expr: &str,
        expose_values: &[&str],
    ) -> PythonParamValidationSpec {
        PythonParamValidationSpec {
            name: name.into(),
            data_type: data_type.into(),
            default_expr: default_expr.into(),
            expose_values: expose_values.iter().map(|v| (*v).to_string()).collect(),
        }
    }

    #[test]
    fn reject_empty_expose() {
        let result = validate_python_node_decl(&[], &[], &[param("x", "FLOAT", "0.0", &[])]);
        assert!(result.is_err());
    }

    #[test]
    fn reject_invalid_expose_value() {
        let result = validate_python_node_decl(&[], &[], &[param("x", "FLOAT", "0.0", &["bad"])]);
        assert!(result.is_err());
    }

    #[test]
    fn reject_complex_default_value() {
        let result =
            validate_python_node_decl(&[], &[], &[param("img", "IMAGE", "\"x\"", &["control"])]);
        assert!(result.is_err());
    }

    #[test]
    fn reject_duplicate_input_side_name() {
        let result = validate_python_node_decl(
            &[input("strength")],
            &[],
            &[param("strength", "FLOAT", "0.0", &["input"])],
        );
        assert!(result.is_err());
    }

    #[test]
    fn allow_same_name_between_input_and_output() {
        let result = validate_python_node_decl(&[input("latent")], &[output("latent")], &[]);
        assert!(result.is_ok());
    }
}
