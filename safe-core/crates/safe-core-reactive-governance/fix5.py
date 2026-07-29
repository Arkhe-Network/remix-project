with open("src/lib.rs", "r") as f:
    data = f.read()

bad_string = """let sig_bytes = gov_sk.sign(&payload).to_vec();"""
good_string = """let signature: p256::ecdsa::Signature = gov_sk.sign(&payload);
        let sig_bytes = signature.to_vec();"""
data = data.replace(bad_string, good_string)

with open("src/lib.rs", "w") as f:
    f.write(data)
