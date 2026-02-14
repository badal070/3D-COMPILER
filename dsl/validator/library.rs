/// Library compatibility validation pass.
/// Ensures imported libraries exist and are compatible.
/// Validates library versions and construct availability.
use crate::ast::*;
use crate::errors::{DslError, ErrorCode, ErrorCollector};
use std::collections::HashMap;
use std::path::PathBuf;

/// Represents a library that can be imported
#[derive(Debug, Clone)]
pub struct Library {
    pub name: String,
    pub version: String,
    pub provides_components: Vec<String>,
    pub provides_constraints: Vec<String>,
    pub provides_motions: Vec<String>,
}

pub struct LibraryValidator {
    file: PathBuf,
    errors: ErrorCollector,
    available_libraries: HashMap<String, Library>,
}

impl LibraryValidator {
    pub fn new(file: PathBuf) -> Self {
        let mut validator = Self {
            file,
            errors: ErrorCollector::new(),
            available_libraries: HashMap::new(),
        };

        validator.init_default_libraries();
        validator
    }

    pub fn validate(mut self, ast: &AstFile) -> Result<(), Vec<DslError>> {
        // Build map of imported libraries (owned clones so we don't borrow `self`)
        let imported_libraries = self.build_import_map(&ast.library_imports);

        // Validate all imports exist
        self.validate_imports(&ast.library_imports);

        // Validate constructs are provided by imported libraries
        self.validate_construct_availability(
            &ast.entities,
            &ast.constraints,
            &ast.motions,
            &imported_libraries,
        );

        self.errors.into_result(())
    }

    fn init_default_libraries(&mut self) {
        // Core mechanics library
        self.register_library(Library {
            name: "core_mechanics".to_string(),
            version: "1.0.0".to_string(),
            provides_components: vec!["transform".to_string(), "physical".to_string()],
            provides_constraints: vec!["fixed_joint".to_string(), "hinge_joint".to_string()],
            provides_motions: vec!["rotation".to_string(), "translation".to_string()],
        });

        // Basic solids library
        self.register_library(Library {
            name: "basic_solids".to_string(),
            version: "1.0.0".to_string(),
            provides_components: vec!["geometry".to_string()],
            provides_constraints: vec![],
            provides_motions: vec![],
        });

        // Gear systems library
        self.register_library(Library {
            name: "gear_systems".to_string(),
            version: "1.0.0".to_string(),
            provides_components: vec![],
            provides_constraints: vec!["gear_relation".to_string(), "belt_drive".to_string()],
            provides_motions: vec![],
        });

        // Advanced physics library
        self.register_library(Library {
            name: "advanced_physics".to_string(),
            version: "1.0.0".to_string(),
            provides_components: vec!["collision".to_string(), "material".to_string()],
            provides_constraints: vec!["spring".to_string(), "damper".to_string()],
            provides_motions: vec!["oscillation".to_string()],
        });
    }

    fn register_library(&mut self, library: Library) {
        self.available_libraries
            .insert(library.name.clone(), library);
    }

    fn build_import_map(&self, imports: &AstLibraryImports) -> HashMap<String, Library> {
        let mut map = HashMap::new();

        for import in &imports.imports {
            if let Some(library) = self.available_libraries.get(&import.library_name) {
                map.insert(import.alias.clone(), library.clone());
            }
        }

        map
    }

    fn validate_imports(&mut self, imports: &AstLibraryImports) {
        for import in &imports.imports {
            if !self.available_libraries.contains_key(&import.library_name) {
                self.errors.add(
                    DslError::new(
                        ErrorCode::UndefinedLibrary,
                        format!("Unknown library: '{}'", import.library_name),
                        import.span,
                        self.file.clone(),
                    )
                    .with_help(format!(
                        "Available libraries: {}",
                        self.available_libraries
                            .keys()
                            .map(|s| s.as_str())
                            .collect::<Vec<_>>()
                            .join(", ")
                    )),
                );
            }
        }
    }

