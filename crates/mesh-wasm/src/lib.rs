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
#[cfg(target_arch = "wasm32")]
use std::arch::wasm32::{f32x4, f32x4_add, f32x4_mul, f32x4_splat, f32x4_sub, v128};
// Native builds (unit tests) only compile the scalar Q4_K kernel; SIMD math is
// wasm32-only by design. These stubs keep native `cargo test` compiling.
#[cfg(not(target_arch = "wasm32"))]
#[allow(non_camel_case_types, dead_code)]
type v128 = u128;
#[cfg(not(target_arch = "wasm32"))]
#[allow(non_snake_case, dead_code)]
fn f32x4(_v: f32) -> v128 {
    0
}
#[cfg(target_arch = "wasm32")]
#[inline(always)]
fn make4(a: f32, b: f32, c: f32, d: f32) -> v128 {
    f32x4(a, b, c, d)
}
#[cfg(not(target_arch = "wasm32"))]
fn make4(_a: f32, _b: f32, _c: f32, _d: f32) -> v128 {
    0
}
#[cfg(not(target_arch = "wasm32"))]
fn f32x4_add(a: v128, _b: v128) -> v128 {
    a
}
#[cfg(not(target_arch = "wasm32"))]
fn f32x4_mul(a: v128, _b: v128) -> v128 {
    a
}
#[cfg(not(target_arch = "wasm32"))]
fn f32x4_splat(_v: f32) -> v128 {
    0
}
#[cfg(not(target_arch = "wasm32"))]
fn f32x4_sub(a: v128, _b: v128) -> v128 {
    a
}

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
#[cfg_attr(target_arch = "wasm32", target_feature(enable = "simd128"))]
pub unsafe extern "C" fn mesh_matvec(
    w: *const f32,
    x: *const f32,
    y: *mut f32,
    rows: usize,
    k_: usize,
) {
    mesh_matvec_partial(w, x, y, rows, k_, 0, k_);
}

/// y[r] = Σ_{k in [k0, k1)} W[r*row_stride + k] * X[k] — partial-K matvec used by
/// the `matvec_shard` op (row-major W with arbitrary row stride; X is indexed
/// with an absolute base at k0). Enables sharded reductions on mesh peers.
#[no_mangle]
#[cfg_attr(target_arch = "wasm32", target_feature(enable = "simd128"))]
pub unsafe extern "C" fn mesh_matvec_partial(
    w: *const f32,
    x: *const f32,
    y: *mut f32,
    rows: usize,
    row_stride: usize,
    k0: usize,
    k1: usize,
) {
    if k1 <= k0 {
        for r in 0..rows {
            *y.add(r) = 0.0;
        }
        return;
    }
    for r in 0..rows {
        let row = w.add(r * row_stride);
        let mut acc = f32x4_splat(0.0);
        let mut base = k0;
        while base + 4 <= k1 {
            let wv = loadx4(row.add(base));
            let xv = loadx4(x.add(base));
            acc = f32x4_add(acc, f32x4_mul(wv, xv));
            base += 4;
        }
        let mut sum = sumx4(acc);
        while base < k1 {
            sum += *row.add(base) * *x.add(base);
            base += 1;
        }
        *y.add(r) = sum;
    }
}

/// f16 -> f32 widening (same field math as GSV `wgpu_dequant::f16_to_f32`).
#[inline(always)]
fn f16_to_f32(h: u16) -> f32 {
    let sign = ((h >> 15) as u32) << 31;
    let exp = ((h >> 10) & 0x1F) as u32;
    let frac = (h & 0x3FF) as u32;
    let bits = match exp {
        0 => {
            if frac == 0 {
                sign
            } else {
                // Match GSV wgpu_dequant::f16_to_f32 exactly: start the
                // subnormal exponent at 113 (bias-1), not at `exp` (0),
                // otherwise `d`/`min` scales blow up by ~2^14.
                let mut e = 113u32;
                let mut f = frac;
                while f & 0x400 == 0 {
                    f <<= 1;
                    e -= 1;
                }
                sign | (e << 23) | ((f & 0x3FF) << 13)
            }
        }
        0x1F => sign | 0x7F80_0000 | (frac << 13),
        _ => sign | ((exp + 112) << 23) | (frac << 13),
    };
    f32::from_bits(bits)
}

#[inline(always)]
fn scale_min_k4(j: usize, q: &[u8]) -> (u8, u8) {
    if j < 4 {
        (q[j] & 63, q[j + 4] & 63)
    } else {
        (
            (q[j + 4] & 0xF) | ((q[j - 4] >> 6) << 4),
            (q[j + 4] >> 4) | ((q[j] >> 6) << 4),
        )
    }
}

