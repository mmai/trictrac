//! # Expose trictrac game state and rules in a python module
use pyo3::prelude::*;
use pyo3::types::PyTuple;

#[pyclass]
struct TricTrac {
    state: String, // Remplace par ta structure d'état du jeu
}

#[pymethods]
impl TricTrac {
    #[new]
    fn new() -> Self {
        TricTrac {
            state: "Initial state".to_string(),
        }
    }

    fn get_state(&self) -> String {
        self.state.clone()
    }

    fn get_available_moves(&self) -> Vec<(i32, i32)> {
        vec![(0, 5), (3, 8)] // Remplace par ta logique de génération de coups
    }

    fn play_move(&mut self, from_pos: i32, to_pos: i32) -> bool {
        // Ajoute la logique du jeu ici
        println!("Move... from {} to {}", from_pos, to_pos);
        true
    }
}

/// A Python module implemented in Rust. The name of this function must match
/// the `lib.name` setting in the `Cargo.toml`, else Python will not be able to
/// import the module.
#[pymodule]
fn trictrac(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<TricTrac>()?;

    Ok(())
}
