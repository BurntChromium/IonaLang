//! Generate C Code
//!
//! Note: we don't lift the type writing into a function because it's somewhat context dependent (ex. strings cannot have Void types but Enums can)

use std::borrow::Cow;
use std::fs;

use crate::aggregation::TypeTable;
use crate::parser::*;

// -------------------- Monomorphization Templates --------------------

/// Load a C header template for monomorphization
pub fn load_c_template(template_name: &str) -> String {
    fs::read_to_string(format!("c_libs/templates/{}", template_name)).expect(&format!(
        "could not find template for {}, are the c_libs missing? (check for /c_libs/templates/{})",
        template_name, template_name
    ))
}

/// A concrete, monomorphized type
///
/// header_file means the actual .h file, while header_name is the name of that file
trait TemplateInstance {
    fn get_type(&self) -> &Type;
    fn get_name(&self) -> &str;
    fn get_header_file(&self) -> &str;
    fn get_header_name(&self) -> &str;
}

/// TODO: extend this to handle doubly-nested types (Array<Array<Byte>> or Array<Map<String, T>> or whatever)
struct MonomorphizedArray {
    type_: Type,
    name: String,
    header_file: String,
    header_name: String,
}

/// Generate specialized C array code
///
/// `array_type_name`: something like ByteArray or IntArray
///
/// `type_method_prefix`: byte_array or int_array
///
/// `c_type`: the underlying c type (or the appropriate new type defined in C like `Integer` or `Float`)
fn monomorphize_array_template(
    template: &str,
    array_type_name: &str,
    type_method_prefix: &str,
    c_type: &str,
) -> String {
    let elem_type = c_type;
    let prefix = type_method_prefix;
    template
        .replace("ARRAY_NAME", &array_type_name)
        .replace("ELEM_TYPE", elem_type)
        .replace("PREFIX", prefix)
}

/// TODO: make recursive
fn boxed_type_name(type_: &Type) -> String {
    match type_ {
        Type::Array(inner) => format!("{}Array", write_fn_arg_type(inner)),
        _ => todo!(),
    }
}

impl MonomorphizedArray {
    fn new(type_: &Type) -> MonomorphizedArray {
        let template = load_c_template("array.h");
        let header_file = monomorphize_array_template(
            &template,
            &format!("{}Array", write_fn_arg_type(type_)),
            &format!("{}_array", write_fn_arg_type(type_).to_lowercase()),
            &write_fn_arg_type(type_),
        );
        let header_name: String =
            format!("gen_{}_array.h", write_fn_arg_type(type_).to_lowercase());
        MonomorphizedArray {
            type_: type_.clone(),
            name: write_fn_arg_type(&type_).to_string(),
            header_file,
            header_name,
        }
    }
}

impl TemplateInstance for MonomorphizedArray {
    fn get_type(&self) -> &Type {
        &self.type_
    }

    fn get_name(&self) -> &str {
        &self.name
    }

    fn get_header_file(&self) -> &str {
        &self.header_file
    }

    fn get_header_name(&self) -> &str {
        &self.header_name
    }
}

// -------------------- Programmatic C Code --------------------

/// Holds names of
///
/// - existing libs that we need to import
///
/// - bundled data for monomorphized templates
pub struct StdLibHandler {
    pre_existing_lib_names: Vec<String>,
    generated_libs: Vec<Box<dyn TemplateInstance>>,
}

/// Check the Type Table to see which standard libraries we need
///
/// This also emits import headers for generated
pub fn identify_std_libs(type_table: &TypeTable) -> StdLibHandler {
    let mut pre_existing_lib_names: Vec<String> = Vec::new();
    let mut generated_libs: Vec<Box<dyn TemplateInstance>> = Vec::new();
    for t in type_table.type_list.iter() {
        match t {
            Type::String => pre_existing_lib_names.push("gen_strings.h".to_string()),
            Type::Integer | Type::Float => pre_existing_lib_names.push("numbers.h".to_string()),
            Type::Byte => pre_existing_lib_names.push("bytes.h".to_string()),
            Type::Boolean => pre_existing_lib_names.push("stdbool.h".to_string()),
            Type::Array(inner) => {
                let data = MonomorphizedArray::new(inner);
                generated_libs.push(Box::new(data));
            }
            _ => {}
        }
    }
    StdLibHandler {
        pre_existing_lib_names,
        generated_libs,
    }
}

