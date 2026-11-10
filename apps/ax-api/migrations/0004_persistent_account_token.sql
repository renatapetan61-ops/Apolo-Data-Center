ALTER TABLE api_tokens ADD COLUMN IF NOT EXISTS token_ciphertext BYTEA;
CREATE UNIQUE INDEX IF NOT EXISTS api_tokens_one_active_per_user
    ON api_tokens(user_id);
