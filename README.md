# Iona Programming Language

Iona is a high-level, imperative programming language with advanced features normally found in functional languages. Currently, Iona compiles to C. 

**Caveat**: This is a personal project, and is not ready for production (or any) use at the moment.

### Current Status

- Lexer: done
- Parser: done
- Static analysis
    - Type usage: in progress
    - Module/Import resolver: in progress'
    - Typechecking: to do
- Basic codegen: in progress
- C runtime: to do

### Short Term To Do List

- Create code generation for functions 
- Get strings working
- Handle memory management for doubly-nested data structures
- Support compiler mode arguments (beyond "build")
- Major perf regression after switching to Paths from Strings

## Language Features

### Effects System

Iona has an effects system aimed at improving supply chain security and program correctness. 

Explicit, opt-in permissions are required for functions to use "side effects" (like file i/o, network i/o, and so on). This requirement, plus (forthcoming) tooling to make auditing these permissions easy, would reduce the risk of malicious packages masquerading as harmless packages. For instance, a function with the type signature and permission set below should raise some eyebrows.

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