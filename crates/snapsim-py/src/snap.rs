use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use snapsim::snap::SnapError;

pyo3::create_exception!(snapsim_py, NotPlanningError, PyValueError);

#[pyfunction]
fn position_from_snap(observation_json: &str, options_json: &str) -> PyResult<String> {
    let observation =
        serde_json::from_str(observation_json).map_err(|e| PyValueError::new_err(e.to_string()))?;
    let options: serde_json::Value =
        serde_json::from_str(options_json).map_err(|e| PyValueError::new_err(e.to_string()))?;
    if !options.is_object() {
        return Err(PyValueError::new_err("options must be a JSON object"));
    }
    for name in ["deck", "deck_added", "revealed_turns"] {
        if !options[name].is_null() && !options[name].is_array() {
            return Err(PyValueError::new_err(format!(
                "{name} must be an array or null"
            )));
        }
    }
    let position =
        snapsim::snap::position_from_snap(&observation, &options).map_err(|e| match e {
            SnapError::Invalid(message) | SnapError::InvalidType(message) => {
                PyValueError::new_err(message)
            }
            SnapError::NotPlanning(message) => NotPlanningError::new_err(message),
        })?;
    serde_json::to_string(&position).map_err(|e| PyValueError::new_err(e.to_string()))
}

pub fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add(
        "NotPlanningError",
        module.py().get_type::<NotPlanningError>(),
    )?;
    module.add_function(wrap_pyfunction!(position_from_snap, module)?)
}
