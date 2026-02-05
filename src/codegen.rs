use crate::ast::{Expr, Literal, Program, Type, WordDef};
use crate::error::CodeGenError;
use std::collections::HashMap;
use wasm_encoder::{
    CodeSection, ExportKind, ExportSection, Function, FunctionSection,
    Instruction, MemorySection, MemoryType, Module, TypeSection, ValType,
};

/// WASM code generator for Obsidian programs.
pub struct CodeGen {
    /// Word name -> function index mapping.
    word_indices: HashMap<String, u32>,
    /// Type index for each unique signature.
    type_indices: HashMap<(Vec<ValType>, Vec<ValType>), u32>,
    /// Next available type index.
    next_type_index: u32,
}

impl CodeGen {
    /// Create a new code generator.
    pub fn new() -> Self {
        Self {
            word_indices: HashMap::new(),
            type_indices: HashMap::new(),
            next_type_index: 0,
        }
    }

    /// Generate WASM binary from a program.
    pub fn generate(&mut self, program: &Program) -> Result<Vec<u8>, CodeGenError> {
        let mut module = Module::new();
        
        // Phase 1: Collect all word signatures and assign indices
        for (i, word) in program.words.iter().enumerate() {
            self.word_indices.insert(word.name.clone(), i as u32);
        }

        // Type section (must come first)
        let mut types = TypeSection::new();
        for word in &program.words {
            let params = self.effect_to_params(&word.effect.inputs);
            let results = self.effect_to_results(&word.effect.outputs);
            let sig = (params.clone(), results.clone());
            
            if !self.type_indices.contains_key(&sig) {
                types.ty().function(params, results);
                self.type_indices.insert(sig.clone(), self.next_type_index);
                self.next_type_index += 1;
            }
        }
        
        // Add type for _start (no params, no results)
        let start_sig = (vec![], vec![]);
        if !self.type_indices.contains_key(&start_sig) {
            types.ty().function(vec![], vec![]);
            self.type_indices.insert(start_sig.clone(), self.next_type_index);
            self.next_type_index += 1;
        }
        
        module.section(&types);

        // Function section (declares types for each function)
        let mut functions = FunctionSection::new();
        for word in &program.words {
            let params = self.effect_to_params(&word.effect.inputs);
            let results = self.effect_to_results(&word.effect.outputs);
            let type_idx = self.type_indices[&(params, results)];
            functions.function(type_idx);
        }
        // Add _start function
        let start_type_idx = self.type_indices[&start_sig];
        functions.function(start_type_idx);
        module.section(&functions);

        // Memory section (1 page = 64KB)
        let mut memory = MemorySection::new();
        memory.memory(MemoryType {
            minimum: 1,
            maximum: Some(16),
            memory64: false,
            shared: false,
            page_size_log2: None,
        });
        module.section(&memory);

        // Export section
        let mut exports = ExportSection::new();
        exports.export("memory", ExportKind::Memory, 0);
        
        // Export main if it exists, otherwise export _start
        let main_idx = self.word_indices.get("main");
        let start_idx = program.words.len() as u32;
        exports.export("_start", ExportKind::Func, start_idx);
        
        // Also export user functions
        for (name, &idx) in &self.word_indices {
            exports.export(name, ExportKind::Func, idx);
        }
        module.section(&exports);

        // Code section
        let mut codes = CodeSection::new();
        
        // Generate code for each word
        for word in &program.words {
            let func = self.generate_word(word)?;
            codes.function(&func);
        }
        
        // Generate _start function
        let start_func = self.generate_start(main_idx)?;
        codes.function(&start_func);
        
        module.section(&codes);

        Ok(module.finish())
    }

    /// Convert stack effect inputs to WASM parameter types.
    fn effect_to_params(&self, inputs: &[crate::ast::StackItem]) -> Vec<ValType> {
        inputs.iter().map(|i| self.item_to_valtype(i)).collect()
    }

    /// Convert stack effect outputs to WASM result types.
    fn effect_to_results(&self, outputs: &[crate::ast::StackItem]) -> Vec<ValType> {
        outputs.iter().map(|i| self.item_to_valtype(i)).collect()
    }

