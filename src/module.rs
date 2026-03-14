//! Module loader for Obsidian.
//!
//! Handles file resolution, parsing of imported files, and circular import detection.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use crate::ast::{Import, Program, WordDef};
use crate::lexer::Lexer;
use crate::parser::Parser;

/// Error during module loading.
#[derive(Debug)]
pub enum ModuleError {
    /// File not found.
    NotFound(PathBuf),
    /// Circular import detected.
    CircularImport(Vec<PathBuf>),
    /// Lexer error.
    LexError(String),
    /// Parser error.
    ParseError(String),
    /// IO error.
    IoError(std::io::Error),
}

impl std::fmt::Display for ModuleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ModuleError::NotFound(path) => write!(f, "module not found: {}", path.display()),
            ModuleError::CircularImport(chain) => {
                write!(f, "circular import detected: ")?;
                for (i, path) in chain.iter().enumerate() {
                    if i > 0 {
                        write!(f, " -> ")?;
                    }
                    write!(f, "{}", path.display())?;
                }
                Ok(())
            }
            ModuleError::LexError(msg) => write!(f, "lexer error: {}", msg),
            ModuleError::ParseError(msg) => write!(f, "parser error: {}", msg),
            ModuleError::IoError(e) => write!(f, "IO error: {}", e),
        }
    }
}

impl std::error::Error for ModuleError {}

impl From<std::io::Error> for ModuleError {
    fn from(e: std::io::Error) -> Self {
        ModuleError::IoError(e)
    }
}

/// A loaded module with its contents.
#[derive(Debug, Clone)]
pub struct Module {
    /// Canonical path to the module file.
    pub path: PathBuf,
    /// Words defined in this module.
    pub words: Vec<WordDef>,
    /// Modules imported by this module (paths).
    pub imports: Vec<PathBuf>,
}

/// Module loader with caching and circular import detection.
pub struct ModuleLoader {
    /// Base directory for resolving relative imports.
    base_dir: PathBuf,
    /// Standard library directory (optional).
    std_dir: Option<PathBuf>,
    /// Cache of loaded modules by canonical path.
    cache: HashMap<PathBuf, Module>,
    /// Currently loading modules (for cycle detection).
    loading: HashSet<PathBuf>,
    /// Import chain for error reporting.
    import_chain: Vec<PathBuf>,
}

impl ModuleLoader {
    /// Create a new module loader.
    pub fn new(base_dir: impl AsRef<Path>) -> Self {
        Self {
            base_dir: base_dir.as_ref().to_path_buf(),
            std_dir: None,
            cache: HashMap::new(),
            loading: HashSet::new(),
            import_chain: Vec::new(),
        }
    }

    /// Set the standard library directory.
    pub fn with_std_dir(mut self, std_dir: impl AsRef<Path>) -> Self {
        self.std_dir = Some(std_dir.as_ref().to_path_buf());
        self
    }

    /// Resolve an import path to a canonical file path.
    fn resolve_path(&self, import_path: &str, from_dir: &Path) -> Option<PathBuf> {
        // Check if it's a std/ import
        if import_path.starts_with("std/") {
            if let Some(std_dir) = &self.std_dir {
                let path = std_dir.join(&import_path[4..]);
                let with_ext = if path.extension().is_none() {
                    path.with_extension("obs")
                } else {
                    path
                };
                if with_ext.exists() {
                    return with_ext.canonicalize().ok();
                }
            }
        }

        // Try relative to the importing file
        let relative = from_dir.join(import_path);
        let with_ext = if relative.extension().is_none() {
            relative.with_extension("obs")
        } else {
            relative
        };
        if with_ext.exists() {
            return with_ext.canonicalize().ok();
        }

        // Try relative to base directory
        let from_base = self.base_dir.join(import_path);
        let with_ext = if from_base.extension().is_none() {
            from_base.with_extension("obs")
        } else {
            from_base
        };
        if with_ext.exists() {
            return with_ext.canonicalize().ok();
        }

        None
    }

    /// Load a module from a file path.
    pub fn load(&mut self, path: impl AsRef<Path>) -> Result<&Module, ModuleError> {
        let path = path.as_ref();
        let canonical = path.canonicalize()?;

        // Return cached if already loaded
        if self.cache.contains_key(&canonical) {
            return Ok(self.cache.get(&canonical).unwrap());
        }

        // Check for circular import
        if self.loading.contains(&canonical) {
            self.import_chain.push(canonical.clone());
            return Err(ModuleError::CircularImport(self.import_chain.clone()));
        }

        // Mark as loading
        self.loading.insert(canonical.clone());
        self.import_chain.push(canonical.clone());

        // Read and parse the file
        let source = fs::read_to_string(&canonical)?;
        let module = self.parse_module(&canonical, &source)?;

        // Load all imports
        for import in &module.imports {
            self.load(import)?;
        }

        // Remove from loading and chain
        self.loading.remove(&canonical);
        self.import_chain.pop();

        // Cache and return
        self.cache.insert(canonical.clone(), module);
        Ok(self.cache.get(&canonical).unwrap())
    }

