#!/usr/bin/env python3
"""APOLO AX worker agent. Runs only on the machine where this process is installed."""
import json
import os
import platform
import subprocess
import tempfile
import time
from pathlib import Path
from urllib.parse import quote

import psutil
import requests

API = os.environ["APOLO_AX_API"].rstrip("/")
TOKEN = os.environ["APOLO_AX_TOKEN"]
NAME = os.getenv("APOLO_WORKER_NAME", platform.node())
POLL_SECONDS = int(os.getenv("APOLO_WORKER_POLL_SECONDS", "8"))
SESSION = requests.Session()
SESSION.headers.update({"Authorization": f"Bearer {TOKEN}", "Accept": "application/json"})

def call(method, path, **kwargs):
    response = SESSION.request(method, API + path, timeout=90, **kwargs)
    response.raise_for_status()
    return response

def hardware():
    gpu_name, gpu_bytes = None, 0
    try:
        import torch
        if torch.cuda.is_available():
            gpu_name = torch.cuda.get_device_name(0)
            gpu_bytes = torch.cuda.get_device_properties(0).total_memory
    except Exception:
        pass
    return {
        "name": NAME,
        "endpoint": None,
        "cpu_cores": psutil.cpu_count(logical=True) or 1,
        "ram_bytes": psutil.virtual_memory().total,
        "gpu_name": gpu_name,
        "gpu_memory_bytes": gpu_bytes,
        "framework": "pytorch",
    }

def telemetry():
    data = {"ram_available_bytes": psutil.virtual_memory().available, "cpu_utilization_pct": psutil.cpu_percent(interval=None), "accelerator_kind": "cpu"}
    try:
        import torch
        if torch.cuda.is_available():
            free_bytes, _total_bytes = torch.cuda.mem_get_info(0)
            data.update({"gpu_free_memory_bytes": int(free_bytes), "accelerator_kind": "cuda", "compute_capability": ".".join(map(str, torch.cuda.get_device_capability(0)))})
    except Exception:
        pass
    return data

def download(project_id, file_id, target):
    url = f"/api/v1/projects/{project_id}/files/{file_id}/download"
    response = call("GET", url, stream=True)
    with open(target, "wb") as output:
        for chunk in response.iter_content(1024 * 1024):
            if chunk:
                output.write(chunk)

def upload_checkpoint(project_id, path):
    name = Path(path).name
    response = call(
        "POST",
        f"/api/v1/projects/{project_id}/files/{quote(name)}",
        data=Path(path).read_bytes(),
        headers={"Content-Type": "application/octet-stream", "X-Apolo-File-Kind": "checkpoint"},
    )
    return response.json()["id"]

def run_job(worker_id, job):
    project_id, job_id = job["project_id"], job["id"]
    try:
        listing = call("GET", f"/api/v1/projects/{project_id}/files").json()
        dataset = next((f for f in listing if f["kind"] == "dataset"), None)
        model_source = next((f for f in listing if f["filename"] == "model.py"), None)
        train_source = next((f for f in listing if f["filename"] == "train.py"), None)
        if not dataset or not model_source or not train_source:
            raise RuntimeError("Projeto precisa de dataset JSONL tokenizado e arquivos model.py/train.py.")
        with tempfile.TemporaryDirectory(prefix="apolo-ax-") as tmp:
            root = Path(tmp)
            download(project_id, dataset["id"], root / "dataset.jsonl")
            download(project_id, model_source["id"], root / "model.py")
            download(project_id, train_source["id"], root / "train.py")
            checkpoints = root / "checkpoints"
            command = [
                os.getenv("PYTHON", "python"), "train.py",
                "--data", "dataset.jsonl",
                "--output", "checkpoints",
                "--epochs", os.getenv("APOLO_TRAIN_EPOCHS", "1"),
                "--batch-size", os.getenv("APOLO_TRAIN_BATCH_SIZE", "2"),
            ]
            process = subprocess.Popen(command, cwd=root, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True, bufsize=1)
            last_loss = None
            for line in process.stdout:
                print(line.rstrip(), flush=True)
                try:
                    event = json.loads(line)
                    if "loss" in event:
                        last_loss = event["loss"]
                        call("POST", f"/api/v1/training/{job_id}/metrics", json={
                            "metrics": {"loss": last_loss, "step": event.get("step"), "epoch": event.get("epoch"), "device": event.get("device")}
                        })
                except (ValueError, requests.RequestException):
                    pass
            code = process.wait()
            if code != 0:
                raise RuntimeError(f"train.py terminou com código {code}")
            saved = sorted(checkpoints.glob("checkpoint-*.pt"))
            checkpoint_id = upload_checkpoint(project_id, saved[-1]) if saved else None
            call("POST", f"/api/v1/training/{job_id}/metrics", json={
                "metrics": {"loss": last_loss, "checkpoint_count": len(saved), "worker": NAME},
                "checkpoint_key": checkpoint_id,
            })
        call("POST", f"/api/v1/training/{job_id}/complete", json={"success": True})
        print(f"Job {job_id} concluído no worker {NAME}", flush=True)
    except Exception as error:
        try:
            call("POST", f"/api/v1/training/{job_id}/complete", json={"success": False, "error": str(error)[:2000]})
        except Exception:
            pass
        print(f"Job {job_id} falhou: {error}", flush=True)

def main():
    registered = call("POST", "/api/v1/workers", json=hardware()).json()
    worker_id = registered["id"]
    print(f"APOLO AX worker registrado: {worker_id}", flush=True)
    while True:
        try:
            call("POST", f"/api/v1/workers/{worker_id}/heartbeat", json=telemetry())
            job = call("POST", f"/api/v1/workers/{worker_id}/claim").json()
            if job:
                run_job(worker_id, job)
            else:
                time.sleep(POLL_SECONDS)
        except requests.RequestException as error:
            print(f"Falha de comunicação com APOLO AX: {error}", flush=True)
            time.sleep(POLL_SECONDS)

if __name__ == "__main__":
    main()