    /// Convert a stack item to WASM value type.
    fn item_to_valtype(&self, item: &crate::ast::StackItem) -> ValType {
        match &item.typ {
            Some(Type::I32) => ValType::I32,
            Some(Type::I64) => ValType::I64,
            Some(Type::F32) => ValType::F32,
            Some(Type::F64) => ValType::F64,
            Some(Type::Bool) => ValType::I32, // Bools are i32 in WASM
            None => ValType::I64, // Default to i64 for untyped
        }
    }

    /// Generate code for a word definition.
    fn generate_word(&self, word: &WordDef) -> Result<Function, CodeGenError> {
        let num_params = word.effect.inputs.len() as u32;
        
        // We need locals for stack manipulation
        // For now, use one local per stack slot
        let mut func = Function::new(vec![]);
        
        // Generate body
        self.generate_body(&mut func, &word.body, num_params)?;
        
        // End function
        func.instruction(&Instruction::End);
        
        Ok(func)
    }

    /// Generate the _start entry point.
    fn generate_start(&self, main_idx: Option<&u32>) -> Result<Function, CodeGenError> {
        let mut func = Function::new(vec![]);
        
        if let Some(&idx) = main_idx {
            func.instruction(&Instruction::Call(idx));
        }
        
        func.instruction(&Instruction::End);
        Ok(func)
    }

    /// Generate code for a body (list of expressions).
    fn generate_body(
        &self,
        func: &mut Function,
        body: &[Expr],
        _num_params: u32,
    ) -> Result<(), CodeGenError> {
        for expr in body {
            self.generate_expr(func, expr)?;
        }
        Ok(())
    }

    /// Generate code for a single expression.
    fn generate_expr(&self, func: &mut Function, expr: &Expr) -> Result<(), CodeGenError> {
        match expr {
            Expr::Literal(lit) => {
                self.generate_literal(func, lit);
            }
            
            Expr::Word { name, .. } => {
                self.generate_word_call(func, name)?;
            }
            
            Expr::If { then_branch, else_branch, .. } => {
                // WASM if-else: if <blocktype> <then> else <else> end
                // The condition is already on stack (from previous expression)
                
                // Determine result type based on branch effects
                // For simplicity, assume branches are balanced
                func.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
                self.generate_body(func, then_branch, 0)?;
                
                if let Some(else_body) = else_branch {
                    func.instruction(&Instruction::Else);
                    self.generate_body(func, else_body, 0)?;
                }
                
                func.instruction(&Instruction::End);
            }
            
            Expr::While { cond, body, .. } => {
                // WASM while pattern:
                // block $exit
                //   loop $loop
                //     <cond>
                //     i32.eqz
                //     br_if $exit
                //     <body>
                //     br $loop
                //   end
                // end
                func.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
                func.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
                
                // Generate condition
                self.generate_body(func, cond, 0)?;
                
                // If false (0), exit loop
                func.instruction(&Instruction::I32Eqz);
                func.instruction(&Instruction::BrIf(1)); // br to outer block
                
                // Generate body
                self.generate_body(func, body, 0)?;
                
                // Loop back
                func.instruction(&Instruction::Br(0)); // br to loop start
                
                func.instruction(&Instruction::End); // end loop
                func.instruction(&Instruction::End); // end block
            }
            
            Expr::Times { body, .. } => {
                // WASM times pattern (count already on stack):
                // block $exit
                //   loop $loop
                //     local.get $count
                //     i32.eqz
                //     br_if $exit
                //     <body>
                //     local.get $count
                //     i32.const 1
                //     i32.sub
                //     local.set $count
                //     br $loop
                //   end
                // end
                
                // For now, simplified version using the stack
                // This is tricky - we need a local for the counter
                // TODO: Proper local allocation
                
                // Simple infinite-loop avoidance: just do body once for now
                // Full implementation needs local allocation
                func.instruction(&Instruction::Drop); // drop count
                self.generate_body(func, body, 0)?;
            }
        }
        Ok(())
    }

    /// Generate code for a literal.
    fn generate_literal(&self, func: &mut Function, lit: &Literal) {
        match lit {
            Literal::Integer(n) => {
                func.instruction(&Instruction::I64Const(*n));
            }
            Literal::Float(f) => {
                func.instruction(&Instruction::F64Const(wasm_encoder::Ieee64::from(*f)));
            }
            Literal::Bool(b) => {
                func.instruction(&Instruction::I32Const(if *b { 1 } else { 0 }));
            }
            Literal::String(_s) => {
                // Strings need to be stored in memory
                // For now, just push address 0
                // TODO: String interning and memory layout
                func.instruction(&Instruction::I32Const(0));
            }
        }
    }