    /// Parse a module from source.
    fn parse_module(&self, path: &Path, source: &str) -> Result<Module, ModuleError> {
        let mut lexer = Lexer::new(source);
        let tokens = lexer
            .tokenize()
            .map_err(|e| ModuleError::LexError(e.to_string()))?;

        let mut parser = Parser::new(tokens);
        let program = parser
            .parse()
            .map_err(|e| ModuleError::ParseError(e.to_string()))?;

        let parent_dir = path.parent().unwrap_or(&self.base_dir);
        
        // Resolve import paths
        let mut imports = Vec::new();
        for import in &program.imports {
            match self.resolve_path(&import.path, parent_dir) {
                Some(resolved) => imports.push(resolved),
                None => return Err(ModuleError::NotFound(PathBuf::from(&import.path))),
            }
        }

        Ok(Module {
            path: path.to_path_buf(),
            words: program.words,
            imports,
        })
    }

    /// Get all loaded modules.
    pub fn modules(&self) -> impl Iterator<Item = &Module> {
        self.cache.values()
    }

    /// Get all words from all loaded modules.
    pub fn all_words(&self) -> Vec<&WordDef> {
        self.cache.values().flat_map(|m| &m.words).collect()
    }

    /// Merge all loaded modules into a single program.
    pub fn merge_into_program(&self) -> Program {
        let mut program = Program::new();
        for module in self.cache.values() {
            program.words.extend(module.words.clone());
        }
        program
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::TempDir;

    fn create_test_file(dir: &Path, name: &str, content: &str) -> PathBuf {
        let path = dir.join(name);
        let mut file = File::create(&path).unwrap();
        write!(file, "{}", content).unwrap();
        path
    }

    #[test]
    fn test_load_simple_module() {
        let tmp = TempDir::new().unwrap();
        let path = create_test_file(
            tmp.path(),
            "simple.obs",
            "def double (n -- n) dup + end",
        );

        let mut loader = ModuleLoader::new(tmp.path());
        let module = loader.load(&path).unwrap();

        assert_eq!(module.words.len(), 1);
        assert_eq!(module.words[0].name, "double");
    }

    #[test]
    fn test_load_with_import() {
        let tmp = TempDir::new().unwrap();
        
        // Create math.obs
        create_test_file(
            tmp.path(),
            "math.obs",
            "def square (n -- n) dup * end",
        );
        
        // Create main.obs that imports math
        let main_path = create_test_file(
            tmp.path(),
            "main.obs",
            r#"import "math.obs"
def cube (n -- n) dup dup * * end"#,
        );

        let mut loader = ModuleLoader::new(tmp.path());
        loader.load(&main_path).unwrap();

        // Should have loaded both modules
        assert_eq!(loader.cache.len(), 2);
        
        // Merge should have both words
        let program = loader.merge_into_program();
        assert_eq!(program.words.len(), 2);
    }

    #[test]
    fn test_circular_import_detection() {
        let tmp = TempDir::new().unwrap();
        
        // a.obs imports b.obs
        create_test_file(tmp.path(), "a.obs", r#"import "b.obs""#);
        
        // b.obs imports a.obs (circular!)
        create_test_file(tmp.path(), "b.obs", r#"import "a.obs""#);
        
        let mut loader = ModuleLoader::new(tmp.path());
        let result = loader.load(tmp.path().join("a.obs"));
        
        assert!(matches!(result, Err(ModuleError::CircularImport(_))));
    }

    #[test]
    fn test_import_not_found() {
        let tmp = TempDir::new().unwrap();
        
        let path = create_test_file(
            tmp.path(),
            "main.obs",
            r#"import "nonexistent.obs""#,
        );

        let mut loader = ModuleLoader::new(tmp.path());
        let result = loader.load(&path);
        
        assert!(matches!(result, Err(ModuleError::NotFound(_))));
    }

    #[test]
    fn test_resolve_path_with_extension() {
        let tmp = TempDir::new().unwrap();
        create_test_file(tmp.path(), "lib.obs", "");
        
        let loader = ModuleLoader::new(tmp.path());
        
        // Should resolve with explicit extension
        let resolved = loader.resolve_path("lib.obs", tmp.path());
        assert!(resolved.is_some());
        
        // Should resolve without extension (adds .obs)
        let resolved = loader.resolve_path("lib", tmp.path());
        assert!(resolved.is_some());
    }

    #[test]
    fn test_module_caching() {
        let tmp = TempDir::new().unwrap();
        let path = create_test_file(
            tmp.path(),
            "cached.obs",
            "def foo (--) end",
        );

        let mut loader = ModuleLoader::new(tmp.path());
        
        // Load twice
        loader.load(&path).unwrap();
        loader.load(&path).unwrap();
        
        // Should only be cached once
        assert_eq!(loader.cache.len(), 1);
    }
}
