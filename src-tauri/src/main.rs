//! veilanon desktop entry point
//! Delegates entirely to the library crate.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    veilanon_lib::run();
}