    fn validate_construct_availability(
        &mut self,
        entities: &[AstEntity],
        constraints: &[AstConstraint],
        motions: &[AstMotion],
        imported_libraries: &HashMap<String, Library>,
    ) {
        // Collect all available constructs from imported libraries
        let mut available_components = Vec::new();
        let mut available_constraints = Vec::new();
        let mut available_motions = Vec::new();

        for library in imported_libraries.values() {
            available_components.extend(library.provides_components.iter().map(|s| s.as_str()));
            available_constraints.extend(library.provides_constraints.iter().map(|s| s.as_str()));
            available_motions.extend(library.provides_motions.iter().map(|s| s.as_str()));
        }

        // Validate entities use available components
        for entity in entities {
            for component in &entity.components {
                if !available_components.contains(&component.name.as_str()) {
                    self.errors.add(
                        DslError::new(
                            ErrorCode::UnknownLibraryConstruct,
                            format!(
                                "Component '{}' is not provided by any imported library",
                                component.name
                            ),
                            component.span,
                            self.file.clone(),
                        )
                        .with_help(self.suggest_library_for_component(&component.name)),
                    );
                }
            }
        }

        // Validate constraints use available types
        for constraint in constraints {
            if let Some(constraint_type) = constraint.constraint_type() {
                if !available_constraints.contains(&constraint_type) {
                    let type_field = constraint.get_field("type").unwrap();
                    self.errors.add(
                        DslError::new(
                            ErrorCode::UnknownLibraryConstruct,
                            format!(
                                "Constraint type '{}' is not provided by any imported library",
                                constraint_type
                            ),
                            type_field.span,
                            self.file.clone(),
                        )
                        .with_help(self.suggest_library_for_constraint(constraint_type)),
                    );
                }
            }
        }

        // Validate motions use available types
        for motion in motions {
            if let Some(motion_type) = motion.motion_type() {
                if !available_motions.contains(&motion_type) {
                    let type_field = motion.get_field("type").unwrap();
                    self.errors.add(
                        DslError::new(
                            ErrorCode::UnknownLibraryConstruct,
                            format!(
                                "Motion type '{}' is not provided by any imported library",
                                motion_type
                            ),
                            type_field.span,
                            self.file.clone(),
                        )
                        .with_help(self.suggest_library_for_motion(motion_type)),
                    );
                }
            }
        }
    }

    fn suggest_library_for_component(&self, component_name: &str) -> String {
        for (lib_name, library) in &self.available_libraries {
            if library
                .provides_components
                .iter()
                .any(|c| c == component_name)
            {
                return format!(
                    "Import library '{}' to use component '{}'",
                    lib_name, component_name
                );
            }
        }
        format!(
            "No available library provides component '{}'",
            component_name
        )
    }

    fn suggest_library_for_constraint(&self, constraint_type: &str) -> String {
        for (lib_name, library) in &self.available_libraries {
            if library
                .provides_constraints
                .iter()
                .any(|c| c == constraint_type)
            {
                return format!(
                    "Import library '{}' to use constraint type '{}'",
                    lib_name, constraint_type
                );
            }
        }
        format!(
            "No available library provides constraint type '{}'",
            constraint_type
        )
    }

    fn suggest_library_for_motion(&self, motion_type: &str) -> String {
        for (lib_name, library) in &self.available_libraries {
            if library.provides_motions.iter().any(|m| m == motion_type) {
                return format!(
                    "Import library '{}' to use motion type '{}'",
                    lib_name, motion_type
                );
            }
        }
        format!(
            "No available library provides motion type '{}'",
            motion_type
        )
    }

    /// Add a custom library at runtime (for extensibility)
    pub fn add_library(&mut self, library: Library) {
        self.available_libraries
            .insert(library.name.clone(), library);
    }

    /// Get available libraries
    pub fn available_libraries(&self) -> &HashMap<String, Library> {
        &self.available_libraries
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_libraries() {
        let validator = LibraryValidator::new(PathBuf::from("test.dsl"));

        assert!(validator.available_libraries.contains_key("core_mechanics"));
        assert!(validator.available_libraries.contains_key("basic_solids"));
        assert!(validator.available_libraries.contains_key("gear_systems"));
        assert!(validator
            .available_libraries
            .contains_key("advanced_physics"));
    }

    #[test]
    fn test_library_provides() {
        let validator = LibraryValidator::new(PathBuf::from("test.dsl"));

        let core_lib = validator.available_libraries.get("core_mechanics").unwrap();
        assert!(core_lib
            .provides_components
            .contains(&"transform".to_string()));
        assert!(core_lib.provides_motions.contains(&"rotation".to_string()));

        let gear_lib = validator.available_libraries.get("gear_systems").unwrap();
        assert!(gear_lib
            .provides_constraints
            .contains(&"gear_relation".to_string()));
    }

    #[test]
    fn test_add_custom_library() {
        let mut validator = LibraryValidator::new(PathBuf::from("test.dsl"));

        let custom_lib = Library {
            name: "custom_mechanics".to_string(),
            version: "0.1.0".to_string(),
            provides_components: vec!["custom_component".to_string()],
            provides_constraints: vec![],
            provides_motions: vec![],
        };

        validator.add_library(custom_lib);
        assert!(validator
            .available_libraries
            .contains_key("custom_mechanics"));
    }
}
