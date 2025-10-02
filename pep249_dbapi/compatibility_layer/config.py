
import os
from pathlib import Path


class Config:
    
    def __init__(self):
        self.project_root = Path(__file__).parent.absolute()
        self.universal_driver_root = self._find_universal_driver()
        self.pep249_path = self.universal_driver_root / "pep249_dbapi"
        self.core_lib_path = self._find_core_library()
        self.old_driver_root = self._find_old_driver()
        
    def _find_universal_driver(self) -> Path:
        candidates = [
            self.project_root.parent.parent,  # We're now in universal-driver/pep249_dbapi/compatibility_layer
            self.project_root.parent.parent.parent / "universal-driver",
            Path.cwd() / "universal-driver",
        ]
        
        for candidate in candidates:
            if (candidate / "pep249_dbapi").exists():
                return candidate
                
        raise FileNotFoundError(
            "Universal driver not found. Expected to find 'universal-driver/pep249_dbapi' directory. "
            f"Searched: {[str(c) for c in candidates]}"
        )
    
    def _find_core_library(self) -> Path:
        target_dir = self.universal_driver_root / "target"
        
        for build_type in ["debug", "release"]:
            for lib_name in ["libsf_core.dylib", "libsf_core.so", "sf_core.dll"]:
                lib_path = target_dir / build_type / lib_name
                if lib_path.exists():
                    return lib_path
        
        raise FileNotFoundError(
            f"Core library not found in {target_dir}. "
            "Make sure the universal driver is built (cargo build)."
        )
    
    def _find_old_driver(self) -> Path:
        candidates = [
            self.project_root.parent.parent.parent / "snowflake-connector-python",  # Go up to repo root
            self.project_root.parent.parent / "snowflake-connector-python",
            self.project_root / "snowflake-connector-python", 
            Path.cwd() / "snowflake-connector-python",
        ]
        
        for candidate in candidates:
            if (candidate / "test").exists():
                return candidate
                
        raise FileNotFoundError(
            "Old driver not found. Expected to find 'snowflake-connector-python/test' directory. "
            f"Searched: {[str(c) for c in candidates]}"
        )
    
    def validate(self):
        checks = [
            (self.pep249_path, "PEP249 DBAPI path"),
            (self.core_lib_path, "Core library"),
            (self.old_driver_root / "test", "Old driver test directory"),
        ]
        
        missing = []
        for path, name in checks:
            if not path.exists():
                missing.append(f"{name}: {path}")
        
        if missing:
            raise FileNotFoundError(f"Missing required paths:\n" + "\n".join(f"  - {m}" for m in missing))
    
    def setup_environment(self):
        import sys
        
        os.environ["CORE_PATH"] = str(self.core_lib_path)
        
        paths_to_add = [str(self.project_root), str(self.pep249_path)]
        for path in paths_to_add:
            if path not in sys.path:
                sys.path.insert(0, path)
        
        current_pythonpath = os.environ.get("PYTHONPATH", "")
        new_paths = paths_to_add.copy()
        
        if current_pythonpath:
            new_paths.append(current_pythonpath)
            
        os.environ["PYTHONPATH"] = os.pathsep.join(new_paths)


config = Config()