    /// Generate code for a word/builtin call.
    fn generate_word_call(&self, func: &mut Function, name: &str) -> Result<(), CodeGenError> {
        match name {
            // Stack operations
            "dup" => {
                // WASM doesn't have dup, need local
                // For i64 values on stack: use temporary local
                // Simplified: assume value is i64
                // TODO: proper local management
                func.instruction(&Instruction::LocalGet(0)); // get param 0 again (hack)
            }
            "drop" => {
                func.instruction(&Instruction::Drop);
            }
            "swap" => {
                // WASM has no swap, need locals
                // TODO: implement with locals
            }
            "over" => {
                // (a b -- a b a)
                // TODO: implement with locals
            }
            "rot" | "nip" | "tuck" => {
                // TODO: implement with locals
            }
            "2dup" => {
                // (a b -- a b a b) - requires locals
                // TODO: implement with locals
            }
            "2drop" => {
                // (a b --)
                func.instruction(&Instruction::Drop);
                func.instruction(&Instruction::Drop);
            }
            "2swap" => {
                // (a b c d -- c d a b) - requires locals
                // TODO: implement with locals
            }

            // Arithmetic (i64 operations)
            "+" => { func.instruction(&Instruction::I64Add); }
            "-" => { func.instruction(&Instruction::I64Sub); }
            "*" => { func.instruction(&Instruction::I64Mul); }
            "/" => { func.instruction(&Instruction::I64DivS); }
            "mod" => { func.instruction(&Instruction::I64RemS); }
            "negate" => {
                func.instruction(&Instruction::I64Const(-1));
                func.instruction(&Instruction::I64Mul);
            }
            "abs" => {
                // abs for i64: if negative, negate
                // Simplified: use select or if
                // TODO: proper implementation
            }
            "min" => {
                // TODO: implement with select
            }
            "max" => {
                // TODO: implement with select
            }

            // Comparison (produce i32 bool)
            "=" => {
                func.instruction(&Instruction::I64Eq);
            }
            "!=" => {
                func.instruction(&Instruction::I64Ne);
            }
            "<" => {
                func.instruction(&Instruction::I64LtS);
            }
            ">" => {
                func.instruction(&Instruction::I64GtS);
            }
            "<=" => {
                func.instruction(&Instruction::I64LeS);
            }
            ">=" => {
                func.instruction(&Instruction::I64GeS);
            }

            // Logic (i32 booleans)
            "and" => {
                func.instruction(&Instruction::I32And);
            }
            "or" => {
                func.instruction(&Instruction::I32Or);
            }
            "not" => {
                func.instruction(&Instruction::I32Eqz);
            }

            // Bitwise (i64)
            "band" => {
                func.instruction(&Instruction::I64And);
            }
            "bor" => {
                func.instruction(&Instruction::I64Or);
            }
            "bxor" => {
                func.instruction(&Instruction::I64Xor);
            }
            "bnot" => {
                func.instruction(&Instruction::I64Const(-1));
                func.instruction(&Instruction::I64Xor);
            }
            "shl" => {
                func.instruction(&Instruction::I64Shl);
            }
            "shr" => {
                func.instruction(&Instruction::I64ShrS);
            }

            // Memory
            "@" => {
                // fetch i64 from address
                func.instruction(&Instruction::I64Load(wasm_encoder::MemArg {
                    offset: 0,
                    align: 3, // 8-byte alignment
                    memory_index: 0,
                }));
            }
            "!" => {
                // store i64 at address (value addr --)
                func.instruction(&Instruction::I64Store(wasm_encoder::MemArg {
                    offset: 0,
                    align: 3,
                    memory_index: 0,
                }));
            }
            "c@" => {
                // fetch byte from address
                func.instruction(&Instruction::I32Load8U(wasm_encoder::MemArg {
                    offset: 0,
                    align: 0,
                    memory_index: 0,
                }));
            }
            "c!" => {
                // store byte at address
                func.instruction(&Instruction::I32Store8(wasm_encoder::MemArg {
                    offset: 0,
                    align: 0,
                    memory_index: 0,
                }));
            }
            "alloc" => {
                // Simplified: no-op for now
                // TODO: bump allocator
            }

            // IO (no-ops in pure WASM)
            "print" => {
                func.instruction(&Instruction::Drop); // consume value
            }
            "emit" => {
                func.instruction(&Instruction::Drop);
            }

            // User-defined word
            _ => {
                if let Some(&idx) = self.word_indices.get(name) {
                    func.instruction(&Instruction::Call(idx));
                } else {
                    return Err(CodeGenError {
                        message: format!("unknown word: {}", name),
                    });
                }
            }
        }
        Ok(())
    }
}

