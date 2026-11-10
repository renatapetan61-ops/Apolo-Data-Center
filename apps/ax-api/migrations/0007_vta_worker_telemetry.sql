-- APOLO VTA (Virtual Tensor Accelerator) telemetry.
ALTER TABLE ax_workers
 ADD COLUMN IF NOT EXISTS ram_available_bytes BIGINT,
 ADD COLUMN IF NOT EXISTS gpu_free_memory_bytes BIGINT,
 ADD COLUMN IF NOT EXISTS cpu_utilization_pct DOUBLE PRECISION,
 ADD COLUMN IF NOT EXISTS gpu_utilization_pct DOUBLE PRECISION,
 ADD COLUMN IF NOT EXISTS accelerator_kind TEXT,
 ADD COLUMN IF NOT EXISTS compute_capability TEXT,
 ADD COLUMN IF NOT EXISTS network_bandwidth_mbps DOUBLE PRECISION;
