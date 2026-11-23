use pyo3::prelude::*;

mod api;
mod env;
mod snap;
mod tools;

#[pymodule]
fn snapsim_py(module: &Bound<'_, PyModule>) -> PyResult<()> {
    api::register(module)?;
    snap::register(module)?;
    tools::register(module)?;
    env::register(module)
}
