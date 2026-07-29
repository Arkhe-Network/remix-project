"""
Cathedral ARKHE v17.0 - Benchmark Suite
Reproduz os testes do Section 8 do guia.
"""
import asyncio
import time
import logging
import numpy as np
from pathlib import Path

logging.basicConfig(level=logging.WARNING)

from v17.config_loader import CathedralConfig
from v17.fast_brain import FastBrain, VisionModule, WorldModelRSSM, SafetyEngineZ3, EpisodicMemoryHNSW, MetaLearningModule

def benchmark_vision():
    print("=" * 60)
    print("BENCHMARK: Vision Module (YOLOv8-Nano)")
    print("=" * 60)

    vision = VisionModule("yolov8n", "cpu", 0.5)
    dummy_frame = np.random.randint(0, 255, (640, 640, 3), dtype=np.uint8)

    # Warmup
    for _ in range(3):
        vision.process(dummy_frame)

    # Batch 1
    times = []
    for _ in range(100):
        t0 = time.perf_counter()
        vision.process(dummy_frame)
        times.append((time.perf_counter() - t0) * 1000)

    print(f"  Batch=1:  {np.mean(times):.2f}ms avg, {1000/np.mean(times):.0f} Hz")

    # Batch 4
    if vision.model:
        frames = [dummy_frame] * 4
        times4 = []
        for _ in range(50):
            t0 = time.perf_counter()
            for f in frames:
                vision.process(f)
            times4.append((time.perf_counter() - t0) * 1000)
        print(f"  Batch=4:  {np.mean(times4):.2f}ms avg, {4000/np.mean(times4):.0f} Hz total")

    print()


def benchmark_world_model():
    print("=" * 60)
    print("BENCHMARK: World Model (RSSM)")
    print("=" * 60)

    wm = WorldModelRSSM(256, 32, 256, "cpu")
    features = np.random.randn(256).astype(np.float32)
    action = np.random.randn(4).astype(np.float32)

    # Warmup
    for _ in range(10):
        wm.step(features, action)

    times = []
    for _ in range(1000):
        t0 = time.perf_counter()
        wm.step(features, action)
        times.append((time.perf_counter() - t0) * 1000)

    print(f"  1 step:   {np.mean(times)*1000:.1f}µs avg, {1000/np.mean(times):.0f} Hz")
    print()


def benchmark_safety():
    print("=" * 60)
    print("BENCHMARK: Safety Engine (Z3)")
    print("=" * 60)

    safety = SafetyEngineZ3(10.0, ["humano", "animal", "objeto_fragil"])
    action = np.array([0.5, 0.3, 0.1, 0.0], dtype=np.float32)

    # Regras simples
    times_simple = []
    for _ in range(1000):
        t0 = time.perf_counter()
        safety.check(action, [])
        times_simple.append((time.perf_counter() - t0) * 1000)
    print(f"  Simples:  {np.mean(times_simple):.2f}ms avg, {1000/np.mean(times_simple):.0f} Hz")

    # Regras complexas (com detecções)
    detections = [{"class": 0, "conf": 0.9, "xyxy": [100, 100, 200, 200]}]
    action_unsafe = np.array([5.0, 3.0, 0.0, 0.0], dtype=np.float32)
    times_complex = []
    for _ in range(100):
        t0 = time.perf_counter()
        safety.check(action_unsafe, detections)
        times_complex.append((time.perf_counter() - t0) * 1000)
    print(f"  Complexo: {np.mean(times_complex):.2f}ms avg, {1000/np.mean(times_complex):.0f} Hz")
    print()


def benchmark_memory():
    print("=" * 60)
    print("BENCHMARK: Episodic Memory (HNSW)")
    print("=" * 60)

    mem = EpisodicMemoryHNSW(dim=288, data_dir="Cathedral/zvec_data/bench_temp")

    # Popula com 10k vetores
    print("  Populando 10.000 memórias...")
    t0 = time.perf_counter()
    for i in range(100):
        vec = np.random.randn(288).astype(np.float32)
        mem.store(vec, {"step": i, "label": f"mem_{i}"})
    populate_time = time.perf_counter() - t0
    print(f"  População: {populate_time:.2f}s ({10000/populate_time:.0f} mem/s)")

    # Retrieval
    query = np.random.randn(288).astype(np.float32)
    times = []
    for _ in range(1000):
        t0 = time.perf_counter()
        mem.retrieve(query, top_k=5)
        times.append((time.perf_counter() - t0) * 1000)
    print(f"  Retrieval: {np.mean(times):.2f}ms avg, {1000/np.mean(times):.0f} Hz")
    print()


