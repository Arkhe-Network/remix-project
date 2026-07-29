import os
from web3 import Web3

N = 16
W = 8
L = 43
K = 8
A = 16
D = 2
H_TOTAL = 24
H_PER_LAYER = H_TOTAL // D

# The target sum for 43 base-8 digits. Mean is 43*3.5 = 150.5. We use 150 to make grinding fast.
WOTS_TARGET_SUM = 150

def hash_n(data: bytes) -> bytes:
    h = Web3.keccak(data)
    return h[:N]

def hash_n_multi(*args) -> bytes:
    data = b''.join(args)
    return hash_n(data)

def base_w_digits(msg: bytes, w: int, out_len: int) -> list[int]:
    digits = []
    bit_len = 0
    while (1 << bit_len) < w:
        bit_len += 1

    in_idx = 0
    bits_in_buffer = 0
    buffer = 0

    for _ in range(out_len):
        if bits_in_buffer < bit_len:
            b = msg[in_idx] if in_idx < len(msg) else 0
            buffer = (buffer << 8) | b
            in_idx += 1
            bits_in_buffer += 8

        shift = bits_in_buffer - bit_len
        digit = (buffer >> shift) & ((1 << bit_len) - 1)
        bits_in_buffer -= bit_len
        digits.append(digit)

    return digits

def wots_pk_from_sig(sig: bytes, msg: bytes, w: int, l: int) -> bytes:
    chains = []

    msg_hash = hash_n(msg)
    msg_hash_32 = msg_hash + b'\x00'*(32-len(msg_hash))
    msg_digits = base_w_digits(msg_hash_32, w, l)

    for i in range(l):
        val = sig[i*N:(i+1)*N]
        digit = msg_digits[i] if i < len(msg_digits) else 0

        for _ in range(w - 1 - digit):
            val_32 = val + b'\x00'*(32-N)
            val = Web3.keccak(val_32)[:N]

        chains.append(val + b'\x00'*(32-N))

    h_final = Web3.keccak(b''.join(chains))
    return h_final[:N]

def build_merkle_tree(leaves: list[bytes]) -> tuple[bytes, list[list[bytes]]]:
    n_leaves = len(leaves)
    tree = [leaves]

    while len(tree[-1]) > 1:
        layer = tree[-1]
        next_layer = []
        for i in range(0, len(layer), 2):
            left = layer[i]
            right = layer[i+1] if i+1 < len(layer) else left
            left_32 = left + b'\x00'*(32-N)
            right_32 = right + b'\x00'*(32-N)
            h = Web3.keccak(left_32 + right_32)
            next_layer.append(h[:N])
        tree.append(next_layer)

    root = tree[-1][0]

    auth_paths = []
    for i in range(n_leaves):
        path = []
        idx = i
        for layer in tree[:-1]:
            sibling_idx = idx ^ 1
            if sibling_idx < len(layer):
                path.append(layer[sibling_idx])
            else:
                path.append(layer[idx])
            idx >>= 1
        auth_paths.append(b''.join(path))

    return root, auth_paths

def fors_sign(md: bytes, sk_seed: bytes) -> tuple[bytes, bytes]:
    num_leaves = 1 << A
    pk_roots = []
    sig_items = []

    for i in range(K):
        leaves = []
        for j in range(num_leaves):
            sec = hash_n_multi(sk_seed, b'fors', i.to_bytes(4, 'big'), j.to_bytes(4, 'big'))
            sec_32 = sec + b'\x00'*(32-N)
            leaf = Web3.keccak(sec_32)[:N]
            leaves.append(leaf)

        root, auth_paths = build_merkle_tree(leaves)
        pk_roots.append(root + b'\x00'*(32-N))

        # SPHINCS+ disjoint index logic:
        # FORS uses top 128 bits: md[0:16]
        shift = i * A
        md_int = int.from_bytes(md[:16], 'big')
        leaf_idx = (md_int >> (128 - shift - A)) & ((1 << A) - 1)

        sec = hash_n_multi(sk_seed, b'fors', i.to_bytes(4, 'big'), leaf_idx.to_bytes(4, 'big'))
        sig_items.append(sec + auth_paths[leaf_idx])

    pk = Web3.keccak(b''.join(pk_roots))
    return pk, b''.join(sig_items)

