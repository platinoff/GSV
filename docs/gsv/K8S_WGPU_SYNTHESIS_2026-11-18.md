# wgpu в Kubernetes — 2026 synthesis

**Date:** 2026-11-18 · **Question (owner):** «що стосовно wgpu контейнерів кубернетіса» · **Audience:** owner + future infra work

## TL;DR

1. **Для нашого fleet (Windows hub + 4 Android-phones, завжди-он, 0 Docker, 0 k8s)** — Kubernetes забагато machinery сьогодні. Single-box + phones — не час k8s.
2. **Якщо масштабування в ≥1 dedicated GPU node** — Kubernetes + device-plugin стане розумним тоді, коли phones не вистачає для 35B-A3B 360° forward або для окремого serve.
3. **wasm in k8s** — mature для short-task functions, не для GPU compute graphs; WebGPU/Vulkan міст між ними слабкий.
4. **wgpu (native Rust)** — official (rust-lang/wgpu), те, що ми вже використовуємо. Для k8s GPU-контейнери впритул NVIDIA/AMD device plugins (не wgpu-specific — wgpu просто бігає всередині контейнера як будь-який CUDA/Vulkan процес).
5. **Реальна ланка, яку варто знати:** WebGPU (browser) ↔ WGSL compute ↔ наші `mesh-wasm.wasm` ↔ `wgpu` Rust crate — все WGSL, всі сумісні концептуально, але **не контейнерним чином**.

## A. Що реально робить GPU в k8s

### A1. NVIDIA device plugin (k8s-device-plugin)

- Exposes `nvidia.com/gpu: N`. Daemonset. Потрібен pinned container toolkit + NVIDIA driver на хості.
- Driver ~= 384+ , container toolkit ≥1.11 для integrated Tegra GPU.
- Інтегровані GPU (AMD Radeon iGPU, Intel Arc) — поза офіційним NVIDIA — тримаються AMD ROCm device-plugin або generic `gpu.feature-discovery`.

### A2. AMD GPU nodes (gaming/desktop tier)

- AMD has `k8s-device-plugin` for ROCm (`rocm-dev` image). RDNA iGPU/GPUs visible as `amd.com/gpu: 1`.
- Потрібні kernel 5.15+ + ROCm 5.5+. Сучасний stream — `amd.com/gpu` ресурс тип, official node labeller.

### A3. Generic GPU NFD labelling

- `nodeFeatureDiscovery` (NFD) label node за `pci-10.present: true` — будь-який k8s node з GPU видимий через NodeSelector.

### A4. NVIDIA GPU Operator (Helm)

- Готовий пакет для k8s GPU: toolkit + driver image + device plugin + driver validation. Найпростіший шлях.
- У 7.6 GB RAM-боксі з iGPU — не запускається; для ≥24 GB dedicated GPU node — дійде.

## B. Wasm in Kubernetes (окремо від GPU)

### B1. SpinKube / containerd-wasm-shims

```
crane push my-func.wasm ghcr.io/...
```

Runtime: `io.containerd.wasmedge-v1` / `wasmtime`. Кожен pod = 1 wasm module. Ідеально для short handlers (<500 ms).

### B2. wasi-nn / wasmedge for ML

- wasi-nn exposes an ndarray compute graph over ONNX/TFLite. Works on CPU. GPU compute через wasi-nn не було стабільним до 2026 — свіжі docs мають TensorFlow/ONNX execution, GPU backend не primary.
- WebGPU-специфічний wasm-компонент: `wasi-nn-webgpu` (experimental, не mainstream).

### B3. wasmCloud (distributed wasm operator)

- wasmCloud operator claims: vendor-neutral distributed runtime, lattice of wasm components, scales from 1 node to 100+.
- Під капотом — wRPC over NATS, не WebGPU. Для нашого use case (phone WebGPU workers computing 35B tensors) — не допомагав би.

### B4. Спинність

```
"Is WebAssembly Ready to Replace Containers on Kubernetes in 2026?" → No. It runs alongside (2026 verdict), not instead.
```

## C. WebGPU / wgpu containers

