# APOLO AX Worker

Este agente é um worker de treinamento real, executado na máquina que o administrador controla. Ele detecta CPU/RAM e GPU CUDA, registra inventário, envia heartbeat, reivindica jobs com bloqueio PostgreSQL e executa o train.py persistido no workspace.

## Instalação

1. Use uma máquina Linux com Python 3.11+ e armazenamento suficiente. Instale drivers CUDA e uma build de PyTorch compatível se for usar GPU.
2. Crie um token API no aplicativo APOLO Data Center.
3. Instale dependências: python -m pip install -r requirements.txt
4. Configure APOLO_AX_API com a URL HTTPS da API e APOLO_AX_TOKEN com o token.
5. Execute: python worker.py

## Limites importantes

- O agente executa o scaffold PyTorch fornecido pelo projeto e espera datasets JSONL tokenizados com campo input_ids.
- O agente não converte datasets automaticamente e não torna viável um modelo maior do que a memória física.
- Não use o worker para dados sem direitos de uso.
- Para produção, use um token dedicado, rede restrita, diretório de trabalho isolado, limites de CPU/RAM/disco e monitoramento.
- O worker não está implantado automaticamente; execute-o em hardware real que você controla.
