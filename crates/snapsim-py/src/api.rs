use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use std::sync::Mutex;

#[pyfunction]
#[pyo3(signature = (position_json, options_json, cancel=None))]
fn advise(
    py: Python<'_>,
    position_json: &str,
    options_json: &str,
    cancel: Option<Py<PyAny>>,
) -> PyResult<String> {
    let callback_error = Mutex::new(None);
    let stopped = || {
        cancel.as_ref().is_some_and(|callback| {
            Python::with_gil(
                |py| match callback.call0(py).and_then(|v| v.is_truthy(py)) {
                    Ok(value) => value,
                    Err(error) => {
                        *callback_error.lock().expect("cancellation lock poisoned") = Some(error);
                        true
                    }
                },
            )
        })
    };
    let result = py.allow_threads(|| {
        snapsim::advise::advise_json_with_cancel(position_json, options_json, &stopped)
    });
    if let Some(error) = callback_error
        .into_inner()
        .expect("cancellation lock poisoned")
    {
        return Err(error);
    }
    result.map_err(PyValueError::new_err)
}

#[pyfunction]
fn coverage(position_json: &str) -> PyResult<String> {
    let position =
        snapsim::position::Position::from_json(position_json).map_err(PyValueError::new_err)?;
    Ok(snapsim::coverage::coverage(&position).to_string())
}
#[pyfunction]
fn supported() -> String {
    snapsim::coverage::supported().to_string()
}
#[pyfunction]
fn deck_eligible_ids() -> String {
    serde_json::json!(snapsim::cards::deck_eligible_ids()).to_string()
}
#[pyfunction]
fn recruit_card_pool() -> String {
    serde_json::json!(snapsim::position::recruit_cards()).to_string()
}
#[pyfunction]
fn recruit_card_weights() -> String {
    serde_json::json!(snapsim::position::recruit_weights()).to_string()
}

pub fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_function(wrap_pyfunction!(advise, module)?)?;
    module.add_function(wrap_pyfunction!(coverage, module)?)?;
    module.add_function(wrap_pyfunction!(supported, module)?)?;
    module.add_function(wrap_pyfunction!(deck_eligible_ids, module)?)?;
    module.add_function(wrap_pyfunction!(recruit_card_pool, module)?)?;
    module.add_function(wrap_pyfunction!(recruit_card_weights, module)?)?;
    Ok(())
}
