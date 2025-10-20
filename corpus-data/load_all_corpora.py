#!/usr/bin/env python3
"""
Load all available dialect corpora using the existing corpus processor
"""

import os
import subprocess
import sys
from pathlib import Path

import json

# Load dialect inventory from persistent storage
def load_dialect_inventory():
    """Load the authoritative dialect inventory from JSON file"""
    # Inventory is at project root, script is in corpus-data/
    script_dir = Path(__file__).resolve().parent
    inventory_path = script_dir.parent / "DIALECT_INVENTORY.json"
    
    if not inventory_path.exists():
        print(f"❌ Dialect inventory not found at {inventory_path}")
        return {}
    
    with open(inventory_path, 'r') as f:
        inventory = json.load(f)
    
    # Extract only dialects with AVAILABLE status
    available_dialects = {}
    for dialect_id, dialect_info in inventory['configured_dialects'].items():
        if dialect_info['status'] == 'AVAILABLE' and dialect_info['corpus_file']:
            available_dialects[dialect_id] = dialect_info['corpus_file']
    
    return available_dialects

# Get mapping from the persistent inventory
CORPUS_MAPPING = load_dialect_inventory()

def check_existing_data():
    """Check what corpus files we actually have"""
    print("=== Checking Available Corpus Files ===")
    
    available_files = {}
    script_dir = Path(__file__).resolve().parent
    
    # Check dialect directories for raw files and processed files
    for dialect_dir in sorted(script_dir.iterdir()):
        if not dialect_dir.is_dir() or dialect_dir.name.startswith('.'):
            continue
            
        print(f"\nChecking {dialect_dir.name}:")
        
        # Check for raw .txt files
        for txt_file in dialect_dir.glob("*.txt"):
            size_mb = txt_file.stat().st_size / (1024 * 1024)
            available_files[txt_file.name] = f"{size_mb:.1f}MB"
            print(f"  ✅ Raw: {txt_file.name} ({size_mb:.1f}MB)")
        
        # Check for processed files in dialect_dir/processed/
        processed_dir = dialect_dir / "processed"
        if processed_dir.exists():
            for jsonl_file in processed_dir.glob("*.jsonl"):
                size_mb = jsonl_file.stat().st_size / (1024 * 1024)
                print(f"  📦 Processed: {jsonl_file.name} ({size_mb:.1f}MB)")
    
    return available_files

def get_dialect_mapping():
    """Create accurate mapping of available data to dialect IDs"""
    available_files = check_existing_data()
    
    dialect_mapping = {}
    
    # Map available files to dialect IDs
    for dialect_id, filename in CORPUS_MAPPING.items():
        if filename in available_files:
            dialect_mapping[dialect_id] = filename
            print(f"📋 {dialect_id} -> {filename}")
        else:
            print(f"❌ {dialect_id} -> {filename} (NOT FOUND)")
    
    print(f"\n✅ Ready to process {len(dialect_mapping)} dialects:")
    for dialect_id in dialect_mapping:
        print(f"  - {dialect_id}")
    
    return dialect_mapping

def process_corpus(dialect_id, input_file):
    """Process a single corpus file using the Rust corpus processor"""
    print(f"\n=== Processing {dialect_id} ({input_file}) ===")
    
    # Project root is parent of corpus-data/
    script_dir = Path(__file__).resolve().parent
    project_root = script_dir.parent
    
    # Find the dialect directory for this dialect_id
    dialect_dir = None
    for d in script_dir.iterdir():
        if d.is_dir() and d.name.replace('-', '_') == dialect_id:
            dialect_dir = d
            break
    
    if not dialect_dir:
        print(f"❌ Cannot find dialect directory for {dialect_id}")
        return False
    
    # Output goes to the dialect's processed directory
    output_dir = dialect_dir / "processed"
    output_dir.mkdir(exist_ok=True)
    
    # Input file path (should be in the dialect directory)
    input_path = dialect_dir / input_file
    if not input_path.exists():
        print(f"❌ Input file not found: {input_path}")
        return False
    
    # Determine language from dialect_id
    language = None
    if dialect_id.startswith("arabic_"):
        language = "arabic"
    elif dialect_id.startswith("spanish_"):
        language = "spanish" 
    elif dialect_id.startswith("french_"):
        language = "french"
    else:
        print(f"❌ Cannot determine language for dialect: {dialect_id}")
        return False
    
    # Run the corpus processor
    cmd = [
        "cargo", "run", "-p", "corpus-processor", "--",
        "process",
        "--language", language,
        "--dialect", dialect_id,
        "--input", str(input_path),
        "--output", str(output_dir)
    ]
    
    print(f"Running: {' '.join(cmd)}")
    
    try:
        # Change to the project root directory - stream output in real-time
        result = subprocess.run(
            cmd, 
            cwd=project_root,
            # Don't capture output - let it stream to terminal
            text=True,
            check=True
        )
        
        print(f"✅ Successfully processed {dialect_id}")
        return True
        
    except subprocess.CalledProcessError as e:
        print(f"❌ Failed to process {dialect_id}")
        print(f"Return code: {e.returncode}")
        print(f"STDOUT: {e.stdout}")
        print(f"STDERR: {e.stderr}")
        return False
    except Exception as e:
        print(f"❌ Error processing {dialect_id}: {e}")
        return False

