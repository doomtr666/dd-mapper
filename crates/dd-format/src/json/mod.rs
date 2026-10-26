//! JSON text exactly as Godot 3 `JSON.print(value, "\t")` writes it.

mod writer;

pub use writer::to_godot_json;
