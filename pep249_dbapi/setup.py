#!/usr/bin/env python3
"""
Setup script for PEP 249 Database API 2.0 implementation.
"""

from setuptools import setup, find_packages

with open("README.md", "r", encoding="utf-8") as fh:
    long_description = fh.read()

setup(
    name="pep249-dbapi",
    version="0.1.0",
    author="Your Name",
    author_email="your.email@example.com",
    description="A PEP 249 Database API 2.0 implementation with empty interface",
    long_description=long_description,
    long_description_content_type="text/markdown",
    url="https://github.com/yourusername/pep249-dbapi",
    packages=find_packages(),
    classifiers=[
        "Development Status :: 3 - Alpha",
        "Intended Audience :: Developers",
        "Topic :: Database",
        "Topic :: Software Development :: Libraries :: Python Modules",
        "License :: OSI Approved :: MIT License",
        "Programming Language :: Python :: 3",
        "Programming Language :: Python :: 3.7",
        "Programming Language :: Python :: 3.8",
        "Programming Language :: Python :: 3.9",
        "Programming Language :: Python :: 3.10",
        "Programming Language :: Python :: 3.11",
        "Programming Language :: Python :: 3.12",
    ],
    python_requires=">=3.7",
    install_requires=[
        "thrift>=0.17.0",
    ],
    extras_require={
        "dev": [
            "pytest>=6.0",
            "pytest-cov",
            "black",
            "flake8",
            "mypy",
        ],
        "test": [
            "pytest>=6.0",
            "pytest-cov",
        ],
    },
    entry_points={
        "console_scripts": [],
    },
    keywords="database api pep249 dbapi python",
    project_urls={
        "Bug Reports": "https://github.com/yourusername/pep249-dbapi/issues",
        "Source": "https://github.com/yourusername/pep249-dbapi/",
        "Documentation": "https://pep249-dbapi.readthedocs.io/",
    },
) 