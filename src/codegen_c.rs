//! Generate C Code
//!
//! Note: we don't lift the type writing into a function because it's somewhat context dependent (ex. strings cannot have Void types but Enums can)

use std::borrow::Cow;
use std::collections::HashSet;
use std::fs;
use std::iter::zip;

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
pub trait TemplateInstance {
    fn get_type(&self) -> &Type;
    fn get_name(&self) -> &str;
    fn get_header_file(&self) -> &str;
    fn get_header_name(&self) -> &str;
}

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
///
/// `other_imports`: what other modules do we need to make this array work?
fn monomorphize_array_template(
    inner_type: &Type,
    template: &str,
    array_type_name: &str,
    type_method_prefix: &str,
    c_type: &str,
) -> String {
    let elem_type = c_type;
    let prefix = type_method_prefix;
    // TODO: support nested types, this will require a loop and/or recursion
    let imports = match type_to_std_lib(&inner_type) {
        Some(t) => &format!("#include \"{}\"\n", t),
        None => "",
    };
    template
        .replace("ARRAY_NAME", &array_type_name)
        .replace("ELEM_TYPE", elem_type)
        .replace("PREFIX", prefix)
        .replace("<OTHER_IMPORTS>", imports)
}

/// Create the C-side name for a given type, handling nested types recursively
fn boxed_type_name(type_: &Type) -> String {
    match type_ {
        Type::Array(inner) => format!("{}Array", boxed_type_name(inner)),
        _ => write_fn_arg_type(type_).to_string(),
    }
}

impl MonomorphizedArray {
    fn new(type_: &Type) -> MonomorphizedArray {
        let template = load_c_template("array.h");
        let header_file = monomorphize_array_template(
            type_,
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

pub fn generate_templated_libs(type_table: &TypeTable) -> Vec<Box<dyn TemplateInstance>> {
    let mut generated_libs: Vec<Box<dyn TemplateInstance>> = Vec::new();

    fn collect_array_types(t: &Type, set: &mut HashSet<Type>) {
        if let Type::Array(inner) = t {
            set.insert(t.clone());
            collect_array_types(inner, set);
        }
    }

    let mut all_array_types = HashSet::new();
    for t in type_table.type_list.iter() {
        collect_array_types(t, &mut all_array_types);
    }

    for t in all_array_types {
        if let Type::Array(inner) = t {
            let data = MonomorphizedArray::new(&inner);
            generated_libs.push(Box::new(data));
        }
    }

    generated_libs
}

pub fn emit_templated_stdlib_files(generated_libs: &Vec<Box<dyn TemplateInstance>>) {
    for lib in generated_libs.iter() {
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

/// Input a type and receive the name of the header file which implements it
fn type_to_std_lib(type_: &Type) -> Option<String> {
    match type_ {
        Type::String => Some("gen_strings.h".to_string()),
        Type::Integer | Type::Float => Some("numbers.h".to_string()),
        Type::Byte => Some("bytes.h".to_string()),
        Type::Boolean => Some("<stdbool.h>".to_string()),
        Type::Array(inner) => Some(format!(
            "gen_{}_array.h",
            write_fn_arg_type(inner).to_lowercase()
        )),
        _ => None,
    }
}

/// Check the Type Table to see which standard libraries we need
fn identify_std_libs(type_table: &TypeTable, filename: &str) -> Vec<String> {
    let mut pre_existing_lib_names: Vec<String> = Vec::new();
    let relevant_types = type_table
        .types_used_by_module
        .get(filename)
        .expect(&format!(
            "creating imports failed for {}, could not find file name in type table\nTable:\n{:?}",
            filename, type_table.types_used_by_module
        ));
    for t in relevant_types.iter() {
        if let Some(h) = type_to_std_lib(t) {
            pre_existing_lib_names.push(h);
        }
    }
    pre_existing_lib_names
}

/// Handles import for core libraries
fn write_header(type_table: &TypeTable, filename: &str, is_stdlib: bool) -> String {
    let relevant_types = type_table
        .types_used_by_module
        .get(filename)
        .expect(&format!(
            "creating imports failed for {}, could not find file name in type table\nTable:\n{:?}",
            filename, type_table.types_used_by_module
        ));
    let mut buffer = format!("// source: {}\n\n", filename);
    for (t, i) in zip(relevant_types, identify_std_libs(type_table, filename)) {
        // If we're creating a stdlib file, then we're all in the same folder
        if is_stdlib {
            buffer.push_str(&format!("#include \"{}\"", i));
        } else {
            // If we're creating a user file, then stdlib files are in a parallel folder and custom files are in this directory
            match t {
                Type::Custom(_) => {
                    buffer.push_str(&format!("#include \"{}\"\n", i));
                }
                _ => {
                    // Actual C stdlib
                    if i.starts_with('<') && i.ends_with('>') {
                        buffer.push_str(&format!("#include {}", i));
                    } else {
                        // Some C file we wrote
                        buffer.push_str(&format!("#include \"../c_libs/{}\"", i));
                    }
                }
            }
        }
        buffer += "\n";
    }
    // Extra newline for separating imports from rest of file
    buffer += "\n";
    buffer
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
    buffer.pop(); // pop comma
    buffer.pop(); // pop space
    buffer.push(')');
    buffer.push(';');
    buffer
}

// -------------------- All Together --------------------

/// Write an AST to a string
pub fn write_all<'ast, I>(ast: I, type_table: &TypeTable, filename: &str, is_stdlib: bool) -> String
where
    I: Iterator<Item = &'ast ASTNode>,
{
    let mut buffer = write_header(type_table, filename, is_stdlib);
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

// -------------------- Unit Tests --------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aggregation::TypeTable;
    use crate::lexer::Lexer;
    use crate::parser::Parser;

    #[test]
    fn monomorphize_nested_arrays() {
        const PROGRAM: &'static str = r#"
fn main() -> Void {
    let x: Array<Int>;
    let y: Array<Array<String>>;
    let z: Array<Array<Array<Bool>>>;
}
"#;
        let mut lexer = Lexer::new("test.iona");
        lexer.lex(PROGRAM);
        let mut parser = Parser::new(lexer.token_stream);
        let out = parser.parse_all();
        assert!(out.output.is_some());
        let ast = out.output.unwrap();

        let mut type_table = TypeTable::new();
        type_table.update(&ast, "test.iona");

        let generated_libs = generate_templated_libs(&type_table);

        assert_eq!(generated_libs.len(), 6);
        let names: HashSet<String> = generated_libs
            .iter()
            .map(|lib| lib.get_header_name().to_string())
            .collect();
        // Check for all expected monomorphizations
        assert!(names.contains("gen_integer_array.h"));
        assert!(names.contains("gen_string_array.h"));
        assert!(names.contains("gen_stringarray_array.h"));
        assert!(names.contains("gen_bool_array.h"));
        assert!(names.contains("gen_boolarray_array.h"));
        assert!(names.contains("gen_boolarrayarray_array.h"));
    }

    #[test]
    fn boxed_type_naming() {
        let t1 = Type::Array(Box::new(Type::Integer));
        assert_eq!(boxed_type_name(&t1), "IntegerArray");

        let t2 = Type::Array(Box::new(Type::Array(Box::new(Type::String))));
        assert_eq!(boxed_type_name(&t2), "StringArrayArray");

        let t3 = Type::Array(Box::new(Type::Array(Box::new(Type::Array(Box::new(
            Type::Boolean,
        ))))));
        assert_eq!(boxed_type_name(&t3), "boolArrayArrayArray");
    }
}
