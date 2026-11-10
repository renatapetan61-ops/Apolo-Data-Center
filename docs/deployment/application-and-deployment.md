# Aplicação e implantação APOLO Data Center

## Componentes desta entrega

- `apps/ax-api`: API HTTP Rust e painel web servido pela mesma origem.
- `infrastructure/Dockerfile.api`: imagem multi-stage, executada como UID/GID não-root.
- `infrastructure/compose.yaml`: ambiente local com API, PostgreSQL, Redis e MinIO.
- `.dockerignore`: impede copiar arquivos de ambiente e artefatos locais para o contexto Docker.
- `.github/workflows/container-ci.yml`: valida a construção da imagem.
- `.github/workflows/container-publish.yml`: publica imagem no GHCR quando uma tag `v*` é enviada.

## API inicial

- `GET /`: painel em português.
- `GET /healthz`: health check.
- `GET /api/v1/overview`: estado resumido do serviço.
- `GET /api/v1/nodes`: inventários registrados no scheduler em memória.

## Implantação local

1. Copie `infrastructure/.env.example` para `infrastructure/.env`.
2. Substitua as senhas de exemplo por segredos fortes e exclusivos.
3. Na raiz do repositório execute:

   ```sh
   docker compose --env-file infrastructure/.env -f infrastructure/compose.yaml config
   docker compose --env-file infrastructure/.env -f infrastructure/compose.yaml up --build -d
   ```

4. Abra `http://127.0.0.1:8080`.
5. Confirme o health check com `curl http://127.0.0.1:8080/healthz`.

Os serviços de banco, cache e objetos ficam publicados apenas em loopback no host. O serviço API ainda não está conectado a PostgreSQL, Redis ou MinIO nesta versão; eles são provisionados, mas não são apresentados como persistência já integrada.

## Publicação da imagem

Depois que o build de container e os testes estiverem verdes, crie e envie uma tag versionada, por exemplo `v0.1.0`. O workflow `container-publish.yml` publica `ghcr.io/renatapetan61-ops/apolo-ax-api:v0.1.0`.

A publicação da imagem no GHCR não equivale a uma implantação pública em produção. Para isso ainda será necessário escolher um provedor/servidor, provisionar DNS e TLS, configurar secrets, firewall, backups, monitoramento e política de atualização/rollback.

## Limites de segurança e funcionalidade

Esta versão é um MVP local:
- Scheduler e inventários são voláteis e vivem no processo da API.
- Não há autenticação de usuários, RBAC, tenants, billing ou auditoria persistente.
- Não há endpoint público de registro de agentes.
- Não há persistência de reservas/jobs nem integração de dados com os serviços auxiliares.
- Não exponha a API diretamente à internet.
