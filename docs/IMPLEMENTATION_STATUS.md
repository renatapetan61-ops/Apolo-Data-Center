# Registro de implementacao

Data de referencia: 2026-11-10

## Fase 1 — Fundacao e arquitetura

Status: fundacao inicial versionada; revisao e validacao de compilacao ainda pendentes.

Entregue:
- Workspace Rust e crates ax-core / ax-scheduler.
- Tipos iniciais de inventario de recursos e capacidade.
- Separacao explicita entre quota logica e capacidade fisica.
- Regra para impedir uso de inventario fisico nao verificado.
- Documentacao arquitetural e ADR inicial.
- Compose de desenvolvimento com PostgreSQL, Redis e MinIO.
- Pipeline GitHub Actions para fmt, clippy e testes.

Pendente:
- Executar CI e corrigir qualquer falha reportada.
- Instalar Rust/Cargo no ambiente de desenvolvimento local.
- Completar contratos de API, esquema de banco e política de migrações.
- Criar agentes de nó e descoberta de hardware (fase posterior).

## Fases seguintes

2. AX Core: completar invariantes, identificadores, estados, perfis de recursos e testes.
3. AX Storage: metadados, object storage, integridade, quotas e versionamento.
4. AX Data Ingestion: upload, validação, filas, deduplicação e backpressure.
5. AX Scheduler: seleção de nós e reservas transacionais.
6. AX Compute: workers isolados e adapters de aceleradores.
7. AX Token Engine: tokenização, ledger e quotas persistentes.
8. AX Training Engine: workers PyTorch/JAX e checkpoints.
9–11. APIs, backend, Android e integração.
12–15. Segurança, benchmarks, implantação e documentação operacional.

## Limites comprovados do ambiente

No ambiente de execução examinado, Rust/Cargo, Gradle, Docker e kubectl não estão instalados. O ambiente é Linux x86-64 sem GPU acessível. Portanto, testes de compilação Rust, build Android, execução Compose, benchmarks de GPU e testes de cluster não podem ser declarados aprovados neste ambiente.