def benchmark_full_cycle():
    print("=" * 60)
    print("BENCHMARK: Full Fast Brain Cycle")
    print("=" * 60)

    config = CathedralConfig()
    fb = FastBrain(config)

    # Warmup
    for _ in range(5):
        fb.cycle()

    times = []
    for _ in range(100):
        t0 = time.perf_counter()
        fb.cycle()
        times.append((time.perf_counter() - t0) * 1000)

    mean_ms = np.mean(times)
    freq = 1000 / mean_ms
    target_pass = mean_ms < 5.0

    print(f"  Média:    {mean_ms:.2f}ms")
    print(f"  Min:      {np.min(times):.2f}ms")
    print(f"  Max:      {np.max(times):.2f}ms")
    print(f"  P99:      {np.percentile(times, 99):.2f}ms")
    print(f"  Freq:     {freq:.0f} Hz")
    print(f"  Alvo:     < 5ms, > 100 Hz  ->  {'✅ PASS' if target_pass else '❌ FAIL'}")
    print()


async def benchmark_slow_brain():
    print("=" * 60)
    print("BENCHMARK: Slow Brain (SGLang)")
    print("=" * 60)

    config = CathedralConfig()
    from v17.slow_brain import SlowBrainSGLang
    sb = SlowBrainSGLang(config)

    healthy = await sb.health_check()
    if not healthy:
        print("  ❌ SGLang offline — pulando benchmark do Slow Brain")
        print("  Inicie o SGLang no WSL2 e rode novamente.")
        print()
        return

    prompts = [
        ("Simples", "O robô deve seguir em frente.", 512),
        ("RAG", "Contexto com 5 memórias episódicas recuperadas do zVEC.", 2048),
        ("Complexo", "Dilema: copo de vidro + humano à frente + baixa confiança do Fast Brain.", 4096),
    ]

    for name, prompt, _ in prompts:
        times = []
        for _ in range(3):
            t0 = time.perf_counter()
            await sb.reason(dilemma=prompt)
            times.append((time.perf_counter() - t0) * 1000)
        print(f"  {name:12s}: {np.mean(times):.0f}ms avg")

    print()


def benchmark_router():
    print("=" * 60)
    print("BENCHMARK: Router Decisions")
    print("=" * 60)

    config = CathedralConfig()
    from v17.orchestrator_v17 import Router
    from v17.fast_brain import FastBrainState

    router = Router(config)

    scenarios = [
        ("Corredor limpo", 0.8, True, []),
        ("Copo de vidro", 0.2, False, []),
        ("Baixa confiança", 0.1, True, []),
    ]

    for name, conf, safety, mems in scenarios:
        state = FastBrainState(
            confidence=conf,
            safety_approved=safety,
            zvec_memories=mems,
            action=np.zeros(4),
        )
        times = []
        for _ in range(1000):
            t0 = time.perf_counter()
            route = router.decide(state)
            times.append((time.perf_counter() - t0) * 1000)
        print(f"  {name:20s}: route={route:5s}, {np.mean(times):.0f}µs avg")

    print()


def print_hardware_info():
    print("=" * 60)
    print("HARDWARE INFO")
    print("=" * 60)
    import torch
    import psutil

    if torch.cuda.is_available():
        props = torch.cuda.get_device_properties(0)
        print(f"  GPU: {props.name}")
        print(f"  VRAM: {props.total_mem / 1024**3:.1f} GB")
        print(f"  Compute: sm_{props.major}{props.minor}")
    else:
        print("  GPU: N/A (CUDA não disponível)")

    print(f"  RAM: {psutil.virtual_memory().total / 1024**3:.0f} GB")
    print(f"  CPU: {psutil.cpu_count(logical=False)} cores / {psutil.cpu_count()} threads")
    print(f"  PyTorch: {torch.__version__}")
    print()


if __name__ == "__main__":
    print_hardware_info()
    benchmark_vision()
    benchmark_world_model()
    benchmark_safety()
    benchmark_memory()
    benchmark_full_cycle()
    benchmark_router()
    asyncio.run(benchmark_slow_brain())

    print("=" * 60)
    print("BENCHMARKS COMPLETOS")
    print("=" * 60)
