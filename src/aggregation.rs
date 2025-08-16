//! State/Tables for the compiler

use std::collections::hash_map::Entry;
use std::collections::{HashMap, HashSet};

use crate::parser::{ASTNode, DataProperties, Enum, FunctionProperties, Statement, Struct, Type};

pub struct ParsingTables {
    pub modules: ModuleTable,
    pub types: TypeTable,
}

impl ParsingTables {
    pub fn new() -> ParsingTables {
        ParsingTables {
            modules: ModuleTable::new(),
            types: TypeTable::new(),
        }
    }

    pub fn update(&mut self, nodes: &Vec<ASTNode>, module_name: &str) {
        self.modules.update(nodes, module_name);
        self.types.update(nodes, module_name);
    }
}

/// Track all declared module imports
///
/// Each key in the HashMaps corresponds to a filename
///
/// - `parsing_status` lets us track if we need to load and parse a new module
///
/// - `imported_items` tracks everything that *any* module has tried to bring in from a certain file (functions, structs, enums, etc.)
///
/// - `exported_items` tracks all things marked as Export within a file
///
/// If the `imported_items` and the `exported_items` don't align, then we've got a problem!
#[derive(Debug, Clone, PartialEq)]
pub struct ModuleTable {
    pub parsing_status: HashMap<String, bool>,
    imported_items: HashMap<String, HashSet<String>>,
    public_items: HashMap<String, HashSet<String>>,
    exported_items: HashMap<String, HashSet<String>>,
}

impl ModuleTable {
    pub fn new() -> ModuleTable {
        ModuleTable {
            parsing_status: HashMap::new(),
            imported_items: HashMap::new(),
            public_items: HashMap::new(),
            exported_items: HashMap::new(),
        }
    }

    pub fn update(&mut self, ast: &Vec<ASTNode>, module_name: &str) {
        for node in ast {
            match node {
                ASTNode::ImportStatement(i) => {
                    // Mark this file as needing to be parsed if we haven't seen it before
                    self.parsing_status.entry(i.file.clone()).or_insert(false);

                    // Handle the imported items
                    match self.imported_items.entry(i.file.clone()) {
                        Entry::Occupied(mut entry) => {
                            // Add all items to the existing set
                            entry.get_mut().extend(i.items.iter().cloned());
                        }
                        Entry::Vacant(entry) => {
                            // Create a new set with all the items
                            let items_set: HashSet<String> = i.items.iter().cloned().collect();
                            entry.insert(items_set);
                        }
                    }
                }
                ASTNode::EnumDeclaration(e) => {
                    if e.properties.contains(&DataProperties::Export) {
                        self.exported_items
                            .entry(module_name.to_string())
                            .or_insert_with(HashSet::new)
                            .insert(e.name.clone());
                    }
                    if e.properties.contains(&DataProperties::Public) {
                        self.public_items
                            .entry(module_name.to_string())
                            .or_insert_with(HashSet::new)
                            .insert(e.name.clone());
                    }
                }
                ASTNode::StructDeclaration(s) => {
                    if s.properties.contains(&DataProperties::Export) {
                        self.exported_items
                            .entry(module_name.to_string())
                            .or_insert_with(HashSet::new)
                            .insert(s.name.clone());
                    }
                    if s.properties.contains(&DataProperties::Public) {
                        self.public_items
                            .entry(module_name.to_string())
                            .or_insert_with(HashSet::new)
                            .insert(s.name.clone());
                    }
                }
                ASTNode::FunctionDeclaration(f) => {
                    if f.properties.contains(&FunctionProperties::Export) {
                        self.exported_items
                            .entry(module_name.to_string())
                            .or_insert_with(HashSet::new)
                            .insert(f.name.clone());
                    }
                    if f.properties.contains(&FunctionProperties::Public) {
                        self.public_items
                            .entry(module_name.to_string())
                            .or_insert_with(HashSet::new)
                            .insert(f.name.clone());
                    }
                }
            }
        }
    }
}

/// Track all types declared and used throughout the program
///
/// All fields except `types_used_by_module` are "global" across the program
///
/// `types_used_by_module` tracks which types are *external* to a module so we know what that module has to import
#[derive(Debug, Clone, PartialEq)]
pub struct TypeTable {
    pub type_list: HashSet<Type>,
    pub types_used_by_module: HashMap<String, HashSet<Type>>,
    new_structs: HashMap<String, Struct>,
    new_enums: HashMap<String, Enum>,
}

