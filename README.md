# Iona Programming Language

Iona is a high-level, imperative programming language with advanced features normally found in functional languages. Currently, Iona compiles to C. 

**Caveat**: This is a personal project, and is not ready for production (or any) use at the moment.

### Current Status

- Lexer: done
- Parser: done
- Static analysis
    - Type usage: in progress
    - Module/Import resolver: done, except for a bug (I think in the parser)
    - Typechecking: to do
- Basic codegen: in progress
- C runtime: to do

### Short Term To Do List

- ~~Get strings working~~
    - ~~Codegen works for Iona translation, but the templates need to permit modifying/importing type files (byte_array isn't importing bytes rn)~~
- Create code generation for functions 
- Handle memory management for doubly-nested (or deeper?) data structures
- Support compiler mode arguments (beyond "build")
- Major perf regression after switching to Paths from Strings (compiler perf loss)

## Language Features

### Effects System

Iona has an effects system aimed at improving supply chain security and program correctness. 

Explicit, opt-in permissions are required for functions to use "side effects" (like file i/o, network i/o, and so on). This requirement, plus (forthcoming) tooling to make auditing these permissions easy, would reduce the risk of malicious packages masquerading as harmless packages. It also reduces the risk of untested (or insufficiently tested) AI generated code. For instance, a function with the type signature and permission set below should raise some eyebrows.

```rust
fn foo(a: Int, b: Int) -> Int {
	@metadata {
		Is: Public;
		Uses: ReadFile, WriteFile;
	}
    ...
}
```

### Contracts & Refinement Types

Iona supports contracts: runtime checks to prevent a program from entering an invalid state. There are three types of supported contract:

1. Preconditions: checks before a function is executed
2. Postconditions: checks on the result of a function, before it's returned
3. Invariants: checks during function execution

The goal of contracts is to try and catch potential runtime errors at compile time. Suppose you have a division function. You could always manually check in the body that denominator != 0, but if you make it a contract the compiler can warn you ahead of time about runtime problems based on the inputs you provide. For instance, when composing functions we can check that the post conditions of the inner function are at least as strict as the pre conditions of outer function.

At least with pre- and post- conditions this is the same idea as [refinement types](https://en.wikipedia.org/wiki/Refinement_type), like Liquid Haskell.

## Compiler Usage

If you've cloned the repo, commands can be forwarded through `cargo run`.

```sh
cargo run ____
```

For example, to build Iona's standard library you can run

```sh
cargo run build stdlib
```

Other options are part of the `cli.rs` file (and its associated cargo docs).

# To Fix

### Parser Error System

Proposal

A Clean System for Parser Error Handling

The core principle of this new design is the separation of concerns:

 1 Parsing Logic: The primary job of a parser function is to recognize a pattern in the token stream and produce an AST
   node. Its return value should reflect success or failure at that task.
 2 Diagnostic Collection: Errors, warnings, and lints are byproducts of the parsing process. They should be collected in
   a central place, not passed around in return values.
 3 Control Flow: The parser needs a way to handle branching logic (e.g., "is this a function or a struct?") and to
   recover from syntax errors. This should be handled explicitly.

Here's how we can implement this:

1. Centralized Diagnostic Collection

The Parser struct itself will become the single source of truth for all diagnostics.

```
// In Parser struct
pub diagnostics: Vec<Diagnostic>,
```

Any parser function, at any time, can add a warning or error to this list (e.g., self.diagnostics.push(...)). This
completely decouples diagnostic reporting from the function's return value. A function can succeed and still report a
warning.

2. A Return Type Focused on Control Flow

Parser functions will return a Result<T, ParseError>.

 • T: The successfully parsed AST node (e.g., Function, Expr).
 • ParseError: An enum that only describes the reason for the parsing failure, for control-flow purposes. It does not
   contain the diagnostic message itself.

```
enum ParseError {
    /// A recoverable failure. The tokens didn't match the expected rule.
    /// This is used for speculative parsing. It is silent and does not
    /// generate a diagnostic. The caller should backtrack and try another rule.
    AttemptFailed,

    /// A non-recoverable, definite syntax error. The tokens are invalid.
    /// The function that returns this is responsible for adding a
    /// detailed `Diagnostic` to the parser's central `diagnostics` list
    /// before returning.
    Fatal,
}
```

3. Clear Distinction Between Failure Types

With this structure, the distinction you're looking for becomes crystal clear:

 • Incorrect Parser Attempt: A function like parse_function looks for the fn keyword. If it's not there, it simply
   returns Err(ParseError::AttemptFailed). No diagnostic is generated. The caller (parse_top_level_declaration) knows
   this isn't an error, it just means the current declaration isn't a function, and it can now try parse_struct.
 • True Parsing Error: The parse_function finds fn foo(a: Int) -> {} (missing return type). This is an unambiguous
   syntax error. The function will:
    1 Create a Diagnostic with a specific error message ("expected return type after ->").
    2 Push it to self.diagnostics.
    3 Return Err(ParseError::Fatal).
 • Warnings: A function finds a valid but questionable pattern (e.g., a variable name that shadows another). It can
   create a Diagnostic with level Warning, push it to self.diagnostics, and then succeed, returning Ok(the_parsed_node).

4. Error Recovery and Accumulation

The main parse_all loop becomes the recovery point. It will repeatedly call parse_top_level_declaration.

```
// Pseudocode for the main parse loop
while !parser.at_end() {
    match parser.parse_top_level_declaration() {
        Ok(ast_node) => {
            nodes.push(ast_node);
        },
        Err(ParseError::Fatal) => {
            // A fatal error was detected and already logged.
            // Now, we recover by skipping tokens until we find
            // something that looks like the start of the next
            // declaration (e.g., 'fn', 'struct', or just a newline).
            parser.synchronize();
        },
        Err(ParseError::AttemptFailed) => {
            // This means we've likely reached the end of the file
            // or there's an unexpected token at the top level.
            // Log an error and synchronize.
            let diagnostic = Diagnostic::new_error(...);
            parser.diagnostics.push(diagnostic);
            parser.synchronize();
        }
    }
}
```

This synchronize() function is key to error accumulation. It prevents a single syntax error from derailing the entire
parsing process, allowing the parser to continue and find subsequent, unrelated errors in the same file.

By adopting this model, we achieve all of your goals: a clean separation of concerns, clear semantics for different
failure types, robust error accumulation, and a simple strategy for error recovery.

