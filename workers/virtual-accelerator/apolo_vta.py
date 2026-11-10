#!/usr/bin/env python3
"""APOLO VTA: virtual accelerator resource planner (not a hardware emulator).

It aggregates observed worker inventory and estimates whether a training plan
could fit. It deliberately does not claim extra FLOPS or execute distributed
training; the current AX worker is single-rank.
"""
from __future__ import annotations

import argparse
import json
import os
import sys
from typing import Any

import requests

BYTES_PER_PARAMETER_ADAMW_BF16 = 16
SAFETY_FACTOR = 0.65
HUGE_NAMES = {
    "googolplex", "100 googolplex", "centillion", "um centilhao",
    "um centilhão", "100 centillion", "100 centilhoes", "100 centilhões",
}


def estimate_capacity(workers: list[dict[str, Any]]) -> dict[str, Any]:
    active = [
        w for w in workers
        if w.get("status") == "online"
        and w.get("last_seen_at") is not None
    ]
    gpu_workers = [
        w for w in active
        if int(w.get("gpu_free_memory_bytes") or 0) > 0
        and (w.get("gpu_name") or w.get("accelerator_kind") == "cuda")
    ]
    cpu_workers = [w for w in active if w not in gpu_workers]
    gpu_free = sum(int(w.get("gpu_free_memory_bytes") or 0) for w in gpu_workers)
    ram_free = sum(int(w.get("ram_available_bytes") or 0) for w in cpu_workers)
    # Conservative planning estimate for BF16 + AdamW, including room for
    # activations, communication buffers, fragmentation and runtime overhead.
    gpu_params = int(gpu_free * SAFETY_FACTOR / BYTES_PER_PARAMETER_ADAMW_BF16)
    cpu_params = int(ram_free * 0.35 / 16) if not gpu_workers else 0
    return {
        "active_workers": len(active),
        "gpu_workers": len(gpu_workers),
        "cpu_only_workers": len(cpu_workers),
        "aggregate_gpu_free_bytes": gpu_free,
        "aggregate_cpu_ram_available_bytes": ram_free,
        "estimated_gpu_parameters_bf16_adamw": gpu_params,
        "estimated_cpu_parameters_very_slow": cpu_params,
        "planning_assumptions": {
            "bytes_per_parameter": BYTES_PER_PARAMETER_ADAMW_BF16,
            "safety_factor": SAFETY_FACTOR,
            "note": "Estimativa aproximada; não é benchmark nem garantia de que o modelo caiba."
        }
    }


def assess(requested: str, capacity: dict[str, Any]) -> dict[str, Any]:
    normalized = requested.strip().lower()
    if normalized in HUGE_NAMES or "googolplex" in normalized or "centilh" in normalized:
        return {
            "requested_parameters": requested,
            "feasible": False,
            "reason": "Escala matematicamente muito além da capacidade física; não existe plano executável.",
        }
    try:
        count = int(normalized.replace("_", "").replace(",", ""))
        if count <= 0:
            raise ValueError()
    except ValueError:
        return {
            "requested_parameters": requested,
            "feasible": None,
            "reason": "Informe a quantidade como inteiro decimal ou use googolplex/centilhão para análise de inviabilidade.",
        }
    limit = capacity["estimated_gpu_parameters_bf16_adamw"]
    if capacity["gpu_workers"] == 0:
        limit = capacity["estimated_cpu_parameters_very_slow"]
    return {
        "requested_parameters": requested,
        "feasible": count <= limit and limit > 0,
        "estimated_parameter_limit": limit,
        "reason": (
            "Estimativa de memória dentro do inventário observado; ainda exige validação do runtime, dataset e comunicação."
            if count <= limit and limit > 0
            else "O inventário online não oferece memória estimada suficiente."
        ),
    }


def main() -> int:
    parser = argparse.ArgumentParser(description="APOLO VTA virtual accelerator planner")
    parser.add_argument("--parameters", required=True, help="Parâmetros do modelo (inteiro), googolplex ou centilhão")
    parser.add_argument("--api", default=os.getenv("APOLO_AX_API", "").rstrip("/"))
    parser.add_argument("--token", default=os.getenv("APOLO_AX_TOKEN", ""))
    args = parser.parse_args()
    if not args.api or not args.token:
        parser.error("Defina APOLO_AX_API e APOLO_AX_TOKEN.")
    response = requests.get(
        args.api + "/api/v1/workers",
        headers={"Authorization": "Bearer " + args.token, "Accept": "application/json"},
        timeout=30,
    )
    response.raise_for_status()
    workers = response.json()
    capacity = estimate_capacity(workers)
    result = {
        "product": "APOLO VTA",
        "mode": "virtual_resource_orchestrator",
        "hardware_emulation": False,
        "planning_only": True,
        "execution_ready": False,
        "capacity": capacity,
        "assessment": assess(args.parameters, capacity),
        "distributed_runtime": "NOT_INSTALLED",
        "next_step": "Conectar workers reais e implementar/validar torchrun + FSDP2/TP antes de habilitar treinamento multi-node.",
        "warning": "Software virtual não cria FLOPS, VRAM ou largura de banda. Aumenta eficiência/orquestração; a velocidade depende do hardware e da rede reais."
    }
    print(json.dumps(result, ensure_ascii=False, indent=2))
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except requests.RequestException as exc:
        print(json.dumps({"error": str(exc)}, ensure_ascii=False), file=sys.stderr)
        raise SystemExit(2)
