# APOLO VTA 2.0

APOLO VTA 2.0 is the virtual tensor and storage orchestration layer integrated with APOLO Data Center.

## Storage contract

- Logical capacity target: 999 TB (999,000,000,000,000 bytes, decimal TB).
- Physical capacity is discovered from the filesystem mounted at `APOLO_STORAGE_DIR` (default `/data/apolo`).
- Logical quota is not physical provisioning. Reads and writes remain bounded by the actual mounted volume and provider limits.
- The Android app displays logical capacity separately from physical total, used, and available bytes.
- Storage directories: `vta2/cache`, `vta2/ram-spill`, `vta2/nvme`, `vta2/archive`, `vta2/models`, `vta2/datasets`, and `vta2/checkpoints`.
- RAM spill is disk-backed spill space; it must never be reported as physical RAM.

## API

Authenticated endpoints:
- `GET /api/v1/vta2/storage`: reports the logical target, filesystem capacity, mount path, and directory readiness.
- `POST /api/v1/vta2/initialize`: creates the VTA 2.0 directory layout under the existing APOLO Data Center storage mount, then returns the same capacity report.

Set `APOLO_STORAGE_DIR` to the provider's persistent volume mount. On Railway, the currently configured mount is `/data/apolo`. Increasing the logical target does not increase Railway volume capacity; the volume must be provisioned separately in the hosting provider.

## VTA 2.0 architecture roadmap

1. Storage plane: persistent object/chunk storage, manifests, checksums, quotas, and resumable uploads.
2. Memory manager: bounded hot cache, RAM-aware admission, disk spill, and eviction policies.
3. Tensor plane: block/shard manifests, dtype metadata, and streaming reads.
4. Compute plane: CPU SIMD backend and optional accelerator backends, each with measured capability.
5. Scheduler: worker heartbeats, resource reservations, placement, retries, and backpressure.
6. Training runtime: distributed data/model parallelism, optimizer-state sharding, checkpoints, and recovery.
7. Benchmarks: measured bandwidth, IOPS, tensor throughput, worker utilization, and end-to-end tokens/second.

The current VTA 2.0 implementation establishes the authenticated storage-status API and Android storage screen. It does not yet implement distributed tensor execution, NVMe hardware creation, a 999 TB physical volume, or trillion-parameter training. Those require real infrastructure and subsequent runtime work.
