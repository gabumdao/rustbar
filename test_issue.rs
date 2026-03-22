use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    // We'll write exactly what the algorithm does.
    let toml_str = r#"
    include = ["colors.toml"]
    
    color = "$color1"
    "#;
    
    let colors_str = r#"
    [variables]
    color1 = "#C0A57E"
    "#;
    
    // Actually let's just create files and run the real code
    fs::write("colors.toml", colors_str).unwrap();
    fs::write("main.toml", toml_str).unwrap();
}
