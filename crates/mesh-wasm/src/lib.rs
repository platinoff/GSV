//! GSV mesh browser WASM SIMD math kernels.
//!
//! The `/wgpu` hub page ships a small WebGPU runner whose JS CPU reference
//! checks (REF) are also written in plain JS. This module replaces those hot
//! reference paths with WASM SIMD128 kernels so the phone/Pi browser lane
//! spends its cycles in native WASM, not the (slower, harder to optimise) JS
//! interpreter loop.
//!
//! Build (MSYS2 bash, from repo root):
//!   cargo build --release -p mesh-wasm --target wasm32-unknown-unknown
//!   cp target/wasm32-unknown-unknown/release/mesh_wasm.wasm ui/assets/mesh-wasm.wasm
//!
//! The browser feature-detects SIMD at instantiation time; if `WebAssembly`
//! rejects the module (no SIMD128), the page falls back to its JS reference.
//!
//! ABI: all pointers are into this module's exported linear memory (`memory`).
//! JS writes f32 inputs via `mesh_alloc` + (Float32Array view on
//! `memory.buffer`), calls the kernel, reads the outputs, then `mesh_free`s.
//! No side effects, no globals, no threads.

#![allow(clippy::missing_safety_doc)]

use std::alloc::{alloc, dealloc, Layout};
use std::arch::wasm32::{f32x4, f32x4_add, f32x4_mul, f32x4_splat, f32x4_sub, v128};

const ALIGN: usize = 4;

/// Raw byte allocator hook used by the JS side. Returns a pointer to `size`
/// uninitialised bytes aligned to 4, or 0 on failure.
#[no_mangle]
pub extern "C" fn mesh_alloc(size: usize) -> *mut u8 {
    let Ok(layout) = Layout::from_size_align(size.max(ALIGN), ALIGN) else {
        return std::ptr::null_mut();
    };
    unsafe { alloc(layout) }
}

/// Companion to `mesh_alloc`. `size` must match the earlier call exactly.
#[no_mangle]
pub unsafe extern "C" fn mesh_free(ptr: *mut u8, size: usize) {
    if ptr.is_null() || size == 0 {
        return;
    }
    if let Ok(layout) = Layout::from_size_align(size.max(ALIGN), ALIGN) {
        dealloc(ptr, layout);
    }
}

#[inline(always)]
unsafe fn loadx4(p: *const f32) -> v128 {
    p.cast::<v128>().read_unaligned()
}

#[inline(always)]
unsafe fn lanes(v: v128) -> [f32; 4] {
    std::mem::transmute(v)
}

#[inline(always)]
unsafe fn sumx4(v: v128) -> f32 {
    let l = lanes(v);
    l[0] + l[1] + l[2] + l[3]
}

/// y[r] = Σ_k W[r*kK + k] * X[k] for r in [0, rows).
#[no_mangle]
#[target_feature(enable = "simd128")]
pub unsafe extern "C" fn mesh_matvec(
    w: *const f32,
    x: *const f32,
    y: *mut f32,
    rows: usize,
    k_: usize,
) {
    for r in 0..rows {
        let row = w.add(r * k_);
        let mut acc = f32x4_splat(0.0);
        let mut base = 0usize;
        while base + 4 <= k_ {
            let wv = loadx4(row.add(base));
            let xv = loadx4(x.add(base));
            acc = f32x4_add(acc, f32x4_mul(wv, xv));
            base += 4;
        }
        let mut sum = sumx4(acc);
        while base < k_ {
            sum += *row.add(base) * *x.add(base);
            base += 1;
        }
        *y.add(r) = sum;
    }
}

/// C[y*64 + x] = Σ_k A[y*64+k] * B[k*64+x] — classic 64×64 f32 matmul.
#[no_mangle]
#[target_feature(enable = "simd128")]
pub unsafe extern "C" fn mesh_matmul64(a: *const f32, b: *const f32, c: *mut f32) {
    const N: usize = 64;
    for y in 0..N {
        for x in 0..N {
            let mut acc = f32x4_splat(0.0);
            let mut k = 0usize;
            while k < N {
                let av = loadx4(a.add(y * N + k));
                let bv = f32x4(
                    *b.add(k * N + x),
                    *b.add((k + 1) * N + x),
                    *b.add((k + 2) * N + x),
                    *b.add((k + 3) * N + x),
                );
                acc = f32x4_add(acc, f32x4_mul(av, bv));
                k += 4;
            }
            *c.add(y * N + x) = sumx4(acc);
        }
    }
}