impl TypeTable {
    pub fn new() -> TypeTable {
        TypeTable {
            type_list: HashSet::new(),
            types_used_by_module: HashMap::new(),
            new_structs: HashMap::new(),
            new_enums: HashMap::new(),
        }
    }

    // Helper method to process individual statements
    fn process_statement(
        &mut self,
        statement: &Statement,
        external_type_tracker: &mut HashSet<Type>,
    ) {
        match statement {
            Statement::VariableDeclaration { type_, .. } => {
                self.type_list.insert(type_.clone());
                external_type_tracker.insert(type_.clone());
            }
            Statement::Conditional(branches) => {
                for branch in branches {
                    for inner_statement in &branch.computations {
                        self.process_statement(inner_statement, external_type_tracker);
                    }
                }
            }
            // Add other statement types as needed
            _ => {}
        }
    }

    /// Walk an AST and build a set of all of the types used
    pub fn update(&mut self, ast: &Vec<ASTNode>, module_name: &str) {
        let mut types_used_by_module: HashSet<Type> = HashSet::new();
        for node in ast {
            match node {
                ASTNode::StructDeclaration(s) => {
                    // store new struct
                    self.new_structs.insert(s.name.clone(), s.clone());
                    // Add all used types to the type list
                    self.type_list.insert(Type::Custom(s.name.clone()));
                    for field in s.fields.iter() {
                        self.type_list.insert(field.field_type.clone());
                        types_used_by_module.insert(field.field_type.clone());
                    }
                }
                ASTNode::EnumDeclaration(e) => {
                    self.new_enums.insert(e.name.clone(), e.clone());
                    // Add all used types to the type list
                    self.type_list.insert(Type::Custom(e.name.clone()));
                    for field in e.fields.iter() {
                        self.type_list.insert(field.field_type.clone());
                        types_used_by_module.insert(field.field_type.clone());
                    }
                }
                ASTNode::FunctionDeclaration(f) => {
                    self.type_list.insert(f.returns.clone());
                    for arg in f.args.iter() {
                        self.type_list.insert(arg.field_type.clone());
                        types_used_by_module.insert(arg.field_type.clone());
                    }
                    for st in f.statements.iter() {
                        self.process_statement(st, &mut types_used_by_module);
                    }
                }
                ASTNode::ImportStatement(_) => {}
            }
        }
        self.types_used_by_module
            .insert(module_name.to_string(), types_used_by_module);
    }
}

// -------------------- Unit Tests --------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;
    use crate::parser::Parser;

    const PROGRAM: &'static str = r#"import npc with Creature;

        struct Animal {
            legs: Int,
            hair: Bool,
            feathers: Bool
            
            @metadata {
                Is: Public, Export;
                Derives: Eq, Show;
            }
        }

        fn do_nothing()
            @metadata {
                Is: Public;
            }
        {
        }

        enum Status {
            Alive,
            Dead

            @metadata {
                Is: Export;
            }
        }
    "#;

    #[test]
    fn construct_import_table() {
        let mut lexer = Lexer::new("test.iona");
        lexer.lex(PROGRAM);
        let mut parser = Parser::new(lexer.token_stream);
        let out = parser.parse_all();
        assert!(out.diagnostics.is_empty());
        let mut module_table = ModuleTable::new();
        module_table.update(&out.output.unwrap(), "test.iona");

        // Test import tracking
        assert!(module_table.parsing_status.contains_key("npc"));
        assert_eq!(*module_table.parsing_status.get("npc").unwrap(), false);
        let imported = module_table.imported_items.get("npc").unwrap();
        assert!(imported.contains("Creature"));
        assert_eq!(imported.len(), 1);

        // Test export tracking
        let exported = module_table.exported_items.get("test.iona").unwrap();
        assert!(exported.contains("Animal"));
        assert!(exported.contains("Status"));
        assert_eq!(exported.len(), 2);

        // Test public tracking
        let public = module_table.public_items.get("test.iona").unwrap();
        assert!(public.contains("Animal"));
        assert!(public.contains("do_nothing"));
        assert_eq!(public.len(), 2);
    }
}