pub fn emit_templated_stdlib_files(lib_handler: &StdLibHandler) {
    for lib in lib_handler.generated_libs.iter() {
        fs::write(
            format!("c_libs/{}", lib.get_header_name()),
            lib.get_header_file(),
        )
        .expect(&format!(
            "Unable to write generated header file: {}",
            lib.get_header_name()
        ));
    }
}

/// Handles import for core libraries
///
/// TODO: actually dynamically handle imports...
fn write_header(filename: &str) -> String {
    format!(
        "// source: {}\n\n#include <stdbool.h>\n#include \"../c_libs/numbers.h\"\n\n",
        filename
    )
}

/// Handles user defined imports
///
/// C doesn't have a notion of qualified imports so this is really simple (qualification is handled by the compiler)
fn write_import(input: &Import) -> String {
    format!("#include \"{}.h\"", input.file)
}

/// Write a Struct to a C struct
///
/// TODO! Replace generic's use of void pointer with Monomorphization (need a table to track this from call sites)
fn write_struct(input: &Struct) -> String {
    let mut buffer: String = format!("struct {} {{\n", input.name);
    for field in input.fields.iter() {
        match &field.field_type {
            Type::String => buffer.push_str("\tString"),
            Type::Byte => buffer.push_str("\tByte"),
            Type::Integer => buffer.push_str("\tInteger"),
            Type::Boolean => buffer.push_str("\tbool"),
            Type::Custom(name) => buffer.push_str(&format!("\t {}", name)),
            Type::Generic(_) => buffer.push_str("\tvoid*"),
            Type::Array(_) => buffer.push_str(&format!("\t{}", boxed_type_name(&field.field_type))),
            Type::Void => panic!("A struct cannot have type Void. This error indicates that there is a compiler issue, it should have been caught before code generation."), // this should not be possible
            _ => {
                println!("WARNING: cannot emit type {:?} yet", &field.field_type);
                buffer.push_str("\tNOT_IMPLEMENTED");
            }
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
            Type::String => buffer.push_str("\tString"),
            Type::Byte => buffer.push_str("\tByte"),
            Type::Integer => buffer.push_str("\tInteger"),
            Type::Boolean => buffer.push_str("\tbool"),
            Type::Generic(_) => buffer.push_str("\tvoid*"),
            Type::Array(_) => buffer.push_str(&format!("\t{}", boxed_type_name(&field.field_type))),
            Type::Custom(name) => buffer.push_str(&format!("\t {}", name)),
            Type::Void => continue,
            _ => {
                println!("WARNING: cannot emit type {:#?} yet", &field.field_type);
                buffer.push_str("\tNOT_IMPLEMENTED");
            }
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

fn write_fn_arg_type(input: &Type) -> Cow<'static, str> {
    match input {
        Type::String => Cow::Borrowed("String"),
        Type::Byte => Cow::Borrowed("Byte"),
        Type::Integer => Cow::Borrowed("Integer"),
        Type::Float => Cow::Borrowed("Float"),
        Type::Boolean => Cow::Borrowed("bool"),
        Type::Custom(name) => Cow::Owned(format!("{}", name)),
        Type::Generic(_) => Cow::Borrowed("void*"),
        Type::Array(_) => Cow::Owned(boxed_type_name(input)),
        Type::Void => Cow::Borrowed("void"),
        _ => todo!(),
    }
}

fn write_fn_declare(input: &Function) -> String {
    let mut buffer: String = format!("{} {}(", write_fn_arg_type(&input.returns), input.name);
    for arg in &input.args {
        buffer += &format!("{} {}, ", write_fn_arg_type(&arg.field_type), arg.name);
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
