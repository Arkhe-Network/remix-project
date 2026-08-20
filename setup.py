from setuptools import setup, find_packages

setup(
    name="zeitgeist-safe",
    version="1.0.0-stable",
    description="Spectral regime detection with safety guarantees",
    author="Arkhe(n) Research Group",
    python_requires=">=3.10",
    packages=find_packages(),
    install_requires=[
        "numpy>=1.24",
    ],
    extras_require={
        "dev": ["pytest>=7.0", "black", "mypy"],
    },
    classifiers=[
        "Development Status :: 5 - Production/Stable",
        "Intended Audience :: Science/Research",
        "License :: OSI Approved :: MIT License",
        "Programming Language :: Python :: 3.10",
        "Programming Language :: Python :: 3.11",
        "Programming Language :: Python :: 3.12",
        "Topic :: Scientific/Engineering :: Artificial Intelligence",
    ],
)