/// In-place numerically-stable softmax over `len` elements.
#[no_mangle]
pub unsafe extern "C" fn mesh_softmax(x: *mut f32, len: usize) {
    if len == 0 {
        return;
    }
    let mut maxv = *x;
    for i in 1..len {
        let v = *x.add(i);
        if v > maxv {
            maxv = v;
        }
    }
    let mut sum = 0.0f32;
    for i in 0..len {
        let e = (*x.add(i) - maxv).exp();
        *x.add(i) = e;
        sum += e;
    }
    let inv = 1.0 / sum;
    for i in 0..len {
        *x.add(i) *= inv;
    }
}

/// y[i] = (x[i] - mean) / sqrt(var + 1e-5) over `len` elements.
#[no_mangle]
#[target_feature(enable = "simd128")]
pub unsafe extern "C" fn mesh_layernorm(x: *const f32, y: *mut f32, len: usize) {
    if len == 0 {
        return;
    }
    let mut acc = f32x4_splat(0.0);
    let mut base = 0usize;
    while base + 4 <= len {
        acc = f32x4_add(acc, loadx4(x.add(base)));
        base += 4;
    }
    let mut sum = sumx4(acc);
    while base < len {
        sum += *x.add(base);
        base += 1;
    }
    let mean = sum / len as f32;
    let meanv = f32x4_splat(mean);
    let mut vacc = f32x4_splat(0.0);
    let mut base = 0usize;
    while base + 4 <= len {
        let d = f32x4_sub(loadx4(x.add(base)), meanv);
        vacc = f32x4_add(vacc, f32x4_mul(d, d));
        base += 4;
    }
    let mut varsum = sumx4(vacc);
    while base < len {
        let d = *x.add(base) - mean;
        varsum += d * d;
        base += 1;
    }
    let inv = 1.0 / (varsum / len as f32 + 1e-5).sqrt();
    for i in 0..len {
        *y.add(i) = (*x.add(i) - mean) * inv;
    }
}

/// Transformer-style MLP block reference:
/// hid[h] = s / (1 + exp(-s)) with s = dot(w1[h*k + :], x)
/// y[o]   = Σ_h w2[o*hidden + h] * hid[h]
#[no_mangle]
#[target_feature(enable = "simd128")]
pub unsafe extern "C" fn mesh_mlpblock(
    w1: *const f32,
    x: *const f32,
    w2: *const f32,
    y: *mut f32,
    hidden: usize,
    out: usize,
    k: usize,
) {
    let mut hid = vec![0.0f32; hidden];
    for h2 in 0..hidden {
        let row = w1.add(h2 * k);
        let mut acc = f32x4_splat(0.0);
        let mut base = 0usize;
        while base + 4 <= k {
            let wv = loadx4(row.add(base));
            let xv = loadx4(x.add(base));
            acc = f32x4_add(acc, f32x4_mul(wv, xv));
            base += 4;
        }
        let mut s = sumx4(acc);
        while base < k {
            s += *row.add(base) * *x.add(base);
            base += 1;
        }
        hid[h2] = s / (1.0 + (-s).exp());
    }
    for o in 0..out {
        let row = w2.add(o * hidden);
        let mut acc = f32x4_splat(0.0);
        let mut base = 0usize;
        while base + 4 <= hidden {
            let wv = loadx4(row.add(base));
            let hv = loadx4(hid.as_ptr().add(base));
            acc = f32x4_add(acc, f32x4_mul(wv, hv));
            base += 4;
        }
        let mut sum = sumx4(acc);
        while base < hidden {
            sum += *row.add(base) * hid[base];
            base += 1;
        }
        *y.add(o) = sum;
    }
}
