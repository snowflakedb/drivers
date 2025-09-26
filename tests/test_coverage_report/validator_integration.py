#!/usr/bin/env python3
"""
Validator Integration Module

Handles integration with the Rust tests_format_validator, including
running the validator, parsing output, and managing Breaking Change data.
"""

import json
import subprocess
from pathlib import Path
from typing import Dict, Optional
import os


class ValidatorIntegration:
    """Manages integration with the Rust tests format validator."""
    
    def __init__(self, workspace_root: Path):
        self.workspace_root = workspace_root
        self.validator_path = workspace_root / "tests" / "tests_format_validator"
        self._validator_cache = None
        self._breaking_change_cache = None
    
    def get_validator_data(self) -> Dict:
        """Get complete validator data including features and Breaking Change information."""
        if self._validator_cache is not None:
            return self._validator_cache
        
        try:
            validator_binary = self.validator_path / "target" / "release" / "tests_format_validator"
            if not validator_binary.exists():
                print("Validator binary not found, building...")
                self._build_validator()
            elif self._needs_rebuild(validator_binary):
                print("Validator source code changed, rebuilding...")
                # Clear cache before rebuilding since results will be different
                self._validator_cache = None
                self._breaking_change_cache = None
                self._build_validator()
            
            # Run validator with JSON output
            result = subprocess.run([
                str(validator_binary.resolve()),
                "--workspace", str(self.workspace_root),
                "--features", str(self.workspace_root / "tests" / "definitions"),
                "--json"
            ], capture_output=True, text=True)
            
            if result.returncode not in [0, 1]:  # 1 is expected when orphaned tests are found
                print(f"Validator failed with return code {result.returncode}")
                print(f"stderr: {result.stderr}")
                print(f"stdout: {result.stdout}")
                return {}
            
            # Parse JSON output
            validator_data = json.loads(result.stdout)
            self._validator_cache = validator_data
            return validator_data
            
        except Exception as e:
            print(f"Error running validator: {e}")
            return {}
    
    def get_breaking_change_data(self) -> Dict:
        """Get Breaking Change data from the validator's JSON output."""
        if self._breaking_change_cache is not None:
            return self._breaking_change_cache
        
        validator_data = self.get_validator_data()
        if validator_data and 'breaking_changes_report' in validator_data:
            self._breaking_change_cache = validator_data['breaking_changes_report']
            return self._breaking_change_cache
        
        return {}
    
    def get_features_data(self) -> Dict:
        """Get features data from validator output."""
        validator_data = self.get_validator_data()
        if not validator_data:
            return {}
        
        return self._parse_validator_output(validator_data.get('validation_results', {}))
    
    def _build_validator(self):
        """Build the Rust validator if it doesn't exist."""
        print("Building Rust validator...")
        # Use release build for better performance and parallel compilation
        build_result = subprocess.run(
            ["cargo", "build", "--release", "-j", "4"],
            cwd=self.validator_path,
            capture_output=True,
            text=True
        )
        if build_result.returncode != 0:
            raise RuntimeError(f"Failed to build validator: {build_result.stderr}")
    
    def _needs_rebuild(self, validator_binary: Path) -> bool:
        """Check if the validator binary needs to be rebuilt based on source file changes."""
        if not validator_binary.exists():
            return True
        
        # Get binary modification time
        binary_mtime = validator_binary.stat().st_mtime
        
        # Check key source files that would require a rebuild
        source_files_to_check = [
            self.validator_path / "Cargo.toml",
            self.validator_path / "Cargo.lock",
            self.validator_path / "src",
        ]
        
        for source_path in source_files_to_check:
            if source_path.exists():
                if source_path.is_file():
                    # Single file - check its modification time
                    if source_path.stat().st_mtime > binary_mtime:
                        return True
                elif source_path.is_dir():
                    # Directory - check all Rust files recursively
                    for rust_file in source_path.rglob("*.rs"):
                        if rust_file.stat().st_mtime > binary_mtime:
                            return True
        
        return False
    
    def _parse_validator_output(self, validation_results: Dict) -> Dict:
        """Parse validator output into features dictionary."""
        features = {}
        
        for language_name, language_validation in validation_results.items():
            if not hasattr(language_validation, 'get'):
                continue
                
            for feature_name, feature_validation in language_validation.get('features', {}).items():
                if feature_name not in features:
                    features[feature_name] = {
                        'path': feature_validation.get('path', ''),
                        'languages': {}
                    }
                
                # Determine implementation status
                implemented = feature_validation.get('implemented', False)
                status = '✅' if implemented else '❌'
                
                features[feature_name]['languages'][language_name] = {
                    'status': status,
                    'implemented': implemented,
                    'methods': feature_validation.get('methods', {}),
                    'scenarios': feature_validation.get('scenarios', [])
                }
        
        return features
