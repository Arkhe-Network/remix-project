// src/lib.rs — Mínimo para compilar e gerar biblioteca nativa
#[no_mangle]
pub extern "C" fn arkhe_process_cycle(
    _input: *const f32,
    _len: usize,
) -> *mut f32 {
    // Stub: retorna vetor de zeros
    let mut result = vec![0.0f32; 4]; // action_dim = 4
    let ptr = result.as_mut_ptr();
    std::mem::forget(result);
    ptr
}

#[no_mangle]
pub extern "C" fn arkhe_free(ptr: *mut f32, len: usize) {
    if !ptr.is_null() {
        unsafe {
            let _ = Vec::from_raw_parts(ptr, len, len);
        }
    }
}
