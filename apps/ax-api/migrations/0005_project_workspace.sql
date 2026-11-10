CREATE TABLE IF NOT EXISTS project_files (
 id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
 project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
 user_id UUID REFERENCES users(id) ON DELETE CASCADE,
 filename TEXT NOT NULL,
 content_type TEXT NOT NULL DEFAULT 'application/octet-stream',
 size_bytes BIGINT NOT NULL,
 sha256 CHAR(64) NOT NULL,
 storage_key TEXT NOT NULL,
 kind TEXT NOT NULL DEFAULT 'file',
 created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS project_files_project_idx ON project_files(project_id,created_at DESC);
CREATE TABLE IF NOT EXISTS project_models (
 id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
 project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
 user_id UUID REFERENCES users(id) ON DELETE CASCADE,
 name TEXT NOT NULL,
 architecture TEXT NOT NULL DEFAULT 'transformer',
 parameters BIGINT,
 config JSONB NOT NULL DEFAULT '{}'::jsonb,
 status TEXT NOT NULL DEFAULT 'draft',
 created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE TABLE IF NOT EXISTS ax_workers (
 id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
 user_id UUID REFERENCES users(id) ON DELETE CASCADE,
 name TEXT NOT NULL,
 endpoint TEXT,
 cpu_cores INTEGER NOT NULL DEFAULT 0,
 ram_bytes BIGINT NOT NULL DEFAULT 0,
 gpu_name TEXT,
 gpu_memory_bytes BIGINT NOT NULL DEFAULT 0,
 framework TEXT NOT NULL DEFAULT 'pytorch',
 status TEXT NOT NULL DEFAULT 'offline',
 last_seen_at TIMESTAMPTZ,
 created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS ax_workers_active_idx ON ax_workers(status,last_seen_at DESC);
CREATE TABLE IF NOT EXISTS training_jobs (
 id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
 project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
 model_id UUID REFERENCES project_models(id) ON DELETE SET NULL,
 user_id UUID REFERENCES users(id) ON DELETE CASCADE,
 status TEXT NOT NULL DEFAULT 'queued',
 config JSONB NOT NULL DEFAULT '{}'::jsonb,
 metrics JSONB NOT NULL DEFAULT '{}'::jsonb,
 checkpoint_key TEXT,
 worker_id UUID REFERENCES ax_workers(id) ON DELETE SET NULL,
 error TEXT,
 created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
 started_at TIMESTAMPTZ,
 finished_at TIMESTAMPTZ,
 updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS training_jobs_project_idx ON training_jobs(project_id,created_at DESC);
CREATE INDEX IF NOT EXISTS training_jobs_queue_idx ON training_jobs(status,created_at);