impl Default for CodeGen {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{Program, WordDef, StackEffect, StackItem};
    use crate::span::Span;

    fn make_word(name: &str, inputs: usize, outputs: usize, body: Vec<Expr>) -> WordDef {
        WordDef {
            name: name.to_string(),
            effect: StackEffect {
                inputs: (0..inputs).map(|_| StackItem::named("a")).collect(),
                outputs: (0..outputs).map(|_| StackItem::named("b")).collect(),
            },
            body,
            span: Span::default(),
        }
    }

    fn word_call(name: &str) -> Expr {
        Expr::Word { name: name.to_string(), span: Span::default() }
    }

    fn int_lit(n: i64) -> Expr {
        Expr::Literal(Literal::Integer(n))
    }

    #[test]
    fn test_codegen_new() {
        let cg = CodeGen::new();
        assert!(cg.word_indices.is_empty());
    }

    #[test]
    fn test_empty_program() {
        let mut cg = CodeGen::new();
        let program = Program::new();
        let wasm = cg.generate(&program).unwrap();
        assert!(!wasm.is_empty());
        // Valid WASM starts with magic number
        assert_eq!(&wasm[0..4], b"\x00asm");
    }

    #[test]
    fn test_simple_word() {
        let mut cg = CodeGen::new();
        let word = make_word("foo", 0, 0, vec![]);
        let program = Program { words: vec![word] };
        let wasm = cg.generate(&program).unwrap();
        assert!(!wasm.is_empty());
        assert_eq!(&wasm[0..4], b"\x00asm");
    }

    #[test]
    fn test_word_with_literal() {
        let mut cg = CodeGen::new();
        let word = make_word("foo", 0, 1, vec![int_lit(42)]);
        let program = Program { words: vec![word] };
        let wasm = cg.generate(&program).unwrap();
        assert!(!wasm.is_empty());
    }

    #[test]
    fn test_arithmetic_add() {
        let mut cg = CodeGen::new();
        let word = make_word("add", 2, 1, vec![word_call("+")]);
        let program = Program { words: vec![word] };
        let wasm = cg.generate(&program).unwrap();
        assert!(!wasm.is_empty());
    }

    #[test]
    fn test_arithmetic_sub() {
        let mut cg = CodeGen::new();
        let word = make_word("sub", 2, 1, vec![word_call("-")]);
        let program = Program { words: vec![word] };
        let wasm = cg.generate(&program).unwrap();
        assert!(!wasm.is_empty());
    }

    #[test]
    fn test_arithmetic_mul() {
        let mut cg = CodeGen::new();
        let word = make_word("mul", 2, 1, vec![word_call("*")]);
        let program = Program { words: vec![word] };
        let wasm = cg.generate(&program).unwrap();
        assert!(!wasm.is_empty());
    }

    #[test]
    fn test_comparison() {
        let mut cg = CodeGen::new();
        let word = make_word("eq", 2, 1, vec![word_call("=")]);
        let program = Program { words: vec![word] };
        let wasm = cg.generate(&program).unwrap();
        assert!(!wasm.is_empty());
    }

    #[test]
    fn test_drop() {
        let mut cg = CodeGen::new();
        let word = make_word("discard", 1, 0, vec![word_call("drop")]);
        let program = Program { words: vec![word] };
        let wasm = cg.generate(&program).unwrap();
        assert!(!wasm.is_empty());
    }

    #[test]
    fn test_if_simple() {
        let mut cg = CodeGen::new();
        let word = make_word("test_if", 1, 0, vec![
            Expr::If {
                then_branch: vec![],
                else_branch: None,
                span: Span::default(),
            },
        ]);
        let program = Program { words: vec![word] };
        let wasm = cg.generate(&program).unwrap();
        assert!(!wasm.is_empty());
    }