/// GASV WASM lane: dequantize `blocks` Q4_K super-blocks (144 B -> 256 f32)
/// natively instead of in the JS interpreter. Same math as
/// `gsv::boxes::wgpu_dequant::dequant_q4_k`.
///
/// # Safety
/// `src` must point to `blocks * 144` readable bytes and `dst` to
/// `blocks * 256` writable f32 slots, both from `mesh_alloc`.
#[no_mangle]
pub unsafe extern "C" fn mesh_dequant_q4_k(src: *const u8, dst: *mut f32, blocks: usize) {
    for b in 0..blocks {
        let base = b * 144;
        let block = std::slice::from_raw_parts(src.add(base), 144);
        let d = f16_to_f32(u16::from_le_bytes([block[0], block[1]]));
        let dmin = f16_to_f32(u16::from_le_bytes([block[2], block[3]]));
        let scales = &block[4..16];
        let qs = &block[16..144];
        let mut o = b * 256;
        for grp in 0..4 {
            let (sc0, m0) = scale_min_k4(grp * 2, scales);
            let (sc1, m1) = scale_min_k4(grp * 2 + 1, scales);
            let d1 = d * sc0 as f32;
            let dm1 = dmin * m0 as f32;
            let d2 = d * sc1 as f32;
            let dm2 = dmin * m1 as f32;
            let qb = grp * 32;
            for l in 0..32 {
                let q = qs[qb + l];
                *dst.add(o) = d1 * (q & 0xF) as f32 - dm1;
                o += 1;
            }
            for l in 0..32 {
                let q = qs[qb + l];
                *dst.add(o) = d2 * (q >> 4) as f32 - dm2;
                o += 1;
            }
        }
    }
}

/// ABI marker for the JS side (bump on breaking kernel signature changes).
#[no_mangle]
pub extern "C" fn mesh_version() -> u32 {
    1
}

/// C[y*64 + x] = Σ_k A[y*64+k] * B[k*64+x] — classic 64×64 f32 matmul.
#[no_mangle]
#[cfg_attr(target_arch = "wasm32", target_feature(enable = "simd128"))]
pub unsafe extern "C" fn mesh_matmul64(a: *const f32, b: *const f32, c: *mut f32) {
    const N: usize = 64;
    for y in 0..N {
        for x in 0..N {
            let mut acc = f32x4_splat(0.0);
            let mut k = 0usize;
            while k < N {
                let av = loadx4(a.add(y * N + k));
                let bv = make4(
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
#[cfg_attr(target_arch = "wasm32", target_feature(enable = "simd128"))]
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
#[allow(clippy::needless_range_loop)]
#[cfg_attr(target_arch = "wasm32", target_feature(enable = "simd128"))]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn q4_k_dequant_kernel_matches_scalar_expectation() {
        // d=1.0, dmin=0, all scales=1 => y = q nibble (low half first).
        let mut block = [0u8; 144];
        block[0..2].copy_from_slice(&0x3c00u16.to_le_bytes());
        for s in block.iter_mut().take(16).skip(4) {
            *s = 1;
        }
        block[16] = 5;
        block[17] = 7;
        let mut out = [0f32; 256];
        unsafe { mesh_dequant_q4_k(block.as_ptr(), out.as_mut_ptr(), 1) };
        assert_eq!(out[0], 5.0);
        assert_eq!(out[1], 7.0);
        assert_eq!(out[32], 0.0); // high nibble of qs[0]

        // Subnormal f16 scale must match GSV's bias-1 path, not blow up.
        let mut sub = [0u8; 144];
        sub[0..2].copy_from_slice(&0x0001u16.to_le_bytes()); // d = 2^-24
        for s in sub.iter_mut().take(16).skip(4) {
            *s = 1;
        }
        sub[16] = 5;
        let mut sub_out = [0f32; 256];
        unsafe { mesh_dequant_q4_k(sub.as_ptr(), sub_out.as_mut_ptr(), 1) };
        assert!(
            (sub_out[0] - 5.0 * 2f32.powi(-24)).abs() < 1e-12,
            "subnormal d must be 2^-24, got {}",
            sub_out[0]
        );

        // Byte-for-byte golden from GSV `dequant_q4_k` (diag_wasm_q4k_golden):
        // subnormal d, min=0.5, packed scales 0x21, q nibbles 1..16.
        let mut golden_block = [0u8; 144];
        golden_block[0..2].copy_from_slice(&0x0001u16.to_le_bytes());
        golden_block[2..4].copy_from_slice(&0x3800u16.to_le_bytes());
        for s in 4..16 {
            golden_block[s] = 0x21;
        }
        for q in 16..144 {
            golden_block[q] = ((q - 16) % 16) as u8 + 1;
        }
        let mut golden_out = [0f32; 256];
        unsafe { mesh_dequant_q4_k(golden_block.as_ptr(), golden_out.as_mut_ptr(), 1) };
        let expected = [
            -16.499998, -16.499996, -16.499994, -16.499992, -16.49999, -16.499989, -16.499987,
            -16.499985, -16.499983, -16.49998, -16.499979, -16.499977, -16.499975, -16.499973,
            -16.499971, -16.5,
        ];
        for (i, want) in expected.iter().enumerate() {
            assert!(
                (golden_out[i] - want).abs() < 1e-5,
                "golden idx {i}: got {} want {want}",
                golden_out[i]
            );
        }
    }
}
