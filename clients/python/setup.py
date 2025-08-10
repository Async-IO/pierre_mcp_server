# ABOUTME: Python package setup configuration for Pierre MCP Client
# ABOUTME: Defines package metadata, dependencies, and installation requirements

"""
Setup configuration for Pierre MCP Client

Python client library for connecting to Pierre MCP Server
"""

from setuptools import setup, find_packages

with open("README.md", "r", encoding="utf-8") as fh:
    long_description = fh.read()

setup(
    name="pierre-mcp-client",
    version="0.1.0",
    author="Pierre MCP Team",
    author_email="dev@pierre-mcp.com",
    description="Python client for Pierre MCP Server fitness data analysis",
    long_description=long_description,
    long_description_content_type="text/markdown",
    url="https://github.com/Async-IO/pierre_mcp_server",
    project_urls={
        "Bug Tracker": "https://github.com/Async-IO/pierre_mcp_server/issues",
        "Documentation": "https://github.com/Async-IO/pierre_mcp_server/blob/main/README.md",
        "Source Code": "https://github.com/Async-IO/pierre_mcp_server",
    },
    classifiers=[
        "Development Status :: 3 - Alpha",
        "Intended Audience :: Developers",
        "Topic :: Software Development :: Libraries :: Python Modules",
        "Topic :: Health",
        "Topic :: Scientific/Engineering :: Artificial Intelligence",
        "License :: OSI Approved :: Apache Software License",
        "License :: OSI Approved :: MIT License",
        "Programming Language :: Python :: 3",
        "Programming Language :: Python :: 3.8",
        "Programming Language :: Python :: 3.9",
        "Programming Language :: Python :: 3.10",
        "Programming Language :: Python :: 3.11",
        "Programming Language :: Python :: 3.12",
        "Operating System :: OS Independent",
    ],
    packages=find_packages(),
    python_requires=">=3.8",
    install_requires=[
        "aiohttp>=3.8.0",
        "asyncio-mqtt>=0.13.0",
    ],
    extras_require={
        "dev": [
            "pytest>=7.0.0",
            "pytest-asyncio>=0.21.0",
            "black>=22.0.0",
            "flake8>=5.0.0",
            "mypy>=1.0.0",
        ],
        "docs": [
            "sphinx>=5.0.0",
            "sphinx-rtd-theme>=1.0.0",
        ],
    },
    keywords="mcp fitness strava api ai assistant claude chatgpt",
    entry_points={
        "console_scripts": [
            "pierre-mcp=pierre_mcp.cli:main",
        ],
    },
)