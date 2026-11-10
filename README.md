# APOLO DATA CENTER

Plataforma de computacao distribuida, armazenamento elastico e treinamento de IA. Nucleo: APOLO AX.

Estado: fundacao arquitetural em implementacao. Capacidade fisica somente sera anunciada apos descoberta por agentes de nos reais.

Principios: separar control plane/data plane; distinguir quota logica, capacidade fisica observada e capacidade reservada; nunca executar codigo de cliente no processo principal; confirmar reservas somente apos validacao atomica do inventario.

Stack planejada: Rust (AX), Python (workers PyTorch/JAX), Kotlin (Android), PostgreSQL, Redis e object storage S3-compatible.

Consulte docs/architecture/overview.md e docs/adr/0001-architecture-boundaries.md.