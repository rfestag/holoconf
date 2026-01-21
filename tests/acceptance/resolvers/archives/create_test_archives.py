#!/usr/bin/env python3
"""Create test archives for extract resolver acceptance tests."""

import os
import tarfile
import zipfile
from pathlib import Path

# Get the directory where this script is located
SCRIPT_DIR = Path(__file__).parent

def create_test_zip():
    """Create a simple ZIP archive with test files."""
    zip_path = SCRIPT_DIR / "test.zip"
    with zipfile.ZipFile(zip_path, 'w') as zf:
        zf.writestr("config.json", '{"name": "test", "version": "1.0"}')
        zf.writestr("README.txt", "This is a test archive")
        zf.writestr("data/values.csv", "id,value\n1,alpha\n2,beta")
    print(f"Created {zip_path}")

def create_test_tar():
    """Create a simple TAR archive."""
    tar_path = SCRIPT_DIR / "test.tar"
    with tarfile.open(tar_path, 'w') as tf:
        # Add files from memory
        import io
        import tarfile as tf_module

        # config.yaml
        info = tf_module.TarInfo(name="config.yaml")
        data = b"app: myapp\nport: 8080"
        info.size = len(data)
        tf.addfile(info, io.BytesIO(data))

        # settings.txt
        info = tf_module.TarInfo(name="settings.txt")
        data = b"debug=true\ntimeout=30"
        info.size = len(data)
        tf.addfile(info, io.BytesIO(data))
    print(f"Created {tar_path}")

def create_test_tar_gz():
    """Create a gzip-compressed TAR archive."""
    tar_gz_path = SCRIPT_DIR / "test.tar.gz"
    with tarfile.open(tar_gz_path, 'w:gz') as tf:
        import io
        import tarfile as tf_module

        # data.json
        info = tf_module.TarInfo(name="data.json")
        data = b'[{"id": 1}, {"id": 2}]'
        info.size = len(data)
        tf.addfile(info, io.BytesIO(data))

        # notes.txt
        info = tf_module.TarInfo(name="notes.txt")
        data = b"Important notes here"
        info.size = len(data)
        tf.addfile(info, io.BytesIO(data))
    print(f"Created {tar_gz_path}")

if __name__ == "__main__":
    create_test_zip()
    create_test_tar()
    create_test_tar_gz()
    print("All test archives created successfully!")
