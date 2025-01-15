//! Generate C Code
//!
//! Note: we don't lift the type writing into a function because it's somewhat context dependent (ex. strings cannot have Void types but Enums can)

use std::borrow::Cow;
use std::fs;

use crate::parser::*;

// -------------------- Monomorphization Templates --------------------

/// Load a C header template for monomorphization
pub fn load_c_template(template_name: &str) -> String {
    fs::read_to_string(format!("/c_libs/templates/{}", template_name))
        .expect("could not find template, are the c_libs missing?")
}

/// Generate specialized C array code
fn monomorphize_array_template(template: &str, iona_type_name: &str, c_type_name: &str) -> String {
    let array_name = format!("{}Array", iona_type_name);
    let elem_type = c_type_name;
    let prefix = iona_type_name;
    template
        .replace("ARRAY_NAME", &array_name)
        .replace("ELEM_TYPE", elem_type)
        .replace("PREFIX", prefix)
}

// -------------------- Programmatic C Code --------------------

/// Handles import for core libraries
///
/// TODO: actually dynamically handle imports...
fn write_header(filename: &str) -> String {
    format!(
        "// source: {}\n\n#include <stdbool.h>\n#include <stdint.h>\n\n",
        filename
    )
}

/// Handles user defined imports
///
/// C doesn't have a notion of qualified imports so this is really simple (qualification is handled by the compiler)
fn write_import(input: &Import) -> String {
    format!("#include \"{}\"", input.file)
}

/// Write a Struct to a C struct
///
/// TODO! Replace generic's use of void pointer with Monomorphization (need a table to track this from call sites)
fn write_struct(input: &Struct) -> String {
    let mut buffer: String = format!("struct {} {{\n", input.name);
    for field in input.fields.iter() {
        match &field.field_type {
            Type::String => buffer.push_str("\tchar"),
            Type::Integer => buffer.push_str("\tint_fast64_t"),
            Type::Boolean => buffer.push_str("\tbool"),
            Type::Custom(name) => buffer.push_str(&format!("\t {}", name)),
            Type::Generic(_) => buffer.push_str("\tvoid*"),
            Type::Void => panic!("A struct cannot have type Void. This error indicates that there is a compiler issue, it should have been caught before code generation."), // this should not be possible
            _ => todo!()
        }
        buffer.push_str(&format!(" {};\n", field.name));
    }
    // We already have a trailing newline from the last field
    buffer.push_str("};\n");
    // C doesn't mark a struct as a type by default
    buffer.push_str(&format!("typedef struct {} {};", input.name, input.name));
    buffer
}

/// Write an enum to C as a tagged union
///
/// TODO! Replace generic's use of void pointer with Monomorphization (need a table to track this from call sites)
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
            Type::Generic(_) => buffer.push_str("\tvoid*"),
            Type::Custom(name) => buffer.push_str(&format!("\t {}", name)),
            _ => todo!(),
        }
        buffer.push_str(&format!(" {};\n", field.name));
    }
    buffer.push_str(&format!("}} {}Values;\n\n", input.name));
    // Create a joined struct (tagged union) to represent the combination
    buffer.push_str(&format!(
        "struct {} {{\n\t{}States tag;\n\t{}Values data;\n}};\n",
        input.name, input.name, input.name
    ));
    // C doesn't mark a struct as a type by default
    buffer.push_str(&format!("typedef struct {} {};", input.name, input.name));
    buffer
}

// -------------------- Functions --------------------

fn write_fn_type(input: &Type) -> Cow<'static, str> {
    match input {
        Type::String => Cow::Borrowed("char"),
        Type::Integer => Cow::Borrowed("int_fast64_t"),
        Type::Boolean => Cow::Borrowed("bool"),
        Type::Custom(name) => Cow::Owned(format!("{}", name)),
        Type::Generic(_) => Cow::Borrowed("void*"),
        Type::Void => Cow::Borrowed("void"),
        _ => todo!(),
    }
}

fn write_fn_declare(input: &Function) -> String {
    let mut buffer: String = format!("{} {}(", write_fn_type(&input.returns), input.name);
    for arg in &input.args {
        buffer += &format!("{} {}, ", write_fn_type(&arg.field_type), arg.name);
    }
    // Remove the trailing `, `
    buffer.pop();
    buffer.pop();
    buffer.push(')');
    buffer.push(';');
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
            ASTNode::ImportStatement(i) => {
                buffer.push_str(&write_import(i));
                buffer.push_str("\n\n");
            }
            ASTNode::FunctionDeclaration(f) => {
                buffer.push_str(&write_fn_declare(f));
            }
        }
    }
    buffer
}
