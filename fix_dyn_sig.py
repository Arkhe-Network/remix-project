with open("safe-core/crates/dyn-signature/src/lib.rs", "r") as f:
    data = f.read()
data = data.replace("#[derive(Debug, Clone, PartialEq, Eq, Hash)]", "#[derive(Debug, Clone, PartialEq, Eq)]")
with open("safe-core/crates/dyn-signature/src/lib.rs", "w") as f:
    f.write(data)
