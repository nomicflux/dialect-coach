use std::fs;
use std::path::Path;

// Test Unicode strings that commonly cause issues
const PROBLEMATIC_STRINGS: &[&str] = &[
    "مرحبا بك في المغرب! هذا نص باللغة العربية.", // Arabic
    "¡Hola! ¿Cómo estás? Niño, niña, año, señor.", // Spanish with accents
    "🚀 Émojis são problemáticos também! 🌟🎉", // Emojis + Portuguese
    "Iñtërnâtiônàlizætiøn is tricky 测试中文", // Mixed scripts
];

fn main() {
    println!("Testing Unicode safety...");
    
    // Create test directory
    let test_dir = Path::new("test_unicode");
    if test_dir.exists() {
        fs::remove_dir_all(test_dir).expect("Failed to clean test directory");
    }
    fs::create_dir_all(test_dir).expect("Failed to create test directory");
    
    // Create test files with problematic Unicode content
    for (i, content) in PROBLEMATIC_STRINGS.iter().enumerate() {
        let file_path = test_dir.join(format!("test_{}.txt", i));
        fs::write(&file_path, content).expect("Failed to write test file");
        println!("Created test file: {} with content: {}", file_path.display(), content);
    }
    
    println!("Test files created in ./test_unicode/");
    println!("Now run: cargo run -p corpus-processor -- process --input test_unicode --output test_output --dialect egyptian-arabic");
}