def wots_sign(msg: bytes, sk_seed: bytes, layer: int) -> tuple[bytes, bytes]:
    msg_hash = hash_n(msg)
    msg_hash_32 = msg_hash + b'\x00'*(32-N)
    msg_digits = base_w_digits(msg_hash_32, W, L)

    sig = []
    for i in range(L):
        sec = hash_n_multi(sk_seed, b'wots', layer.to_bytes(4, 'big'), i.to_bytes(4, 'big'))

        digit = msg_digits[i] if i < len(msg_digits) else 0

        val = sec
        for _ in range(digit):
            val_32 = val + b'\x00'*(32-N)
            val = Web3.keccak(val_32)[:N]

        sig.append(val)

    sig_bytes = b''.join(sig)
    pk_16 = wots_pk_from_sig(sig_bytes, msg, W, L)
    return pk_16, sig_bytes

def keygen() -> bytes:
    return os.urandom(N)

def sign(msg: bytes, sk_seed: bytes) -> tuple[bytes, bytes]:
    # WOTS+C Target Sum Grinding
    while True:
        randomizer = os.urandom(N)
        randomizer_32 = randomizer + b'\x00'*(32-N)
        md = Web3.keccak(randomizer_32 + msg)

        # Check Layer 0 Grinding
        # fors_pk is the message to layer 0 WOTS
        fors_pk_32, fors_sig = fors_sign(md, sk_seed)
        l0_msg_hash = hash_n(fors_pk_32)
        l0_msg_hash_32 = l0_msg_hash + b'\x00'*(32-N)
        l0_digits = base_w_digits(l0_msg_hash_32, W, L)

        if sum(l0_digits) != WOTS_TARGET_SUM:
            continue

        # If layer 0 is good, we need to check layer 1.
        # But wait, layer 1 message is layer0_root.
        # Layer 0 root depends on layer0 WOTS pk and merkle tree.
        # This takes more time, but let's compute it.
        layer0_pk_16, layer0_sig = wots_sign(fors_pk_32, sk_seed, 0)

        md_int = int.from_bytes(md[16:20], 'big')
        idx_tree = (md_int >> 20) & 0xFFF
        idx_leaf = (md_int >> 8) & 0xFFF

        leaves0 = []
        for i in range(1 << H_PER_LAYER):
            if i == idx_leaf:
                leaves0.append(layer0_pk_16)
            else:
                leaves0.append(hash_n_multi(sk_seed, b'layer0_leaf', i.to_bytes(4, 'big')))

        layer0_root_16, auth_paths0 = build_merkle_tree(leaves0)
        layer0_auth_path = auth_paths0[idx_leaf]
        layer0_root_32 = layer0_root_16 + b'\x00'*(32-N)

        l1_msg_hash = hash_n(layer0_root_32)
        l1_msg_hash_32 = l1_msg_hash + b'\x00'*(32-N)
        l1_digits = base_w_digits(l1_msg_hash_32, W, L)

        if sum(l1_digits) == WOTS_TARGET_SUM:
            # We found a randomizer where both layer WOTS sum to exactly WOTS_TARGET_SUM
            break

    layer1_pk_16, layer1_sig = wots_sign(layer0_root_32, sk_seed, 1)

    leaves1 = []
    for i in range(1 << H_PER_LAYER):
        if i == idx_tree:
            leaves1.append(layer1_pk_16)
        else:
            leaves1.append(hash_n_multi(sk_seed, b'layer1_leaf', i.to_bytes(4, 'big')))

    layer1_root_16, auth_paths1 = build_merkle_tree(leaves1)
    layer1_auth_path = auth_paths1[idx_tree]

    signature = randomizer + fors_sig + layer0_sig + layer0_auth_path + layer1_sig + layer1_auth_path
    pk_root_32 = layer1_root_16 + b'\x00'*(32-N)

    return signature, pk_root_32
