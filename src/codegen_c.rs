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
            Type::Void => panic!("A struct cannot have type Void. This error indicates that there is a compiler issue, it should have been caught before code generation.") // this should not be possible
        }
        buffer.push_str(&format!(" {};\n", field.name));
    }
    // We already have a trailing newline from the last field
    buffer.push_str("};");
    buffer
}

/// Write an enum to C as a tagged union
fn write_enum(input: &Enum) -> String {
    // Create the enum for states
    let mut buffer: String = "typedef enum {\n".to_string();
    for field in input.fields.iter() {
        buffer.push_str(&format!("\t{},\n", field.name.to_uppercase()));
    }
    buffer.push_str(&format!("}} {}States;\n\n", input.name));
    // Create the union for data
    buffer.push_str("typedef union {\n");
    for field in input.fields.iter() {
        // Don't assign data to Void types (state only)
        match &field.field_type {
            Type::Void => continue,
            Type::String => buffer.push_str("\tchar"),
            Type::Integer => buffer.push_str("\tint_fast64_t"),
            Type::Boolean => buffer.push_str("\tbool"),
            Type::Custom(name) => buffer.push_str(&format!("\t {}", name)),
        }
        buffer.push_str(&format!(" {};\n", field.name));
    }
    buffer.push_str(&format!("}} {}Values;\n\n", input.name));
    // Create a joined struct (tagged union) to represent the combination
    buffer.push_str(&format!(
        "struct {} {{\n\t{}States tag;\n\t{}Values data;\n}};",
        input.name, input.name, input.name
    ));
    buffer
}

/// Write an AST to a string
pub fn write_all<'ast, I>(filename: &str, ast: I) -> String
where
    I: Iterator<Item = &'ast ASTNode>,
{
    let mut buffer = write_header(filename);
    for node in ast {
        match node {
            ASTNode::EnumDeclaration(e) => {
                buffer.push_str(&write_enum(e));
                buffer.push_str("\n\n");
            }
            ASTNode::StructDeclaration(s) => {
                buffer.push_str(&write_struct(s));
                buffer.push_str("\n\n");
            }
        }
    }
    buffer
}
