use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Deserialize)]
pub struct RuntimeConfigs {
    pub mathematical_functions: MathematicalFunctionsConfig,
    pub numerical_methods: NumericalMethodsConfig,
    pub visualization_defaults: VisualizationDefaultsConfig,
}

impl RuntimeConfigs {
    pub fn load_default() -> Self {
        Self::load_from_dir(Path::new("config")).unwrap_or_else(|_| Self::default())
    }

    pub fn load_from_dir(dir: &Path) -> Result<Self, String> {
        let mf = read_toml::<MathematicalFunctionsConfig>(
            &dir.join("mathematical_functions.toml"),
            "mathematical_functions.toml",
        )?;
        let nm = read_toml::<NumericalMethodsConfig>(
            &dir.join("numerical_methods.toml"),
            "numerical_methods.toml",
        )?;
        let vd = read_toml::<VisualizationDefaultsConfig>(
            &dir.join("visualization_defaults.toml"),
            "visualization_defaults.toml",
        )?;

        Ok(Self {
            mathematical_functions: mf,
            numerical_methods: nm,
            visualization_defaults: vd,
        })
    }
}

impl Default for RuntimeConfigs {
    fn default() -> Self {
        Self {
            mathematical_functions: MathematicalFunctionsConfig::default(),
            numerical_methods: NumericalMethodsConfig::default(),
            visualization_defaults: VisualizationDefaultsConfig::default(),
        }
    }
}

fn read_toml<T: for<'de> Deserialize<'de>>(path: &PathBuf, label: &str) -> Result<T, String> {
    let raw = fs::read_to_string(path).map_err(|e| format!("failed to read {}: {}", label, e))?;
    toml::from_str::<T>(&raw).map_err(|e| format!("failed to parse {}: {}", label, e))
}

#[derive(Debug, Clone, Deserialize)]
pub struct MathematicalFunctionsConfig {
    #[serde(default)]
    pub constants: HashMap<String, f64>,
}

impl Default for MathematicalFunctionsConfig {
    fn default() -> Self {
        let mut constants = HashMap::new();
        constants.insert("pi".to_string(), std::f64::consts::PI);
        constants.insert("e".to_string(), std::f64::consts::E);
        Self { constants }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct NumericalMethodsConfig {
    #[serde(default)]
    pub ode_solving: OdeSolvingConfig,
}

impl Default for NumericalMethodsConfig {
    fn default() -> Self {
        Self {
            ode_solving: OdeSolvingConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct OdeSolvingConfig {
    #[serde(default = "default_step")]
    pub default_step_size: f64,
}

impl Default for OdeSolvingConfig {
    fn default() -> Self {
        Self {
            default_step_size: default_step(),
        }
    }
}

fn default_step() -> f64 {
    0.01
}

#[derive(Debug, Clone, Deserialize)]
pub struct VisualizationDefaultsConfig {
    #[serde(default)]
    pub plotting: PlottingConfig,
}

impl Default for VisualizationDefaultsConfig {
    fn default() -> Self {
        Self {
            plotting: PlottingConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct PlottingConfig {
    #[serde(default = "default_plot_resolution_2d")]
    pub default_resolution_2d: usize,
    #[serde(default = "default_plot_resolution_3d")]
    pub default_resolution_3d: [usize; 2],
}

impl Default for PlottingConfig {
    fn default() -> Self {
        Self {
            default_resolution_2d: default_plot_resolution_2d(),
            default_resolution_3d: default_plot_resolution_3d(),
        }
    }
}

fn default_plot_resolution_2d() -> usize {
    128
}

fn default_plot_resolution_3d() -> [usize; 2] {
    [32, 32]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_workspace_configs() {
        let loaded = RuntimeConfigs::load_default();
        assert!(loaded.visualization_defaults.plotting.default_resolution_2d > 0);
    }
}