def upload_to_qdrant(dialect_id):
    """Upload processed corpus to Qdrant"""
    print(f"\n=== Uploading {dialect_id} to Qdrant ===")
    
    script_dir = Path(__file__).resolve().parent
    project_root = script_dir.parent
    
    # Find the dialect directory and its processed file
    dialect_dir = None
    for d in script_dir.iterdir():
        if d.is_dir() and d.name.replace('-', '_') == dialect_id:
            dialect_dir = d
            break
    
    if not dialect_dir:
        print(f"❌ Cannot find dialect directory for {dialect_id}")
        return False
    
    # Look for any .jsonl file in the processed directory
    processed_dir = dialect_dir / "processed"
    if not processed_dir.exists():
        print(f"❌ No processed directory found: {processed_dir}")
        return False
    
    processed_files = list(processed_dir.glob("*.jsonl"))
    if not processed_files:
        print(f"❌ No processed files found in: {processed_dir}")
        return False
    
    # Use the first .jsonl file found
    processed_file = processed_files[0]
    
    cmd = [
        "cargo", "run", "-p", "corpus-processor", "--",
        "upload",
        "--input", str(processed_file)
    ]
    
    print(f"Running: {' '.join(cmd)}")
    
    try:
        result = subprocess.run(
            cmd,
            cwd=project_root,
            capture_output=True,
            text=True,
            check=True
        )
        
        print(f"✅ Successfully uploaded {dialect_id}")
        print(f"STDOUT: {result.stdout}")
        if result.stderr:
            print(f"STDERR: {result.stderr}")
        
        return True
        
    except subprocess.CalledProcessError as e:
        print(f"❌ Failed to upload {dialect_id}")
        print(f"Return code: {e.returncode}")
        print(f"STDOUT: {e.stdout}")
        print(f"STDERR: {e.stderr}")
        return False
    except Exception as e:
        print(f"❌ Error uploading {dialect_id}: {e}")
        return False

def main():
    """Process and upload all available dialect corpora"""
    print("🚀 Loading All Available Dialect Corpora")
    print("=" * 50)
    
    # Get accurate mapping of available data
    dialect_mapping = get_dialect_mapping()
    
    if not dialect_mapping:
        print("❌ No dialect corpora available to process!")
        return
    
    success_count = 0
    upload_count = 0
    
    print(f"\n📊 Processing {len(dialect_mapping)} dialect corpora...")
    
    for dialect_id, input_file in dialect_mapping.items():
        print(f"\n{'='*60}")
        print(f"Processing {dialect_id.upper().replace('_', ' ')}")
        print(f"{'='*60}")
        
        # Process the corpus
        if process_corpus(dialect_id, input_file):
            success_count += 1
            
            # Upload to Qdrant
            if upload_to_qdrant(dialect_id):
                upload_count += 1
        
    # Summary
    print(f"\n{'='*60}")
    print(f"📊 CORPUS LOADING SUMMARY")
    print(f"{'='*60}")
    print(f"✅ Successfully processed: {success_count}/{len(dialect_mapping)} dialects")
    print(f"🚀 Successfully uploaded: {upload_count}/{len(dialect_mapping)} dialects")
    
    if success_count == len(dialect_mapping):
        print("\n🎉 All available dialect corpora have been loaded!")
    else:
        failed_count = len(dialect_mapping) - success_count
        print(f"\n⚠️  {failed_count} dialects failed to process")
    
    # Show what's missing
    print(f"\n📋 DIALECT STATUS REPORT")
    print("=" * 30)
    
    all_configured_dialects = [
        "spanish_mexican", "spanish_castilian", "spanish_argentinian", 
        "spanish_cuban", "spanish_chilean", "spanish_colombian",
        "arabic_egyptian", "arabic_levantine", "arabic_gulf", 
        "arabic_maghrebi", "arabic_iraqi",
        "french_quebecois", "french_parisian", "french_swiss", 
        "french_belgian", "french_african"
    ]
    
    for dialect in all_configured_dialects:
        if dialect in dialect_mapping:
            print(f"✅ {dialect.replace('_', ' ').title()}: LOADED")
        else:
            print(f"❌ {dialect.replace('_', ' ').title()}: NO DATA")

if __name__ == "__main__":
    main()
