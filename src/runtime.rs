use wasm_encoder::{
    ExportKind, ExportSection, GlobalSection, GlobalType, MemorySection,
    MemoryType, Module, ValType,
};

/// Generate minimal Obsidian runtime WASM.
///
/// The runtime provides:
/// - Memory export (1 page = 64KB)
/// - Stack pointer global
/// - Basic heap management setup
pub fn generate_runtime() -> Vec<u8> {
    let mut module = Module::new();

    // Memory section: 1 page minimum, 16 pages max
    let mut memory = MemorySection::new();
    memory.memory(MemoryType {
        minimum: 1,
        maximum: Some(16),
        memory64: false,
        shared: false,
        page_size_log2: None,
    });
    module.section(&memory);

    // Globals section: stack pointer
    let mut globals = GlobalSection::new();
    
    // Stack pointer starts at 0x400 (1024), grows up
    // Memory layout:
    //   0x0000 - 0x03FF: Runtime data (global vars, etc.)
    //   0x0400 - 0x0FFF: Stack (grows up)
    //   0x1000+:         Heap (user allocations)
    globals.global(
        GlobalType {
            val_type: ValType::I32,
            mutable: true,
            shared: false,
        },
        &wasm_encoder::ConstExpr::i32_const(0x400),
    );
    
    // Heap pointer starts at 0x1000 (4096)
    globals.global(
        GlobalType {
            val_type: ValType::I32,
            mutable: true,
            shared: false,
        },
        &wasm_encoder::ConstExpr::i32_const(0x1000),
    );
    module.section(&globals);

    // Export section
    let mut exports = ExportSection::new();
    exports.export("memory", ExportKind::Memory, 0);
    exports.export("__stack_ptr", ExportKind::Global, 0);
    exports.export("__heap_ptr", ExportKind::Global, 1);
    module.section(&exports);

    module.finish()
}

/// Size target for runtime (in bytes).
pub const RUNTIME_SIZE_LIMIT: usize = 300;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_runtime_generates() {
        let wasm = generate_runtime();
        assert!(!wasm.is_empty());
    }

    #[test]
    fn test_runtime_valid_magic() {
        let wasm = generate_runtime();
        assert_eq!(&wasm[0..4], b"\x00asm");
    }

    #[test]
    fn test_runtime_size_under_limit() {
        let wasm = generate_runtime();
        assert!(
            wasm.len() < RUNTIME_SIZE_LIMIT,
            "runtime size {} exceeds limit {}",
            wasm.len(),
            RUNTIME_SIZE_LIMIT
        );
    }

    #[test]
    fn test_runtime_exports_memory() {
        let wasm = generate_runtime();
        let wasm_str = String::from_utf8_lossy(&wasm);
        assert!(wasm_str.contains("memory"));
    }

    #[test]
    fn test_runtime_exports_stack_ptr() {
        let wasm = generate_runtime();
        let wasm_str = String::from_utf8_lossy(&wasm);
        assert!(wasm_str.contains("__stack_ptr"));
    }
}
