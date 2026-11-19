use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use std::sync::Mutex;

#[pyfunction]
#[pyo3(signature = (args_json, progress=None, cancel=None))]
fn deckbuild(
    py: Python<'_>,
    args_json: &str,
    progress: Option<Py<PyAny>>,
    cancel: Option<Py<PyAny>>,
) -> PyResult<String> {
    let args: serde_json::Value =
        serde_json::from_str(args_json).map_err(|e| PyValueError::new_err(e.to_string()))?;
    let error = Mutex::new(None);
    let stopped = || {
        if error.lock().unwrap().is_some() {
            return true;
        }
        cancel.as_ref().is_some_and(|callback| {
            Python::with_gil(
                |py| match callback.call0(py).and_then(|v| v.is_truthy(py)) {
                    Ok(v) => v,
                    Err(e) => {
                        *error.lock().unwrap() = Some(e);
                        true
                    }
                },
            )
        })
    };
    let mut report = |value: serde_json::Value| {
        if let Some(callback) = &progress {
            Python::with_gil(|py| {
                let result = py
                    .import("json")
                    .and_then(|json| json.call_method1("loads", (value.to_string(),)))
                    .and_then(|v| callback.call1(py, (v,)));
                if let Err(e) = result {
                    *error.lock().unwrap() = Some(e);
                }
            });
        }
    };
    let result = py.allow_threads(|| snapsim::deckbuild::deckbuild(&args, &mut report, &stopped));
    if let Some(error) = error.into_inner().unwrap() {
        return Err(error);
    }
    result.map(|v| v.to_string()).map_err(PyValueError::new_err)
}
pub fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_function(wrap_pyfunction!(deckbuild, module)?)
}