### C1. wgpu as a Rust library

- `wgpu` = our native Rust backend used via llama-rs. It targets Vulkan/Metal/DX12/GL.
- In a Docker container on a host with GPU: the container just sees the GPU through `/dev/dri` (Vulkan) or NVIDIA container toolkit (CUDA fallback для Vulkan NV).
- Для RDNA2/FF-PC → `docker` з `/dev/dri` mount — wgpu буде працювати з RADV (Mesa) або AMD proprietary Vulkan driver.
- Для phones — no Docker/k8s; phones host browser-only WebGPU.

### C2. "wgpu containers" — не існує продукту

"wgpu + k8s" не окремий продукт. Це всього лише: Rust-процес у контейнері + GPU device plugin. Контейнеризація wgpu = контейнеризація Vulkan/ROCm/CUDA.

### C3. Приклад

```yaml
spec:
  containers:
    - name: gsv-inference
      image: ghcr.io/platinoff/gsv-inference:latest
      resources:
        limits:
          amd.com/gpu: 1          # або nvidia.com/gpu: 1
      env:
        VULKAN_SDK: /usr/share/vulkan/1.3.x
```

## D. Для нашої системи: що реально застосувати

### D1. Сьогодні (Windows hub, 4 phones, 7.6 GB)

- **Не k8s. Не Docker.** Single-box Rust server + remote peers.
- Phase-out if: (i) ≥1 dedicated 24 GB GPU node; (ii) phones saturate; (iii) registry needed for multiple backend builds (llama-rs Vulkan/cpu/rpc).

### D2. Можливий середньотерміновий upgrade

- 1 підхід — compose-стек на **≥24 GB node**:
  - `gsv-hub` (axum :9999, task dispatcher, weight store)
  - `llama-serve` (Vulkan/ROCm runtime)
  - `gsv-peer-agent` (Rust, як `mesh-node` на Pi 4)
  - `k8s device-plugin` для AMD/NVIDIA
- Namespace `gsv` з pod для кожного шару.
- No phone WebGPU — cups replaced by GPU-direct Vulkan inference.

### D3. Мінімальний k8s-like (dev/local) той, що inside our scope

- `docker compose` на хабі (Docker Desktop): not feasible (7.6 GB RAM and hub never differs from cargo builds).
- If owner wants k8s for real → потрібен окремий node з ≥16 GB RAM.

### D4. PDF/MCP note

- MCP server `gsv_mcp_openbot` can expose **pod status probes** as tools if hub starts running on k8s (live health probes, but only after that hardware exists).

## E. Вердикт

**WGPu/WebGPU in k8s — не продукт.** Це або (а) k8s з GPU nodes + Vulkan/ROCm srcfite runtime (як llama-rs' Vulkan build) — звичайний контейнеризований GPU workload; або (б) wasm-wasmi спереду (обмежений CPU). Phones не під k8s. Для нашого fleet k8s пізно.

**Що реально варто зробити:**
1. Не міняти fleet topology.
2. Коли буде ≥1 GPU node — будувати gsv-inference як single Pod + AMD/NVIDIA plugin.
3. Поки — plug in phones as already-remote workers + WASM SIMD как або-або.
4. WebAssembly — для hosted short ops (matvec, softmax), не для orchestrated inference.

## F. Джерела

1. k8s GPU scheduling — kubernetes.io/docs/tasks/manage-gpus/scheduling-gpus/
2. NVIDIA k8s-device-plugin — github.com/NVIDIA/k8s-device-plugin
3. NVIDIA GPU Operator — docs.nvidia.com/datacenter/cloud-native/gpu-operator/
4. SpinKube — thenewstack.io/how-to-run-webassembly-on-kubernetes/
5. CNCF wasm-on-k8s practice guide — cncf.io/blog/2024/03/28/
6. Qovery verdict — qovery.com/blog/webassembly-replace-containers-kubernetes-2026-verdict/
7. WasmEdge GPU container — wasmedge.org/docs/develop/deploy/gpu/
8. WebGPU GPU portability — sessionize.com/download/isjatneo~... (CNCF presentation)