    #[test]
    fn test_if_else() {
        let mut cg = CodeGen::new();
        let word = make_word("test_if_else", 1, 0, vec![
            Expr::If {
                then_branch: vec![int_lit(1), word_call("drop")],
                else_branch: Some(vec![int_lit(2), word_call("drop")]),
                span: Span::default(),
            },
        ]);
        let program = Program { words: vec![word] };
        let wasm = cg.generate(&program).unwrap();
        assert!(!wasm.is_empty());
    }

    #[test]
    fn test_while_loop() {
        let mut cg = CodeGen::new();
        let word = make_word("test_while", 0, 0, vec![
            Expr::While {
                cond: vec![Expr::Literal(Literal::Bool(false))],
                body: vec![],
                span: Span::default(),
            },
        ]);
        let program = Program { words: vec![word] };
        let wasm = cg.generate(&program).unwrap();
        assert!(!wasm.is_empty());
    }

    #[test]
    fn test_times_loop() {
        let mut cg = CodeGen::new();
        let word = make_word("test_times", 0, 0, vec![
            int_lit(5),
            Expr::Times {
                body: vec![],
                span: Span::default(),
            },
        ]);
        let program = Program { words: vec![word] };
        let wasm = cg.generate(&program).unwrap();
        assert!(!wasm.is_empty());
    }

    #[test]
    fn test_word_call() {
        let mut cg = CodeGen::new();
        let helper = make_word("helper", 0, 1, vec![int_lit(99)]);
        let caller = make_word("caller", 0, 1, vec![word_call("helper")]);
        let program = Program { words: vec![helper, caller] };
        let wasm = cg.generate(&program).unwrap();
        assert!(!wasm.is_empty());
    }

    #[test]
    fn test_main_exported() {
        let mut cg = CodeGen::new();
        let main = make_word("main", 0, 0, vec![]);
        let program = Program { words: vec![main] };
        let wasm = cg.generate(&program).unwrap();
        assert!(!wasm.is_empty());
        // Check that wasm contains "main" export
        let wasm_str = String::from_utf8_lossy(&wasm);
        assert!(wasm_str.contains("main"));
    }

    #[test]
    fn test_memory_exported() {
        let mut cg = CodeGen::new();
        let program = Program::new();
        let wasm = cg.generate(&program).unwrap();
        let wasm_str = String::from_utf8_lossy(&wasm);
        assert!(wasm_str.contains("memory"));
    }

    #[test]
    fn test_float_literal() {
        let mut cg = CodeGen::new();
        let word = make_word("foo", 0, 1, vec![
            Expr::Literal(Literal::Float(3.14)),
        ]);
        let program = Program { words: vec![word] };
        let wasm = cg.generate(&program).unwrap();
        assert!(!wasm.is_empty());
    }

    #[test]
    fn test_bool_literal() {
        let mut cg = CodeGen::new();
        let word = make_word("foo", 0, 1, vec![
            Expr::Literal(Literal::Bool(true)),
        ]);
        let program = Program { words: vec![word] };
        let wasm = cg.generate(&program).unwrap();
        assert!(!wasm.is_empty());
    }

    #[test]
    fn test_string_literal() {
        let mut cg = CodeGen::new();
        let word = make_word("foo", 0, 1, vec![
            Expr::Literal(Literal::String("hello".to_string())),
        ]);
        let program = Program { words: vec![word] };
        let wasm = cg.generate(&program).unwrap();
        assert!(!wasm.is_empty());
    }

    #[test]
    fn test_logic_ops() {
        let mut cg = CodeGen::new();
        let word = make_word("logic", 2, 1, vec![
            word_call("and"),
        ]);
        let program = Program { words: vec![word] };
        let wasm = cg.generate(&program).unwrap();
        assert!(!wasm.is_empty());
    }

    #[test]
    fn test_bitwise_ops() {
        let mut cg = CodeGen::new();
        let word = make_word("bits", 2, 1, vec![
            word_call("bxor"),
        ]);
        let program = Program { words: vec![word] };
        let wasm = cg.generate(&program).unwrap();
        assert!(!wasm.is_empty());
    }

    #[test]
    fn test_wasm_size_reasonable() {
        let mut cg = CodeGen::new();
        let main = make_word("main", 0, 0, vec![
            int_lit(42),
            word_call("print"),
        ]);
        let program = Program { words: vec![main] };
        let wasm = cg.generate(&program).unwrap();
        // Should be well under 1KB for such a simple program
        assert!(wasm.len() < 1024);
    }
}
