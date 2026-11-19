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

#[pymodule]
fn snapsim_py(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_function(wrap_pyfunction!(advise, module)?)
}
