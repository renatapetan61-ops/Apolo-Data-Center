# APOLO VTA — Virtual Tensor Accelerator

APOLO VTA é a camada virtual de inventário e planejamento de aceleradores do APOLO AX. Ela agrega telemetria dos workers registrados e estima a memória disponível para treinamento BF16 com AdamW.

## O que é e o que não é

- É um **orquestrador virtual**: inventário, agregação de recursos, estimativas e planejamento.
- Não emula uma TPU, não cria GPUs/VRAM/FLOPS por software e não garante ser mais rápido que uma TPU.
- Pode aproveitar vários servidores e melhorar utilização em cargas paralelizáveis. O ganho depende de GPU, interconexão, armazenamento e comunicação.
- Esta primeira versão é **planning-only**. O executor AX existente é single-rank e ainda não executa FSDP2/TP distribuído entre nós.

## Requisitos

Python 3.11+, requests, APOLO AX API acessível, token API e pelo menos um worker AX que envie heartbeats.

O agente `workers/ax-python/worker.py` publica RAM disponível, utilização de CPU, VRAM livre e compute capability CUDA quando disponíveis. A API expõe o inventário em `GET /api/v1/workers`.

## Uso

```bash
python -m pip install requests
export APOLO_AX_API="https://apolo-ax-api-production.up.railway.app"
export APOLO_AX_TOKEN="SEU_TOKEN_API"
python workers/virtual-accelerator/apolo_vta.py --parameters 70000000000
python workers/virtual-accelerator/apolo_vta.py --parameters googolplex
```

## Modelo de estimativa

A estimativa inicial usa 16 bytes por parâmetro para pesos BF16, gradientes e estados AdamW, com 35% de margem de segurança adicional (65% da VRAM livre considerada utilizável). É uma aproximação conservadora; ativações, sequência, batch, arquitetura, comunicação e fragmentação podem exigir mais memória.

## Roteiro para aceleração distribuída real

1. Agente com telemetria de VRAM livre, capacidade de GPU, rede e saúde.
2. Reserva atômica de nós e validação de topologia.
3. Runtime multi-node com torchrun e PyTorch FSDP2, tensor parallel e pipeline parallel quando aplicável.
4. Comunicação rápida (idealmente NVLink/RDMA/InfiniBand ou Ethernet de baixa latência), checkpoints distribuídos e recuperação.
5. Benchmarks reproduzíveis contra uma TPU/cluster de referência. Só então divulgar números de velocidade.

**Limite físico:** nenhum runtime torna treinável um modelo de 100 googolplex ou um centilhão de parâmetros. A VTA deve rejeitar esse pedido antes de reservar recursos ou iniciar jobs.

## Pedido de 100 novemdecillion de parâmetros

Na escala curta usada nos Estados Unidos, 1 novemdecillion = 10^60; portanto, 100 novemdecillion = 10^62 parâmetros. Na escala longa o nome tem outro valor, mas a conclusão física continua sendo inviabilidade absoluta. Somente os pesos em BF16 ocupariam aproximadamente 2 × 10^62 bytes. Uma estimativa simplificada de treinamento BF16 + gradientes + estados AdamW (16 bytes/parâmetro, sem contar ativações e buffers) chega a cerca de 1,6 × 10^63 bytes. Nenhuma VPU virtual pode materializar esse armazenamento ou criar a largura de banda e os FLOPS correspondentes.

A VTA reconhece explicitamente "100 novemdecillion" no planejador local e no endpoint /api/v1/vta/plan, recusando o plano antes de reservar workers ou criar jobs. Isso é um limite honesto de capacidade, não uma limitação arbitrária do produto.
