//! State/Tables for the compiler

use std::collections::hash_map::Entry;
use std::collections::{HashMap, HashSet};

use crate::parser::{ASTNode, Enum, Statement, Struct, Type};

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

    pub fn update(&mut self, nodes: &Vec<ASTNode>) {
        self.modules.update(nodes);
        self.types.update(nodes);
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

    pub fn update(&mut self, ast: &Vec<ASTNode>) {
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
                // ASTNode::EnumDeclaration(e) => {
                //     if e.properties.contains(&DataProperties::Export) => {
                //         match self.exported_items.entry(key)
                //     }
                // }
                _ => {}
            }
        }
    }
}

/// Track all types declared and used throughout the module
#[derive(Debug, Clone, PartialEq)]
pub struct TypeTable {
    pub type_list: HashSet<Type>,
    new_structs: HashMap<String, Struct>,
    new_enums: HashMap<String, Enum>,
}

impl TypeTable {
    pub fn new() -> TypeTable {
        TypeTable {
            type_list: HashSet::new(),
            new_structs: HashMap::new(),
            new_enums: HashMap::new(),
        }
    }

    // Helper method to process individual statements
    fn process_statement(&mut self, statement: &Statement) {
        match statement {
            Statement::VariableDeclaration { type_, .. } => {
                self.type_list.insert(type_.clone());
            }
            Statement::Conditional(branches) => {
                for branch in branches {
                    for inner_statement in &branch.computations {
                        self.process_statement(inner_statement);
                    }
                }
            }
            // Add other statement types as needed
            _ => {}
        }
    }

    /// Walk an AST and build a set of all of the types used
    pub fn update(&mut self, ast: &Vec<ASTNode>) {
        for node in ast {
            match node {
                ASTNode::StructDeclaration(s) => {
                    // store new struct
                    self.new_structs.insert(s.name.clone(), s.clone());
                    // Add all used types to the type list
                    self.type_list.insert(Type::Custom(s.name.clone()));
                    for field in s.fields.iter() {
                        self.type_list.insert(field.field_type.clone());
                    }
                }
                ASTNode::EnumDeclaration(e) => {
                    self.new_enums.insert(e.name.clone(), e.clone());
                    // Add all used types to the type list
                    self.type_list.insert(Type::Custom(e.name.clone()));
                    for field in e.fields.iter() {
                        self.type_list.insert(field.field_type.clone());
                    }
                }
                ASTNode::FunctionDeclaration(f) => {
                    self.type_list.insert(f.returns.clone());
                    for arg in f.args.iter() {
                        self.type_list.insert(arg.field_type.clone());
                    }
                    for st in f.statements.iter() {
                        self.process_statement(st);
                    }
                }
                ASTNode::ImportStatement(_) => {}
            }
        }
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
    "#;

    #[test]
    fn construct_import_table() {
        let mut lexer = Lexer::new("test.iona");
        lexer.lex(PROGRAM);
        let mut parser = Parser::new(lexer.token_stream);
        let out = parser.parse_all();
        assert!(out.diagnostics.is_empty());
        println!("{:#?}", &out.output);
        let mut import_table = ModuleTable::new();
        import_table.update(&out.output.unwrap());
        println!("{:#?}", import_table);
        assert!(import_table.parsing_status.contains_key("npc"));
        assert_eq!(*import_table.parsing_status.get("npc").unwrap(), false);
        // assert!(!import_table.imported_items.get("").unwrap().is_empty());
        // assert_eq!(import_table.imported_items, vec!["Creature".to_string()]);
    }
}
