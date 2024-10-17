//! Generate C Code

use crate::parser::*;

/// Handles imports
///
/// TODO: actually dynamically handle imports...
fn write_header(filename: &str) -> String {
    format!(
        "// source: {}\n\n#include <stdbool.h>\n#include <stdint.h>\n\n",
        filename
    )
}

/// Write a Struct to a C struct
fn write_struct(input: &Struct) -> String {
    let mut buffer: String = format!("struct {} {{\n", input.name);
    for field in input.fields.iter() {
        match &field.field_type {
            Type::String => buffer.push_str("\tchar"),
            Type::Integer => buffer.push_str("\tint_fast64_t"),
            Type::Boolean => buffer.push_str("\tbool"),
            Type::Custom(name) => buffer.push_str(&format!("\t {}", name)),
        }
        buffer.push_str(&format!(" {};\n", field.name));
    }
    // We already have a trailing newline from the last field
    buffer.push_str("};");
    buffer
}

/// Write an AST to a string
///
/// TODO: expand AST beyond just `Struct`s
pub fn write_all<'a, I>(filename: &str, ast: I) -> String
where
    I: Iterator<Item = &'a Struct>,
{
    let mut buffer = write_header(filename);
    for s in ast {
        buffer.push_str(&write_struct(s));
        buffer.push_str("\n\n");
    }
    buffer
}
