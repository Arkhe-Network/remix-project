with open("safe-core/crates/safe-core-reactive-governance/src/reactive_log.rs", "r") as f:
    data = f.read()

bad_string = """        if let Some(vk) = &entry.verifying_key_opt { if !self.authorized_keys.contains(vk) { return Err(GovernanceError::Unauthorized(entry.issued_by)); } } else { return Err(GovernanceError::Unauthorized(entry.issued_by)); }
            return Err(GovernanceError::Unauthorized(entry.issued_by));
        } """
good_string = """        if let Some(vk) = &entry.verifying_key_opt {
            if !self.authorized_keys.contains(vk) {
                return Err(GovernanceError::Unauthorized(entry.issued_by));
            }
        } else {
            return Err(GovernanceError::Unauthorized(entry.issued_by));
        }"""

data = data.replace(bad_string, good_string)

with open("safe-core/crates/safe-core-reactive-governance/src/reactive_log.rs", "w") as f:
    f.write(data)
